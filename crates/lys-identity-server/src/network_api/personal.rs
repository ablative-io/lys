//! Personal computer writes never grant control of another person's agents.

use std::str::FromStr;

use lys_identity::AgentId;
use lys_identity::projection::Projection;

use super::with_network;
use crate::error::ServerError;
use crate::error_team::TeamError;
use crate::network_store::Machine;
use crate::read_api::own_person;
use crate::routes::{AppState, with_directory};
use crate::teams_api::with_teams;
use crate::teams_state::Team;

pub(crate) fn refused(name: &str) -> ServerError {
    ServerError::Withheld {
        refusal: name.to_owned(),
    }
}

pub(crate) fn owns(machine: &Machine, person: &str) -> Result<(), ServerError> {
    if machine.named_by != person {
        return Err(refused("not_your_computer"));
    }
    Ok(())
}

pub(super) fn own_agents(
    directory: &Projection,
    person: lys_identity::PersonId,
    agents: &[String],
    roles: &[String],
) -> Result<(), ServerError> {
    if !roles.is_empty() {
        return Err(refused("agent_not_yours"));
    }
    let owned = directory
        .agents_of(person)
        .map(|entry| entry.map(|(id, _)| id.to_string()))
        .collect::<Result<std::collections::BTreeSet<_>, _>>()?;
    for agent in agents {
        let agent = AgentId::from_str(agent)?.to_string();
        if !owned.contains(&agent) {
            return Err(refused("agent_not_yours"));
        }
    }
    Ok(())
}

pub(super) fn own_team(
    state: &AppState,
    person: &str,
    chosen: Option<&str>,
) -> Result<Option<String>, ServerError> {
    if state.teams.is_none() && chosen.is_none() {
        return Ok(None);
    }
    with_teams(state, |store| {
        let belongs = |team: &&Team| {
            team.retired.is_none()
                && (team.created.owner == person
                    || (team.members.iter().any(|member| member == person)
                        && !team.held.iter().any(|held| held.member == person)))
        };
        if let Some(id) = chosen {
            let team = store
                .team(id)
                .ok_or(ServerError::Team(TeamError::Unknown))?;
            if !belongs(&team) {
                return Err(refused("not_your_computer"));
            }
            return Ok(Some(id.to_owned()));
        }
        let team = store
            .teams_iter()
            .filter(belongs)
            .find(|team| team.created.owner == person)
            .or_else(|| store.teams_iter().find(belongs));
        Ok(team.map(|team| team.created.id.clone()))
    })
}

pub(crate) fn computer_owner(
    state: &AppState,
    actor: &lys_identity::Actor,
    id: &str,
) -> Result<String, ServerError> {
    // An agent admitted at the door acts on a computer its person owns, and
    // the record names the agent (AGENTS-015 R1).
    if let Some(admitted) = crate::agent_acts::admitted() {
        let person = crate::agent_acts::person_of(state, admitted.agent)?.to_string();
        with_network(state, |store| {
            let machine = store.machine(id).ok_or(ServerError::MachineUnknown)?;
            owns(machine, &person)?;
            if machine.retired.is_some() {
                return Err(ServerError::MachineRetired);
            }
            Ok(())
        })?;
        return Ok(admitted.agent.to_string());
    }
    let (administrator, by) = with_directory(state, |directory| {
        let projection = directory.projection()?;
        let administrator = state.admission.is_administrator(projection, actor)?;
        let by = if administrator {
            match projection.person_for(actor.binding()) {
                Some(person) => person.to_string(),
                // A first-run administrator has a login before a person is registered.
                None => serde_json::to_string(&crate::read_api::login(actor.binding())).map_err(
                    |error| ServerError::DirectoryUnavailable {
                        reason: format!("the administrator's login could not be recorded: {error}"),
                    },
                )?,
            }
        } else {
            own_person(projection, actor)?.to_string()
        };
        Ok((administrator, by))
    })?;
    with_network(state, |store| {
        let machine = store.machine(id).ok_or(ServerError::MachineUnknown)?;
        if !administrator {
            owns(machine, &by)?;
        }
        if machine.retired.is_some() {
            return Err(ServerError::MachineRetired);
        }
        Ok(())
    })?;
    Ok(by)
}
