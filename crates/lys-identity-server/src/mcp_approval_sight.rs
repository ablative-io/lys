//! Approval alone admits a lead's current subtree without widening other reads.

use std::str::FromStr;

use axum::http::HeaderMap;
use lys_identity::{AgentId, IdentityId};

use crate::agent_sight::{SeenAgent, seen_agent};
use crate::agent_signature::signed_agent;
use crate::error::ServerError;
use crate::grants::caller;
use crate::routes::{AppState, with_directory};
use crate::teams_api::with_teams;
use crate::tree_views::reach;

/// Admit the existing agent sight or an agent lead's current subtree.
pub(crate) fn seen_target(
    state: &AppState,
    headers: &HeaderMap,
    id: &str,
    signed: (&str, &str, &[u8]),
) -> Result<(SeenAgent, IdentityId), ServerError> {
    let (approver, certified) = with_directory(state, |directory| {
        let projection = directory.projection()?;
        match signed_agent(state, projection, headers, signed)? {
            Some(agent) => Ok((IdentityId::Agent(agent), true)),
            None => Ok((caller(state, headers, projection)?, false)),
        }
    })?;
    if !certified {
        match seen_agent(state, headers, id) {
            Ok(agent) => return Ok((agent, approver)),
            Err(ServerError::AgentNotVisible) => {}
            Err(error) => return Err(error),
        }
    }
    let Ok(agent) = AgentId::from_str(id) else {
        return Err(ServerError::AgentNotVisible);
    };
    let seen = with_directory(state, |directory| {
        let projection = directory.projection()?;
        if !matches!(approver, IdentityId::Agent(_)) {
            return Err(ServerError::AgentNotVisible);
        }
        let record = projection
            .record(IdentityId::Agent(agent))
            .ok_or(ServerError::AgentNotVisible)?;
        Ok(SeenAgent {
            agent,
            responsible: record.responsible(),
        })
    })?;
    if approver == IdentityId::Agent(agent) {
        return Ok((seen, approver));
    }
    let reachable = with_teams(state, |store| {
        let teams = store.teams();
        let subtree = reach(teams, approver);
        Ok(teams.iter().any(|team| {
            subtree.contains(&team.created.id)
                && team.retired.is_none()
                && team.members.iter().any(|member| member == id)
                && !team.held.iter().any(|hold| hold.member == id)
        }))
    })?;
    if !reachable {
        return Err(ServerError::AgentNotVisible);
    }
    Ok((seen, approver))
}
