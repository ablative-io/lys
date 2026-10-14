//! Widget readings select a live session before reading its calls or taking a budget lock.

use std::collections::{BTreeMap, BTreeSet};

use crate::budgets_api::current_usage;
use crate::budgets_feed::Windows;
use crate::budgets_limits::Limit;
use crate::budgets_state::{Held, Measure};
use crate::budgets_usage::Used;
use crate::error::ServerError;
use crate::error_agents::AgentsError;
use crate::error_budget::BudgetError;
use crate::routes::AppState;
use crate::runtime_state::{Reported, Tracked};
use crate::runtime_store::SessionActivity;

/// A single runtime observation supplies both live ids and their actual start order.
pub struct WidgetSessions {
    /// Session activity used for period availability.
    pub activity: BTreeMap<String, SessionActivity>,
    /// Each live session's reported start, in seconds since the Unix epoch.
    pub started: BTreeMap<String, BTreeMap<String, u64>>,
}

fn unavailable(reason: String) -> ServerError {
    ServerError::Agents(AgentsError::Budget(BudgetError::BudgetsUnavailable {
        reason,
    }))
}

fn started_at(tracked: &Tracked) -> Result<u64, String> {
    tracked
        .reports
        .iter()
        .find(|report| report.state == Reported::Starting)
        .map(|report| report.at)
        .ok_or_else(|| {
            format!(
                "the start time of session {} was not reported",
                tracked.session
            )
        })
}

/// Read runtime evidence before the caller takes the budget lock.
///
/// # Errors
/// Names unavailable runtime state, inconsistent session attribution, or an unreported start.
pub fn widget_sessions(
    state: &AppState,
    agents: &BTreeSet<String>,
) -> Result<Option<WidgetSessions>, ServerError> {
    if state.runtime.is_none() {
        return Ok(None);
    }
    crate::runtime_api::with_runtime(state, |store| {
        let activity = store.agents_with_sessions(agents)?;
        let mut started = BTreeMap::new();
        for (agent, known) in &activity {
            let mut sessions = BTreeMap::new();
            for session in known.live_sessions() {
                let tracked = store.session(session).ok_or_else(|| {
                    unavailable(format!(
                        "live session {session} is missing from runtime reports"
                    ))
                })?;
                if tracked.agent.as_deref() != Some(agent.as_str()) {
                    return Err(unavailable(format!(
                        "live session {session} belongs to another agent"
                    )));
                }
                sessions.insert(session.clone(), started_at(tracked).map_err(unavailable)?);
            }
            started.insert(agent.clone(), sessions);
        }
        Ok(Some(WidgetSessions { activity, started }))
    })
}

/// Read one current session's context; period spend remains an agent aggregate.
///
/// # Errors
/// Names corrupt call records, unknown declarations, or period calculations that cannot be read.
pub fn current_widget_usage(
    held: &Held,
    limit: &Limit,
    agents: &BTreeSet<String>,
    zone: &str,
    at_ms: i64,
    sessions: Option<&WidgetSessions>,
    windows: &Windows,
) -> Result<Used, ServerError> {
    if limit.unit != Measure::ContextPercent {
        return current_usage(
            held,
            limit,
            agents,
            zone,
            at_ms,
            sessions.map(|known| &known.activity),
        );
    }
    let Some(sessions) = sessions else {
        return Ok(Used {
            unit: limit.unit,
            period: limit.period,
            figure: None,
            source: None,
            context: None,
            since_ms: None,
            unavailable: Some(
                "session liveness was not reported: the configuration names no runtime_dir"
                    .to_owned(),
            ),
            reported: None,
            account: None,
            tokens: None,
        });
    };
    held.session_context(limit, &sessions.started, windows, at_ms)
        .map_err(unavailable)
}

#[cfg(test)]
#[path = "current_context_tests.rs"]
mod tests;
