//! The administrator's roots for giving people and agents access, apart
//! from the loader's. First-run setup records them; an install set up
//! before them gains them only when its administrator asks, through
//! `POST /grants/agent-roots`, never at a start. They pass on every act the
//! model's `editor` carries that an agent may hold, so nothing withheld
//! from agents can ever be passed through them.
//!
//! Beside the directory roots are the two the one standing choice on an
//! agent's Access tab gives from (AGENTS-017 R4): one on the administrator's
//! own person passing `agent.create`, `agent.provisioning.set` and
//! `agent.provisioning.review` to agents, and one on the computer Lys runs
//! on, which is named first (`own_computer`), passing `machine.agent.set`.
//! A grant is given only on its source's resource, so a directory root alone
//! could not give an agent any of those four.

use lys_identity::grants::{
    Action, GrantError, PassOn, RecipientKind, Relation, Resource, RootRequest, Route, Window,
    agent_may_hold,
};
use lys_identity::{IdentityId, OperationId, PersonId};
use sha2::{Digest, Sha256};

use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::http::HeaderMap;
use serde::Serialize;

use crate::error::ServerError;
use crate::routes::{AppState, signed_in};
use crate::session::now;

/// The directory collections the roots are on.
const COLLECTIONS: [&str; 2] = ["agents", "apps"];

fn operation(owner: PersonId, label: &str) -> OperationId {
    let hash = Sha256::digest(format!("lys/agent-roots/v1/{owner}/{label}"));
    let mut id = [0; 16];
    id.copy_from_slice(&hash[..16]);
    OperationId::from_bytes(id)
}

/// Record `owner`'s roots under the held model, once: a repeat with the
/// same model answers the roots already recorded. A model with no `editor`
/// is refused `RelationUnknown`, and one whose `editor` carries nothing an
/// agent may hold is refused `WithheldFromAgents`, by name.
pub(crate) fn issue(state: &AppState, owner: PersonId) -> Result<Vec<String>, ServerError> {
    let computer = crate::own_computer::name(state, owner)?;
    issue_with(state, owner, computer.as_deref())
}

/// The acts the root on the administrator's own person passes to agents.
const PERSON_ACTS: [&str; 3] = [
    "agent.create",
    "agent.provisioning.review",
    "agent.provisioning.set",
];
/// The act the root on the computer Lys runs on passes to agents.
const MACHINE_ACTS: [&str; 1] = ["machine.agent.set"];

/// [`issue`], with the computer Lys runs on already named, or none.
fn issue_with(
    state: &AppState,
    owner: PersonId,
    computer: Option<&str>,
) -> Result<Vec<String>, ServerError> {
    crate::grants::with_grants(state, |judged| {
        let editor = Relation::new("editor")?;
        let model = judged.grants.model();
        let version = model.version();
        let carried = model.actions(&editor)?;
        let passable: std::collections::BTreeSet<Action> = carried
            .iter()
            .filter(|action| agent_may_hold("directory", action.as_str()))
            .cloned()
            .collect();
        if passable.is_empty() {
            return Err(GrantError::WithheldFromAgents {
                relation: editor.to_string(),
                withheld: carried
                    .iter()
                    .map(Action::as_str)
                    .collect::<Vec<_>>()
                    .join(", "),
            }
            .into());
        }
        let mut wanted = Vec::new();
        for collection in COLLECTIONS {
            wanted.push((
                Resource::new("directory", collection)?,
                passable.clone(),
                format!("{collection}/v{version}"),
            ));
        }
        let own = |kind: &str, acts: &[&str]| -> std::collections::BTreeSet<Action> {
            carried
                .iter()
                .filter(|action| {
                    acts.contains(&action.as_str()) && agent_may_hold(kind, action.as_str())
                })
                .cloned()
                .collect()
        };
        let person = own("person", &PERSON_ACTS);
        if !person.is_empty() {
            wanted.push((
                Resource::new("person", &owner.to_string())?,
                person,
                format!("person/v{version}"),
            ));
        }
        if let Some(machine) = computer {
            let acts = own("machine", &MACHINE_ACTS);
            if !acts.is_empty() {
                wanted.push((
                    Resource::new("machine", machine)?,
                    acts,
                    format!("machine/{machine}/v{version}"),
                ));
            }
        }
        let mut roots = Vec::new();
        for (resource, acts, label) in wanted {
            let root = judged
                .grants
                .issue_root(
                    judged.directory,
                    &RootRequest {
                        operation: operation(owner, &label),
                        caller: IdentityId::Person(owner),
                        route: Route::Api,
                        holder: owner,
                        resource,
                        relation: editor.clone(),
                        pass_on: PassOn::to(
                            acts,
                            [RecipientKind::Person, RecipientKind::Agent].into(),
                        )?,
                        window: Window::new(0, None)?,
                    },
                    now(),
                )?
                .event
                .grant();
            roots.push(root.to_string());
        }
        Ok(roots)
    })
}

/// The configured administrator's person, once first-run setup recorded one.
pub(crate) fn administrator(state: &AppState) -> Result<Option<PersonId>, ServerError> {
    let Some(login) = state.admission.administrator_login()? else {
        return Ok(None);
    };
    crate::routes::with_directory(state, |directory| {
        Ok(directory.projection()?.person_for(&login))
    })
}

/// First-run setup's roots for its administrator. A model that cannot give
/// agents access through `editor` records none at setup, deliberately: setup
/// finishes, and the explicit `POST /grants/agent-roots` names the refusal.
/// Answers `own_computer_unnamed` and why when the computer Lys runs on could
/// not be named now, for the setup answer to say.
pub(crate) fn at_setup(state: &AppState) -> Result<Option<String>, ServerError> {
    let Some(owner) = administrator(state)? else {
        return Ok(None);
    };
    // Setup finishes even when the computer cannot be named now: the setup
    // answer says so by name, and the administrator's next
    // `POST /grants/agent-roots` names it before it gives any root.
    let (computer, unnamed) = match crate::own_computer::name(state, owner) {
        Ok(computer) => (computer, None),
        Err(error) => {
            let words = format!(
                "own_computer_unnamed: the computer Lys runs on was not named at setup: {error}"
            );
            (state.say)(&words);
            (None, Some(words))
        }
    };
    match issue_with(state, owner, computer.as_deref()) {
        Ok(_)
        | Err(ServerError::Grant(
            GrantError::RelationUnknown { .. } | GrantError::WithheldFromAgents { .. },
        )) => Ok(unnamed),
        Err(other) => Err(other),
    }
}

/// The administrator's roots for giving people and agents access.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct AgentRoots {
    /// The roots' grant ids, recorded now or before.
    pub grants: Vec<String>,
}

/// `POST /grants/agent-roots`: the configured administrator records their
/// own roots for giving people and agents access, once. This is how an
/// install set up before them gains them; nothing else issues them.
pub(crate) async fn reissue(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<AgentRoots>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    // A root is a person's to give their own agent, never an agent's
    // (AGENTS-019 amendment, 22 October 2026): refused by name.
    if let Some(agent) = actor.provenance().agent() {
        return Err(GrantError::RootAuthorityRefused {
            caller: agent.to_string(),
        }
        .into());
    }
    state.admission.configured_administrator(&actor)?;
    let owner = administrator(&state)?.ok_or(ServerError::NoPerson)?;
    Ok(Json(AgentRoots {
        grants: issue(&state, owner)?,
    }))
}
