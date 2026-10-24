//! Who sees what is kept about an agent: the administrator, the person
//! responsible for the agent, the agent itself, and an agent admitted at the
//! door of the route (its grant of the act judged there, organisation
//! standing among them, 24 October 2026) for an agent of its own
//! organisation. To anyone else the agent is not visible, in the same words
//! as an agent that does not exist.

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
        let sees = state.admission.is_administrator(directory, &actor)?
            || match asker {
                IdentityId::Agent(own) => {
                    own == agent
                        || (crate::routes::admitted_agent(directory, &actor)?
                            && directory.organisation_of(asker).is_some()
                            && directory.organisation_of(asker)
                                == directory.organisation_of(IdentityId::Agent(agent)))
                }
                IdentityId::ServiceAccount(_)
                | IdentityId::Connector(_)
                | IdentityId::Machine(_)
                | IdentityId::Organisation(_) => false,
                IdentityId::Person(person) => responsible == Some(person),
            };
        if sees {
            Ok(SeenAgent { agent, responsible })
        } else {
            Err(ServerError::AgentNotVisible)
        }
    })
}
