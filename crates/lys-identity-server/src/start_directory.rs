//! The directory's own records as the start service reads them: the caller,
//! the administrator's admission, each agent's record and its lifecycle state.

use std::str::FromStr;
use std::sync::Arc;

use axum::http::HeaderMap;
use lys_identity::start::active::Lifecycles;
use lys_identity::start::authority::Admission;
use lys_identity::start::request::{AgentRecord, AgentRecords};
use lys_identity::{AgentId, IdentityId, LifecycleState, LoginBinding};

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
