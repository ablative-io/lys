//! Writes validate the caller before changing a computer.

use std::str::FromStr;
use std::sync::Arc;

use axum::Json;
use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use lys_identity::{AgentId, IdentityId, LifecycleState, OperationId};

use super::{
    AgentBody, MachineAgentsChanged, MachineTeamChanged, MachineView, NameBody, TeamBody,
    malformed, named, personal, view, with_network,
};
use crate::error::ServerError;
use crate::error_team::TeamError;
use crate::network_store::{AgentsRecorded, Retirement, TeamRecorded};
use crate::read_api::own_person;
use crate::routes::{AppState, signed_in, with_directory};
use crate::runtime_api::last_reports;
use crate::session::now;
use crate::teams_api::{team_id, with_teams};
use crate::teams_state::Team;

pub(super) fn name(
    State(state): State<Arc<AppState>>,
    headers: &HeaderMap,
    body: Result<Json<NameBody>, JsonRejection>,
) -> Result<Json<MachineView>, ServerError> {
    let actor = signed_in(&state, headers)?;
    let administrator = crate::routes::is_administrator(&state, &actor)?;
    let Json(mut body) = body.map_err(|refused| malformed(refused.body_text()))?;
    body.team = body.team.as_deref().map(team_id).transpose()?;
    with_directory(&state, |directory| {
        let directory = directory.projection()?;
        let person = own_person(directory, &actor)?;
        let by = person.to_string();
        if !administrator {
            personal::own_agents(directory, person, &body.may_run, &body.may_run_roles)?;
            body.team = personal::own_team(&state, &by, body.team.as_deref())?;
        }
        let known = crate::roles_api::role_ids(&state)?;
        let machine = named(directory, &body, (&known, by, now()))?;
        let team = owning_team(&state, body.team.as_deref())?;
        let last = last_reports(&state)?;
        with_network(&state, |store| {
            if !administrator && let Some(kept) = store.machine(&machine.id) {
                personal::owns(kept, &machine.named_by)?;
            }
            if store.machine(&machine.id).is_none() {
                active_team(team.as_ref())?;
            }
            store.name(machine.clone())?;
            let kept = store
                .machine(&machine.id)
                .ok_or(ServerError::MachineUnknown)?;
            Ok(Json(view(directory, kept, last.get(&kept.id).copied())?))
        })
    })
}

fn owning_team(state: &AppState, id: Option<&str>) -> Result<Option<Team>, ServerError> {
    id.map(|id| {
        with_teams(state, |store| {
            store
                .team(id)
                .cloned()
                .ok_or(ServerError::Team(TeamError::Unknown))
        })
    })
    .transpose()
}

fn active_team(team: Option<&Team>) -> Result<(), ServerError> {
    if let Some(team) = team.filter(|team| team.retired.is_some()) {
        return Err(ServerError::Team(TeamError::Retired {
            team: team.created.id.clone(),
        }));
    }
    Ok(())
}

pub(super) async fn assign_team(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    body: Result<Json<TeamBody>, JsonRejection>,
) -> Result<Json<MachineTeamChanged>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    let administrator = crate::routes::is_administrator(&state, &actor)?;
    let Json(body) = body.map_err(|refused| malformed(refused.body_text()))?;
    let id = OperationId::from_str(&id)
        .map_err(|error| malformed(format!("computer id does not read: {error}")))?
        .to_string();
    let operation = OperationId::from_str(&body.operation)
        .map_err(|error| malformed(format!("operation does not read: {error}")))?
        .to_string();
    let team = body.team.as_deref().map(team_id).transpose()?;
    with_directory(&state, |directory| {
        let directory = directory.projection()?;
        let by = own_person(directory, &actor)?.to_string();
        let owning = owning_team(&state, team.as_deref())?;
        let last = last_reports(&state)?;
        with_network(&state, |store| {
            let machine = store.machine(&id).ok_or(ServerError::MachineUnknown)?;
            let first = store.team_recorded(&operation);
            let repeating = first
                .is_some_and(|first| first.machine == id && first.team == team && first.by == by);
            let claiming = machine.team.is_none()
                && owning.as_ref().is_some_and(|team| team.created.owner == by);
            if !administrator && !repeating && !claiming {
                return Err(ServerError::NotAdmitted {
                    reason: "only the directory administrator changes computer ownership; a team's owner may claim an unowned computer for their own team",
                });
            }
            if first.is_none() {
                active_team(owning.as_ref())?;
            }
            let recorded = store.assign_team(TeamRecorded {
                operation,
                machine: id.clone(),
                team,
                by,
                at: now(),
            })?;
            let machine = store.machine(&id).ok_or(ServerError::MachineUnknown)?;
            Ok(Json(MachineTeamChanged {
                machine: view(directory, machine, last.get(&id).copied())?,
                recorded,
            }))
        })
    })
}

pub(super) async fn change_agent(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    body: Result<Json<AgentBody>, JsonRejection>,
) -> Result<Json<MachineAgentsChanged>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    let administrator = crate::routes::is_administrator(&state, &actor)?;
    let Json(body) = body.map_err(|refused| malformed(refused.body_text()))?;
    let id = OperationId::from_str(&id)
        .map_err(|error| malformed(format!("computer id does not read: {error}")))?
        .to_string();
    let operation = OperationId::from_str(&body.operation)
        .map_err(|error| malformed(format!("operation does not read: {error}")))?
        .to_string();
    let agent = AgentId::from_str(&body.agent)?;
    // The directory and the network are never locked together: the network write
    // syncs to disk, and no directory reader may wait on that flush.
    let by = with_directory(&state, |directory| {
        let directory = directory.projection()?;
        let person = own_person(directory, &actor)?;
        let by = person.to_string();
        if !administrator {
            personal::own_agents(directory, person, &[agent.to_string()], &[])?;
        }
        let record = directory
            .record(IdentityId::Agent(agent))
            .ok_or(ServerError::AgentNotVisible)?;
        if body.allow && record.state() != LifecycleState::Active {
            return Err(ServerError::Inactive {
                identity: agent.to_string(),
                state: record.state(),
            });
        }
        Ok(by)
    })?;
    let last = last_reports(&state)?;
    let (recorded, machine) = with_network(&state, |store| {
        if !administrator {
            personal::owns(store.machine(&id).ok_or(ServerError::MachineUnknown)?, &by)?;
        }
        let recorded = store.change_agent(AgentsRecorded {
            operation,
            machine: id.clone(),
            agent: agent.to_string(),
            allow: body.allow,
            by,
            at: now(),
            original_may_run: None,
        })?;
        let machine = store
            .machine(&id)
            .cloned()
            .ok_or(ServerError::MachineUnknown)?;
        Ok((recorded, machine))
    })?;
    with_directory(&state, |directory| {
        Ok(Json(MachineAgentsChanged {
            machine: view(directory.projection()?, &machine, last.get(&id).copied())?,
            recorded,
        }))
    })
}

pub(super) async fn retire(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<MachineView>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    crate::routes::administrator(&state, &actor)?;
    with_directory(&state, |directory| {
        let directory = directory.projection()?;
        let by = own_person(directory, &actor)?.to_string();
        let last = last_reports(&state)?;
        with_network(&state, |store| {
            store.retire(&id, Retirement { by, at: now() })?;
            let kept = store.machine(&id).ok_or(ServerError::MachineUnknown)?;
            Ok(Json(view(directory, kept, last.get(&kept.id).copied())?))
        })
    })
}
