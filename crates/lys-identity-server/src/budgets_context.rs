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

#[cfg(test)]
#[path = "budgets_context_tests.rs"]
mod tests;

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
