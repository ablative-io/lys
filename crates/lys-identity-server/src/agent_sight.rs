//! Who sees what is kept about an agent: the administrator, the person
//! responsible for the agent, and the agent itself. To anyone else the
//! agent is not visible, in the same words as an agent that does not exist.

use std::str::FromStr;

use axum::http::HeaderMap;
use lys_identity::{AgentId, IdentityId, PersonId};

use crate::error::ServerError;
use crate::grants::caller;
use crate::routes::{AppState, signed_in, with_directory};

/// An agent the caller sees.
pub(crate) struct SeenAgent {
    /// The agent.
    pub agent: AgentId,
    /// The person responsible for it.
    pub responsible: Option<PersonId>,
}

/// The agent `id` names, when the caller of `headers` sees it.
pub(crate) fn seen_agent(
    state: &AppState,
    headers: &HeaderMap,
    id: &str,
) -> Result<SeenAgent, ServerError> {
    let actor = signed_in(state, headers)?;
    let agent = AgentId::from_str(id).map_err(|_unread| ServerError::AgentNotVisible)?;
    with_directory(state, |directory| {
        let directory = directory.projection()?;
        let asker = caller(state, headers, directory)?;
        let record = directory
            .record(IdentityId::Agent(agent))
            .ok_or(ServerError::AgentNotVisible)?;
        let responsible = record.responsible();
        let sees = state.admission.administrator(&actor).is_ok()
            || match asker {
                IdentityId::Agent(own) => own == agent,
                IdentityId::ServiceAccount(_) => false,
                IdentityId::Person(person) => responsible == Some(person),
            };
        if sees {
            Ok(SeenAgent { agent, responsible })
        } else {
            Err(ServerError::AgentNotVisible)
        }
    })
}
