//! The directory's own records as the start service reads them: the caller,
//! the administrator's admission, each agent's record and its lifecycle state.

use std::str::FromStr;
use std::sync::Arc;

use axum::http::HeaderMap;
use lys_identity::grants::{Action, ExerciseRequest, Resource, Route};
use lys_identity::start::active::Lifecycles;
use lys_identity::start::authority::Admission;
use lys_identity::start::request::{AgentRecord, AgentRecords};
use lys_identity::{AgentId, IdentityId, LifecycleState, LoginBinding, PersonId};

use super::Callers;
use crate::error::ServerError;
use crate::routes::{AppState, signed_in, with_directory};

/// The directory's own records, read for a start: the caller, the
/// administrator's admission, each agent's record and its lifecycle state.
pub(super) struct Directory(pub(super) Arc<AppState>);

impl Directory {
    /// The person bound to `binding`, or the login itself when none is.
    fn caller_of(&self, binding: &LoginBinding) -> String {
        let login = || format!("login {} {}", binding.issuer(), binding.subject());
        match with_directory(&self.0, |directory| {
            Ok(directory.projection()?.person_for(binding))
        }) {
            Ok(Some(person)) => person.to_string(),
            Ok(None) => login(),
            Err(error) => {
                tracing::error!("the directory could not be read for the caller: {error}");
                login()
            }
        }
    }

    fn record(&self, agent: &str) -> Option<lys_identity::projection::Record> {
        let id = AgentId::from_str(agent).ok()?;
        match with_directory(&self.0, |directory| {
            Ok(directory.record(IdentityId::Agent(id))?)
        }) {
            Ok(record) => record,
            Err(error) => {
                tracing::error!(agent, "the directory could not be read: {error}");
                None
            }
        }
    }
}

impl Callers for Directory {
    fn check_admission(&self) -> Result<(), ServerError> {
        self.0.admission.administrator_available()
    }

    fn caller(&self, headers: &HeaderMap) -> Option<String> {
        // An agent admitted at the door is the caller its records name.
        if let Some(admitted) = crate::agent_acts::admitted() {
            return Some(admitted.agent.to_string());
        }
        let actor = signed_in(&self.0, headers).ok()?;
        Some(self.caller_of(actor.binding()))
    }
}

impl Admission for Directory {
    fn is_administrator(&self, caller: &str) -> bool {
        match self.0.admission.administrator_login() {
            Ok(login) => login.is_some_and(|login| caller == self.caller_of(&login)),
            Err(error) => {
                tracing::error!("start administrator admission refused: {error}");
                false
            }
        }
    }

    /// Step 1 admits the configured administrator alone.
    fn admits(&self, caller: &str) -> bool {
        self.is_administrator(caller)
    }

    /// Whether `caller`, a person or an agent, holds a live grant of
    /// `agent.start` on `agent` (AGENTS-015 R1); a grants read that fails is
    /// no grant, said in the log.
    fn holds_start(&self, caller: &str, agent: &str) -> bool {
        let Some(caller) = PersonId::from_str(caller)
            .map(IdentityId::Person)
            .ok()
            .or_else(|| AgentId::from_str(caller).ok().map(IdentityId::Agent))
        else {
            return false;
        };
        let asked = || -> Result<ExerciseRequest, ServerError> {
            Ok(ExerciseRequest {
                caller,
                route: Route::Api,
                resource: Resource::new("agent", agent)?,
                action: Action::new("agent.start")?,
            })
        };
        let held = crate::grants_reads::with_read_grants(
            &self.0,
            None,
            crate::session::now(),
            |_| Ok(vec![asked()?]),
            |judged| {
                Ok(crate::grants::allowed(judged.grants.explain(
                    judged.directory,
                    &asked()?,
                    crate::session::now(),
                    None,
                ))?)
            },
        );
        held.unwrap_or_else(|error| {
            tracing::error!(agent, "the start grants could not be read: {error}");
            false
        })
    }
}

impl AgentRecords for Directory {
    fn agent(&self, agent: &str) -> Option<AgentRecord> {
        let record = self.record(agent)?;
        Some(AgentRecord {
            id: agent.to_owned(),
            responsible: record.responsible().map(|person| person.to_string()),
        })
    }
}

impl Lifecycles for Directory {
    fn state(&self, agent: &str) -> Option<LifecycleState> {
        self.record(agent).map(|record| record.state())
    }
}
