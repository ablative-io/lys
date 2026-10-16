//! App person authority stays with the resources that person answers for.
use axum::http::HeaderMap;
use lys_identity::{AgentId, IdentityId, PersonId};

use crate::error::ServerError;
use crate::routes::{AppState, with_directory};

pub(crate) fn refused(resource: &str) -> ServerError {
    ServerError::NotPermitted {
        reason: format!(
            "the person-app pass may act only for its own person; this {resource} belongs to another person"
        ),
    }
}

pub(crate) fn agent(
    state: &AppState,
    headers: &HeaderMap,
    agent: &str,
) -> Result<Option<PersonId>, ServerError> {
    let Some(actor) = super::app_person_actor(state, headers)? else {
        return Ok(None);
    };
    let agent: AgentId = agent.parse()?;
    with_directory(state, |directory| {
        let projection = directory.projection()?;
        let person = crate::read_api::own_person(projection, &actor)?;
        let responsible = projection
            .record(IdentityId::Agent(agent))
            .ok_or(ServerError::AgentNotVisible)?
            .responsible();
        if responsible != Some(person) {
            return Err(refused("seat"));
        }
        Ok(Some(person))
    })
}
