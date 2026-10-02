//! Team coverage includes admitted descendants and never grants membership authority.

use std::collections::BTreeSet;

use lys_identity::projection::Projection;
use lys_identity::{IdentityId, LifecycleState};

use crate::budgets_state::{Holder, HolderKind, Standing};
use crate::error::ServerError;
use crate::error_budget::BudgetError;
use crate::routes::{AppState, with_directory};
use crate::teams_state::Team;

/// Build coverage from the complete directory, including members with no usage yet.
pub fn from_parts(directory: &Projection, teams: &[Team]) -> Result<Vec<Standing>, ServerError> {
    directory
        .records()
        .filter_map(|(identity, record)| {
            let IdentityId::Agent(agent) = identity else {
                return None;
            };
            if record.state() == LifecycleState::Retired {
                return None;
            }
            let agent = agent.to_string();
            Some(ancestors(&agent, teams).map(|teams| Standing {
                agent,
                teams,
                person: record.responsible().map(|person| person.to_string()),
            }))
        })
        .collect()
}

fn ancestors(agent: &str, teams: &[Team]) -> Result<BTreeSet<String>, ServerError> {
    let mut covered = BTreeSet::new();
    for team in teams.iter().filter(|team| {
        team.retired.is_none()
            && team.members.iter().any(|member| member == agent)
            && !team.held.iter().any(|hold| hold.member == agent)
    }) {
        let mut visiting = BTreeSet::new();
        let mut current = Some(team);
        while let Some(team) = current {
            if team.retired.is_some() {
                break;
            }
            if !visiting.insert(team.created.id.clone()) {
                return Err(ServerError::Budget(BudgetError::BudgetsUnavailable {
                    reason: "team budget coverage contains a parent cycle".to_owned(),
                }));
            }
            covered.insert(team.created.id.clone());
            current = team
                .parent
                .as_ref()
                .map(|parent| {
                    teams
                        .iter()
                        .find(|candidate| candidate.created.id == *parent)
                        .ok_or_else(|| {
                            ServerError::Budget(BudgetError::BudgetsUnavailable {
                                reason: format!("team budget parent {parent} is missing"),
                            })
                        })
                })
                .transpose()?;
        }
    }
    Ok(covered)
}

/// Settled directory and team coverage, with no clock or runner request.
pub fn standings(state: &AppState) -> Result<Vec<Standing>, ServerError> {
    let teams = if state.teams.is_some() {
        crate::teams_api::with_teams(state, |store| Ok(store.teams().to_vec()))?
    } else {
        Vec::new()
    };
    with_directory(state, |directory| {
        from_parts(directory.projection()?, &teams)
    })
}

/// Every admitted agent a holder covers, deduplicated across nested teams.
pub fn covered(holder: &Holder, standings: &[Standing]) -> BTreeSet<String> {
    standings
        .iter()
        .filter(|standing| match holder.kind {
            HolderKind::Agent => standing.agent == holder.id,
            HolderKind::Team => standing.teams.contains(&holder.id),
            HolderKind::Person => standing.person.as_ref() == Some(&holder.id),
        })
        .map(|standing| standing.agent.clone())
        .collect()
}
