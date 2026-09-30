//! The tree's typed summaries, computed from settled state on each read.

use std::collections::{BTreeMap, BTreeSet};

use lys_identity::IdentityId;
use lys_identity::projection::Projection;
use serde::Serialize;

use crate::budgets_state::{Budget, Held, Holder, Measure, Standing, covered};
use crate::error::ServerError;
use crate::error_team::TeamError;
use crate::provisioning_store::Profile;
use crate::roles_records::Role;
use crate::runtime_state::Tracked;
use crate::teams_state::Team;

/// The caller and only the teams within their reach.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct TreeView {
    /// The caller, never a person inserted among team members.
    pub root: Root,
    /// The highest teams in the caller's reach.
    pub teams: Vec<TreeTeam>,
}

/// The identity making the read.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[schema(as = TreeRoot)]
pub struct Root {
    /// Its identity id.
    pub id: String,
    /// Its directory display name.
    pub name: String,
}

/// One team and its descendant teams.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct TreeTeam {
    /// The team's operation id.
    pub id: String,
    /// Its recorded name.
    pub name: String,
    /// Its lead, when an admitted agent member is named.
    pub lead: Option<TreeAgent>,
    /// Its admitted agents, in membership order.
    pub members: Vec<TreeAgent>,
    /// Its immediate children within this read.
    #[schema(no_recursion)]
    pub teams: Vec<TreeTeam>,
}

/// An agent, with no credentials or instructions exposed.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct TreeAgent {
    /// Its identity id.
    pub id: String,
    /// Its directory display name.
    pub name: String,
    /// The names of its current held roles; empty when it holds none.
    pub role: String,
    /// The strongest held session report: live, unconfirmed or stopped.
    pub session: Session,
    /// Its open agent and team goals, in stored order.
    pub goals: Vec<String>,
    /// Every applicable effective budget, each with its measured spend.
    pub budgets: Vec<TreeBudget>,
    /// The latest reviewed version; null when none has been reviewed.
    pub profile: Option<TreeProfile>,
}

/// Session state from reports, with absence of confirmation explicit.
#[derive(Debug, Clone, Copy, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = TreeSession)]
pub enum Session {
    /// At least one session has a running report.
    Live,
    /// At least one session awaits its runtime's confirmation.
    Unconfirmed,
    /// No session is reported running or awaiting confirmation.
    Stopped,
}

/// One budget that applies to this agent.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct TreeBudget {
    /// The recorded holder, including personal budgets.
    pub holder: Holder,
    /// The recorded unit.
    pub measure: Measure,
    /// The effective limit in that unit.
    pub limit: u64,
    /// Spend in the current period, or current context; null without reports.
    pub spent: Option<u64>,
}

/// A reviewed profile's public summary.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct TreeProfile {
    /// The version's operation id.
    pub version: String,
    /// The identity that reviewed it.
    pub reviewed_by: String,
    /// The declared harness name, null when none was declared.
    pub harness: Option<String>,
    /// The declared program, null when no harness was declared.
    pub program: Option<String>,
    /// The profile's primary model, null when none was set.
    pub model: Option<String>,
    /// Only the names of the servers this version holds.
    pub mcp_servers: Vec<String>,
    /// Recorded confinement evidence; null while this state has none.
    pub writable: Option<Vec<String>>,
}

/// Settled state owned by one read, without holding locks during rendering.
pub(crate) struct TreeState {
    pub directory: Projection,
    pub teams: Vec<Team>,
    pub roles: Vec<Role>,
    pub profiles: Vec<Profile>,
    pub sessions: Vec<Tracked>,
    pub budgets: Held,
    pub goals: BTreeMap<String, Vec<String>>,
    pub at: u64,
    pub at_ms: i64,
}

impl TreeState {
    pub(crate) fn standings(&self) -> Vec<Standing> {
        self.directory
            .records()
            .filter_map(|(identity, record)| {
                let IdentityId::Agent(agent) = identity else {
                    return None;
                };
                let id = agent.to_string();
                Some(Standing {
                    agent: id.clone(),
                    person: record.responsible().map(|person| person.to_string()),
                    teams: self
                        .teams
                        .iter()
                        .filter(|team| {
                            team.retired.is_none()
                                && team.members.contains(&id)
                                && !team.held.iter().any(|hold| hold.member == id)
                        })
                        .map(|team| team.created.id.clone())
                        .collect(),
                })
            })
            .collect()
    }

    pub(crate) fn agent(
        &self,
        id: &str,
        standings: &[Standing],
    ) -> Result<Option<TreeAgent>, ServerError> {
        let Some(standing) = standings.iter().find(|standing| standing.agent == id) else {
            return Ok(None);
        };
        let Some(record) = self
            .directory
            .records()
            .find(|(identity, _record)| identity.to_string() == id)
            .map(|(_identity, record)| record)
        else {
            return Err(ServerError::Team(TeamError::Unavailable {
                reason: format!("agent `{id}` has a standing but no directory record"),
            }));
        };
        let roles = self
            .roles
            .iter()
            .filter(|role| {
                role.holding(id)
                    .is_some_and(|held| held.state(self.at) == "holding")
            })
            .map(|role| role.name.clone())
            .collect::<Vec<_>>()
            .join(", ");
        let sessions: Vec<_> = self
            .sessions
            .iter()
            .filter(|session| session.agent.as_deref() == Some(id))
            .collect();
        let session = if sessions.iter().any(|session| session.shown() == "running") {
            Session::Live
        } else if sessions
            .iter()
            .any(|session| session.shown() == "unconfirmed")
        {
            Session::Unconfirmed
        } else {
            Session::Stopped
        };
        let profile = self
            .profiles
            .iter()
            .find(|profile| profile.agent == id)
            .and_then(|profile| {
                profile.versions.iter().rev().find_map(|version| {
                    version.reviewed.as_ref().map(|review| TreeProfile {
                        version: version.operation.clone(),
                        reviewed_by: review.by.clone(),
                        harness: version
                            .settings
                            .harness
                            .as_ref()
                            .map(|harness| harness.name.clone()),
                        program: version
                            .settings
                            .harness
                            .as_ref()
                            .map(|harness| harness.program.clone()),
                        model: version.settings.model_access.first().cloned(),
                        mcp_servers: version
                            .settings
                            .mcp_servers
                            .iter()
                            .map(|server| server.name.clone())
                            .collect(),
                        writable: None,
                    })
                })
            });
        let budgets = self
            .budgets
            .budgets
            .iter()
            .map(|budget| self.budgets.effective(budget))
            .filter(|budget| covered(budget, [standing]).contains(id))
            .map(|budget| self.budget(budget, standings))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Some(TreeAgent {
            id: id.to_owned(),
            name: record.profile().display_name().to_owned(),
            role: roles,
            session,
            goals: self.goals.get(id).cloned().unwrap_or_default(),
            budgets,
            profile,
        }))
    }

    fn budget(&self, budget: &Budget, standings: &[Standing]) -> Result<TreeBudget, ServerError> {
        let agents = covered(budget, standings);
        let spent = if budget.measure == Measure::ContextPercent {
            // Context is a current session figure, never a cumulative charge.
            let mut latest = BTreeMap::new();
            for usage in &self.budgets.uses {
                if agents.contains(&usage.agent) && usage.at_ms <= self.at_ms {
                    if let Some(figure) = usage.context_percent {
                        latest.insert((usage.agent.clone(), usage.session.clone()), figure);
                    }
                }
            }
            latest.values().copied().max()
        } else {
            let start = budget
                .period
                .as_ref()
                .ok_or_else(|| ServerError::BudgetsUnavailable {
                    reason: "a cumulative budget has no recorded period".to_owned(),
                })?
                .start_of(self.at_ms)
                .map_err(|reason| ServerError::BudgetsUnavailable { reason })?;
            let reported = self.budgets.uses.iter().any(|usage| {
                agents.contains(&usage.agent) && usage.at_ms >= start && usage.at_ms <= self.at_ms
            });
            if reported {
                Some(
                    self.budgets
                        .spent(budget, &agents, self.at_ms)
                        .map_err(|reason| ServerError::BudgetsUnavailable { reason })?,
                )
            } else {
                None
            }
        };
        Ok(TreeBudget {
            holder: budget.holder.clone(),
            measure: budget.measure,
            limit: budget.limit,
            spent,
        })
    }
}

/// All descendants of directly owned or led teams, without inserting ancestors.
pub(crate) fn reach(teams: &[Team], caller: IdentityId) -> BTreeSet<String> {
    let id = caller.to_string();
    let mut reachable: BTreeSet<_> = teams
        .iter()
        .filter(|team| team.created.owner == id || team.lead.as_deref() == Some(id.as_str()))
        .map(|team| team.created.id.clone())
        .collect();
    loop {
        let before = reachable.len();
        for team in teams {
            if team
                .parent
                .as_ref()
                .is_some_and(|parent| reachable.contains(parent))
            {
                reachable.insert(team.created.id.clone());
            }
        }
        if reachable.len() == before {
            return reachable;
        }
    }
}
