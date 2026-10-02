//! Team coverage includes admitted descendants and never grants membership authority.

use std::collections::{BTreeMap, BTreeSet};

use lys_identity::projection::Projection;
use lys_identity::{IdentityId, LifecycleState};

use crate::budgets_state::{Holder, HolderKind, Standing};
use crate::error::ServerError;
use crate::error_budget::BudgetError;
use crate::routes::{AppState, with_directory};
use crate::teams_state::Team;

/// Build coverage from the complete directory, including members with no usage yet.
///
/// Team membership is reversed in one pass over the teams, so each agent reads
/// only the teams it belongs to and the parents above them.
pub fn from_parts(directory: &Projection, teams: &[Team]) -> Result<Vec<Standing>, ServerError> {
    let membership = Membership::of(teams);
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
            Some(membership.ancestors(&agent).map(|teams| Standing {
                agent,
                teams,
                person: record.responsible().map(|person| person.to_string()),
            }))
        })
        .collect()
}

/// Teams by id and by admitted member, read from the team list once.
struct Membership<'a> {
    by_id: BTreeMap<&'a str, &'a Team>,
    by_member: BTreeMap<&'a str, Vec<&'a Team>>,
}

impl<'a> Membership<'a> {
    fn of(teams: &'a [Team]) -> Self {
        let mut by_id = BTreeMap::new();
        let mut by_member: BTreeMap<&str, Vec<&Team>> = BTreeMap::new();
        for team in teams {
            #[cfg(test)]
            crate::budgets_work::visit(crate::budgets_work::Work::Team);
            // The first team kept under an id answers a parent lookup, as a scan would.
            by_id.entry(team.created.id.as_str()).or_insert(team);
            if team.retired.is_some() {
                continue;
            }
            let admitted: BTreeSet<&str> = team
                .members
                .iter()
                .map(String::as_str)
                .filter(|member| !team.held.iter().any(|hold| hold.member == *member))
                .collect();
            for member in admitted {
                by_member.entry(member).or_default().push(team);
            }
        }
        Self { by_id, by_member }
    }

    fn ancestors(&self, agent: &str) -> Result<BTreeSet<String>, ServerError> {
        let mut covered = BTreeSet::new();
        for team in self.by_member.get(agent).into_iter().flatten() {
            let mut visiting = BTreeSet::new();
            let mut current = Some(*team);
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
                        self.by_id.get(parent.as_str()).copied().ok_or_else(|| {
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
