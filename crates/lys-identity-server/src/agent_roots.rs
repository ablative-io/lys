//! The administrator's roots for giving people and agents access, apart
//! from the loader's. First-run setup records them; an install set up
//! before them gains them only when its administrator asks, through
//! `POST /grants/agent-roots`, never at a start. They pass on every act the
//! model's `editor` carries that an agent may hold, so nothing withheld
//! from agents can ever be passed through them.

use lys_identity::grants::{
    Action, GrantError, PassOn, RecipientKind, Relation, Resource, RootRequest, Route, Window, agent_may_hold,
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
        let mut roots = Vec::new();
        for collection in COLLECTIONS {
            let root = judged
                .grants
                .issue_root(
                    judged.directory,
                    &RootRequest {
                        operation: operation(owner, &format!("{collection}/v{version}")),
                        caller: IdentityId::Person(owner),
                        route: Route::Api,
                        holder: owner,
                        resource: Resource::new("directory", collection)?,
                        relation: editor.clone(),
                        pass_on: PassOn::to(
                            passable.clone(),
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
pub(crate) fn at_setup(state: &AppState) -> Result<(), ServerError> {
    let Some(owner) = administrator(state)? else {
        return Ok(());
    };
    match issue(state, owner) {
        Ok(_)
        | Err(ServerError::Grant(
            GrantError::RelationUnknown { .. } | GrantError::WithheldFromAgents { .. },
        )) => Ok(()),
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
    state.admission.configured_administrator(&actor)?;
    let owner = administrator(&state)?.ok_or(ServerError::NoPerson)?;
    Ok(Json(AgentRoots {
        grants: issue(&state, owner)?,
    }))
}
