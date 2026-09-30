//! The tree's typed summaries, computed from settled state on each read.

use std::collections::{BTreeMap, BTreeSet};

use lys_identity::IdentityId;
use lys_identity::projection::Projection;
use serde::Serialize;

use crate::budgets_limits::Limit;
use crate::budgets_state::{Act, Held, Holder, Length, Measure, Standing};
use crate::error::ServerError;
use crate::error_budget::BudgetError;
use crate::error_team::TeamError;
use crate::provisioning_store::{Profile, Version};
use crate::roles_records::Role;
use crate::runtime_state::Tracked;
use crate::teams_state::Team;

/// Read the most recent reviewed version, even when a newer version awaits review.
pub(crate) fn latest_reviewed(profile: &Profile) -> Option<&Version> {
    profile
        .versions
        .iter()
        .rev()
        .find(|version| version.reviewed.is_some())
}

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
    #[schema(value_type = f64)]
    pub limit: serde_json::Number,
    /// The limit's independent period.
    pub period: Option<Length>,
    /// What reaching this threshold asks.
    pub act: Act,
    /// Spend in the current period, or current context; null without reports.
    #[schema(value_type = Option<f64>)]
    pub spent: Option<serde_json::Number>,
    /// Why no current figure is available.
    pub unavailable: Option<String>,
    /// Start of the measured period or reported account window.
    pub since_ms: Option<i64>,
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
    pub zone: String,
    pub goals: BTreeMap<String, Vec<String>>,
    pub at: u64,
    pub at_ms: i64,
}

impl TreeState {
    pub(crate) fn standings(&self) -> Result<Vec<Standing>, ServerError> {
        crate::budgets_members::from_parts(&self.directory, &self.teams)
    }

    pub(crate) fn agent(
        &self,
        id: &str,
        standings: &[Standing],
    ) -> Result<Option<TreeAgent>, ServerError> {
        if !standings.iter().any(|standing| standing.agent == id) {
            return Ok(None);
        }
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
            .and_then(latest_reviewed)
            .and_then(|version| {
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
            });
        let mut budgets = Vec::new();
        for collection in &self.budgets.limit_sets {
            if !crate::budgets_members::covered(&collection.holder, standings).contains(id) {
                continue;
            }
            for limit in self.budgets.effective_limits(collection) {
                budgets.push(self.budget(&collection.holder, &limit, standings)?);
            }
        }
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

    fn budget(
        &self,
        holder: &Holder,
        limit: &Limit,
        standings: &[Standing],
    ) -> Result<TreeBudget, ServerError> {
        let agents = crate::budgets_members::covered(holder, standings);
        let mut used = crate::budgets_usage::figure(
            &self.budgets,
            limit,
            &agents,
            &self.zone,
            self.at_ms,
            None,
        )
        .map_err(|reason| ServerError::Budget(BudgetError::BudgetsUnavailable { reason }))?;
        if used.figure.is_some()
            && !self.budgets.uses.iter().any(|usage| {
                agents.contains(&usage.agent)
                    && usage.at_ms <= self.at_ms
                    && used.since_ms.is_none_or(|since| usage.at_ms >= since)
            })
        {
            used.figure = None;
            used.unavailable = Some(format!(
                "{} has not been reported in this period",
                limit.unit.name()
            ));
        }
        Ok(TreeBudget {
            holder: holder.clone(),
            measure: limit.unit,
            limit: limit.amount.clone(),
            period: limit.period,
            act: limit.act,
            spent: used.figure,
            unavailable: used.unavailable,
            since_ms: used.since_ms,
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
