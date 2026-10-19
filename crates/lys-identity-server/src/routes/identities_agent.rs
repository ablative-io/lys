//! An agent registers an agent for its person (AGENTS-015 R1, R2).
//!
//! The calling agent's grant of `agent.create` on the person it answers to
//! is exercised; the new agent answers to that person, never to the calling
//! agent, and is registered active, with no separate transition. Its id is
//! the same for the same caller and operation, so a registration asked again
//! is answered, never registered twice.
//!
//! The agent exists before its rights do (AGENTS-016 R3): the agent and its
//! activation are recorded first, in one append to the directory; only then
//! are the creator's five rights on it and the use of the `agent.create`
//! grant that admitted it recorded, in one append to the grant log. No file
//! log pair writes both at once, so the order is the guarantee: a crash
//! between the two leaves an agent with no rights, on the person's
//! dashboard, which the same registration asked again finishes by writing
//! only the rights not already standing; never rights on an agent nobody
//! holds.

use std::str::FromStr;

use lys_identity::grants::{Action, CreatedRequest, Resource, Route};
use lys_identity::{
    AgentId, Change, IdentityEvent, IdentityId, LifecycleState, OperationId, PersonId, Profile,
    Transition,
};
use sha2::{Digest, Sha256};

use super::identities::AgentRegistration;
use super::{AppState, identity_id};
use crate::directory_views::{AgentRegistered, receipt_view};
use crate::error::ServerError;
use crate::session::now;

/// The agent `creator` registers under `operation`: the same id every time
/// it is asked.
fn new_agent(creator: AgentId, operation: OperationId) -> AgentId {
    let hash = Sha256::digest(format!("lys/agent-registered/v1/{creator}/{operation}"));
    let mut id = [0; lys_identity::id::ID_LEN];
    id.copy_from_slice(&hash[..lys_identity::id::ID_LEN]);
    AgentId::from_bytes(id)
}

/// Register the agent `body` names for the person `creator` answers to.
///
/// # Errors
/// `not_permitted` without a live grant of `agent.create` on that person;
/// `NotAdmitted` for any other person or agent to answer to; the grants' and
/// the directory's own refusals when either cannot record it.
pub(super) fn register(
    state: &AppState,
    creator: AgentId,
    body: &AgentRegistration,
) -> Result<AgentRegistered, ServerError> {
    let operation = OperationId::from_str(&body.operation)?;
    let profile = Profile::new(&body.display_name)?;
    let person = crate::agent_acts::person_of(state, creator)?;
    if let Some(named) = body.answers_to.as_deref()
        && identity_id(named)? != IdentityId::Person(person)
    {
        return Err(ServerError::NotAdmitted {
            reason: "an agent registers an agent only to answer to the person it answers to",
        });
    }
    let admitting = crate::agent_acts::admitting(
        state,
        creator,
        &Resource::new("person", &person.to_string())?,
        &Action::new(lys_identity::grants::lineage::CREATE)?,
    )?;
    let agent = new_agent(creator, operation);
    let at = now();
    let answer = crate::routes::with_directory(state, |directory| {
        let actor = crate::agent_acts::actor_of(directory.projection()?, creator)?;
        registered(directory, (actor, operation), agent, person, profile, at)
    })?;
    crate::agent_policy_api::with_policies(state, |policies| {
        policies.ensure_default(&agent.to_string())
    })?;
    crate::grants::with_directory_grants(
        state,
        |judged| {
            let request = CreatedRequest {
                operation,
                source: admitting.grant,
                agent,
                route: Route::Api,
            };
            judged
                .grants
                .record_created(judged.directory, &request, at)?;
            Ok(())
        },
        |_, ()| Ok(answer),
    )
}

/// The agent registered and active, answering to `person`, in one append;
/// answered from its record when this registration was recorded before.
fn registered(
    directory: &mut lys_identity::Directory<lys_log_store::FileLeafStore>,
    (actor, operation): (lys_identity::Actor, OperationId),
    agent: AgentId,
    person: PersonId,
    profile: Profile,
    at: u64,
) -> Result<AgentRegistered, ServerError> {
    let first = match directory.record(IdentityId::Agent(agent))? {
        Some(record) if record.responsible() == Some(person) => record.events().first().copied(),
        Some(_) => {
            return Err(ServerError::NotAdmitted {
                reason: "this registration names an agent that answers to someone else",
            });
        }
        None => None,
    };
    let receipt = if let Some(index) = first {
        directory
            .receipt_at(index)?
            .ok_or(ServerError::DirectoryUnavailable {
                reason: "the agent's registration has no receipt".to_owned(),
            })?
    } else {
        let activation = activation(operation);
        let events = [
            IdentityEvent::new(
                operation,
                actor.clone(),
                IdentityId::Agent(agent),
                at,
                Change::RegisterAgent {
                    responsible: person,
                    profile,
                },
            )?,
            IdentityEvent::new(
                activation,
                actor,
                IdentityId::Agent(agent),
                at,
                Change::Transition {
                    transition: Transition::Activate,
                    from: LifecycleState::Registered,
                    to: LifecycleState::Active,
                    reason: String::new(),
                },
            )?,
        ];
        directory.commit_batch(&events)?.into_iter().next().ok_or(
            ServerError::DirectoryUnavailable {
                reason: "the registration was answered with no receipt".to_owned(),
            },
        )?
    };
    Ok(AgentRegistered {
        agent: agent.to_string(),
        responsible: person.to_string(),
        reports_to: crate::reporting_views::edge(IdentityId::Person(person))?,
        receipt: receipt_view(&receipt),
    })
}

/// The activation's operation, derived from the registration's.
fn activation(operation: OperationId) -> OperationId {
    let hash = Sha256::digest(format!("lys/agent-registered/v1/{operation}/activate"));
    let mut id = [0; 16];
    id.copy_from_slice(&hash[..16]);
    OperationId::from_bytes(id)
}
