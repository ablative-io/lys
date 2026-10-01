//! Context availability folds once with usage, keeping reads independent of event history.

use std::collections::{BTreeMap, BTreeSet};

use crate::budgets_limits::Limit;
use crate::budgets_state::Usage;
use crate::budgets_usage::Used;

pub(crate) const NO_SESSION: &str =
    "the usage report names no session, so its context cannot be measured";
pub(crate) const NO_CONTEXT: &str = "context_percent was not reported";
pub(crate) const STOP_UNMEASURED: &str =
    "context_percent was not reported, so the context Stop cannot be measured";
const FUTURE: &str = "the context report is ahead of the observation time";

/// The missing input of one report, independent of any chosen action.
pub(crate) fn missing(usage: &Usage) -> Option<&'static str> {
    if usage.session.is_none() {
        Some(NO_SESSION)
    } else if usage.context_percent.is_none() {
        Some(NO_CONTEXT)
    } else {
        None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Reading {
    at_ms: i64,
    figure: Option<u64>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct Agent {
    sessions: BTreeMap<String, Reading>,
    figures: BTreeMap<u64, usize>,
    missing: BTreeSet<String>,
    unnamed_at: Option<i64>,
    named_at: Option<i64>,
}

impl Agent {
    fn keep(&mut self, usage: &Usage) -> Result<(), String> {
        let Some(session) = &usage.session else {
            self.unnamed_at = Some(
                self.unnamed_at
                    .map_or(usage.at_ms, |at| at.max(usage.at_ms)),
            );
            return Ok(());
        };
        let previous = self.sessions.get(session).copied();
        if previous.is_some_and(|reading| reading.at_ms > usage.at_ms) {
            return Ok(());
        }
        if let Some(figure) = previous.and_then(|reading| reading.figure) {
            let count = self
                .figures
                .get_mut(&figure)
                .ok_or("context figure count is absent")?;
            *count = count
                .checked_sub(1)
                .ok_or("context figure count underflows")?;
            if *count == 0 {
                self.figures.remove(&figure);
            }
        }
        if let Some(figure) = usage.context_percent {
            let count = self.figures.entry(figure).or_default();
            *count = count
                .checked_add(1)
                .ok_or("context figure count overflows")?;
            self.missing.remove(session);
        } else {
            self.missing.insert(session.clone());
        }
        self.sessions.insert(
            session.clone(),
            Reading {
                at_ms: usage.at_ms,
                figure: usage.context_percent,
            },
        );
        self.named_at = Some(self.named_at.map_or(usage.at_ms, |at| at.max(usage.at_ms)));
        Ok(())
    }

    fn remove(&mut self, session: &str) -> Result<(), String> {
        if let Some(reading) = self.sessions.remove(session) {
            if let Some(figure) = reading.figure {
                let count = self
                    .figures
                    .get_mut(&figure)
                    .ok_or("context figure count is absent")?;
                *count = count
                    .checked_sub(1)
                    .ok_or("context figure count underflows")?;
                if *count == 0 {
                    self.figures.remove(&figure);
                }
            }
            self.missing.remove(session);
            self.named_at = self.sessions.values().map(|reading| reading.at_ms).max();
        }
        Ok(())
    }

    fn gap(&self, at_ms: i64) -> Option<&'static str> {
        if self.named_at.is_some_and(|at| at > at_ms)
            || self.unnamed_at.is_some_and(|at| at > at_ms)
        {
            return Some(FUTURE);
        }
        if self
            .unnamed_at
            .is_some_and(|unnamed| self.named_at.is_none_or(|named| unnamed >= named))
        {
            Some(NO_SESSION)
        } else if !self.missing.is_empty() {
            Some(NO_CONTEXT)
        } else {
            None
        }
    }
}

/// At most one current reading per tracked session, rather than one per event.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Availability {
    agents: BTreeMap<String, Agent>,
}

impl Availability {
    /// Update the current reading when a usage leaf is folded.
    pub(crate) fn keep(&mut self, usage: &Usage) -> Result<(), String> {
        self.agents
            .entry(usage.agent.clone())
            .or_default()
            .keep(usage)
    }

    pub(crate) fn agents(&self) -> BTreeSet<String> {
        self.agents.keys().cloned().collect()
    }

    pub(crate) fn remove(&mut self, agent: &str, session: &str) -> Result<(), String> {
        if let Some(agent) = self.agents.get_mut(agent) {
            agent.remove(session)?;
        }
        Ok(())
    }

    pub(crate) fn reconcile(
        &mut self,
        sessions: &BTreeMap<String, crate::runtime_store::SessionActivity>,
    ) -> Result<Vec<String>, String> {
        let mut removed = Vec::new();
        for (name, activity) in sessions {
            let Some(agent) = self.agents.get_mut(name) else {
                continue;
            };
            let closed: Vec<_> = agent
                .sessions
                .keys()
                .filter(|session| !activity.live_sessions().contains(*session))
                .cloned()
                .collect();
            for session in closed {
                agent.remove(&session)?;
                removed.push(session);
            }
        }
        Ok(removed)
    }

    pub(crate) fn used_live(
        &self,
        limit: &Limit,
        agents: &BTreeSet<String>,
        at_ms: i64,
        sessions: Option<&BTreeMap<String, crate::runtime_store::SessionActivity>>,
    ) -> Used {
        let Some(sessions) = sessions else {
            return self.used(limit, agents, at_ms);
        };
        let mut figure = None;
        let mut gap = None;
        for name in agents {
            // An agent with no live session, and a live session that has not
            // reported yet, add no figure and no gap, as the fold over reports
            // does; only a report that names no context is a gap.
            let Some(activity) = sessions.get(name) else {
                continue;
            };
            let agent = self.agents.get(name);
            let mut named_at = None;
            for session in activity.live_sessions() {
                let Some(reading) = agent.and_then(|agent| agent.sessions.get(session)) else {
                    continue;
                };
                named_at = Some(named_at.map_or(reading.at_ms, |at: i64| at.max(reading.at_ms)));
                if reading.at_ms > at_ms {
                    gap = Some(FUTURE);
                    break;
                }
                let Some(current) = reading.figure else {
                    gap = Some(NO_CONTEXT);
                    break;
                };
                figure = Some(figure.map_or(current, |held: u64| held.max(current)));
            }
            if gap.is_some() {
                break;
            }
            if let Some(unnamed) = agent.and_then(|agent| agent.unnamed_at) {
                if unnamed > at_ms {
                    gap = Some(FUTURE);
                    break;
                }
                if named_at.is_none_or(|named| unnamed >= named) {
                    gap = Some(NO_SESSION);
                    break;
                }
            }
        }
        Used {
            unit: limit.unit,
            period: limit.period,
            figure: gap.is_none().then_some(figure).flatten().map(Into::into),
            since_ms: None,
            unavailable: gap
                .or_else(|| figure.is_none().then_some(NO_CONTEXT))
                .map(str::to_owned),
            account: None,
        }
    }

    /// Read covered agents' current maximum, refusing to invent a figure for a gap.
    pub(crate) fn used(&self, limit: &Limit, agents: &BTreeSet<String>, at_ms: i64) -> Used {
        let mut figure = None;
        for agent in agents.iter().filter_map(|agent| self.agents.get(agent)) {
            if let Some(reason) = agent.gap(at_ms) {
                return Used {
                    unit: limit.unit,
                    period: limit.period,
                    figure: None,
                    since_ms: None,
                    unavailable: Some(reason.to_owned()),
                    account: None,
                };
            }
            if let Some((&highest, _)) = agent.figures.last_key_value() {
                figure = Some(figure.map_or(highest, |held: u64| held.max(highest)));
            }
        }
        Used {
            unit: limit.unit,
            period: limit.period,
            figure: figure.map(Into::into),
            since_ms: None,
            unavailable: figure.is_none().then(|| NO_CONTEXT.to_owned()),
            account: None,
        }
    }
}

#[cfg(test)]
#[path = "budgets_context_tests.rs"]
mod tests;

/// A confirmed runtime end removes only that session's derived context state.
pub(crate) fn finish(
    state: &crate::routes::AppState,
    agent: &str,
    session: &str,
) -> Result<(), crate::error::ServerError> {
    if state.budgets.is_none() {
        return Ok(());
    }
    crate::budgets_api::with_budgets(state, |store| store.end_context(agent, session))
}
