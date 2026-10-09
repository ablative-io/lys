//! What a person who is not an administrator may see of the network: the
//! computers they named, their teams' computers, and the in-use computers that
//! may run one of their active agents or one of the roles those agents hold.

use std::collections::BTreeSet;

use lys_identity::projection::Projection;
use lys_identity::{Actor, LifecycleState};

use crate::error::ServerError;
use crate::network_store::Machine;
use crate::read_api::own_person;
use crate::routes::AppState;
use crate::session::now;
use crate::teams_api::with_teams;

/// One person's reach over the network, read once per answer.
pub(crate) struct PersonalNetwork {
    person: String,
    teams: BTreeSet<String>,
    agents: BTreeSet<String>,
    roles: BTreeSet<String>,
}

impl PersonalNetwork {
    pub(crate) fn read(
        state: &AppState,
        directory: &Projection,
        actor: &Actor,
    ) -> Result<Option<Self>, ServerError> {
        if state.admission.is_administrator(directory, actor)? {
            return Ok(None);
        }
        let person = own_person(directory, actor)?;
        let agents = directory
            .agents_of(person)
            .filter_map(|entry| match entry {
                Ok((id, record)) if record.state() == LifecycleState::Active => {
                    Some(Ok(id.to_string()))
                }
                Ok(_) => None,
                Err(error) => Some(Err(ServerError::from(error))),
            })
            .collect::<Result<BTreeSet<_>, _>>()?;
        let person = person.to_string();
        let teams = if state.teams.is_some() {
            with_teams(state, |store| {
                Ok(store
                    .teams_iter()
                    .filter(|team| {
                        team.created.owner == person
                            || (team.members.contains(&person)
                                && !team.held.iter().any(|held| held.member == person))
                    })
                    .map(|team| team.created.id.clone())
                    .collect())
            })?
        } else {
            BTreeSet::new()
        };
        let roles = if let Some(store) = &state.roles {
            let mut store = store
                .lock()
                .map_err(|error| ServerError::RolesUnavailable {
                    reason: format!("the roles lock is poisoned: {error}"),
                })?;
            store.settle()?;
            let at = now();
            store
                .roles()
                .iter()
                .filter(|role| {
                    agents.iter().any(|agent| {
                        role.holding(agent)
                            .is_some_and(|holding| holding.state(at) == "holding")
                    })
                })
                .map(|role| role.id.clone())
                .collect()
        } else {
            BTreeSet::new()
        };
        Ok(Some(Self {
            person,
            teams,
            agents,
            roles,
        }))
    }

    pub(crate) fn permits(&self, machine: &Machine) -> bool {
        machine.named_by == self.person
            || machine
                .team
                .as_ref()
                .is_some_and(|team| self.teams.contains(team))
            || (machine.retired.is_none()
                && machine.runtime.is_some()
                && (machine
                    .may_run
                    .iter()
                    .any(|agent| self.agents.contains(agent))
                    || machine
                        .may_run_roles
                        .iter()
                        .any(|role| self.roles.contains(role))))
    }
}
