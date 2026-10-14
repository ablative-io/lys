//! Context availability folds once with usage, keeping reads independent of event history.

use std::collections::{BTreeMap, BTreeSet};

use crate::budgets_limits::Limit;
use crate::budgets_state::Usage;
use crate::budgets_usage::Used;

#[path = "budgets_state/session_context.rs"]
mod session_context;
pub use session_context::SessionReading;

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

/// The retained context reading's provenance, derived from existing usage.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, utoipa::ToSchema)]
pub struct ContextReading {
    /// The session this reading belongs to.
    pub session: Option<String>,
    /// When the figure was read, absent until this session reports one.
    pub read_at_ms: Option<i64>,
    /// Later model calls in this session that reported no context figure.
    pub reported_none: usize,
    /// The model that supplied the reading, or the unmeasured session's model.
    pub model: Option<String>,
    /// The call's reported input, cache writes and cache reads, excluding output.
    pub context_tokens: Option<u64>,
    /// The declared window used for this reading; absence is never guessed.
    pub window_tokens: Option<u64>,
    /// Each live session's own reading, available for an explicit canvas choice.
    pub live_sessions: Vec<SessionReading>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Reading {
    session: String,
    at_ms: i64,
    latest_at_ms: i64,
    figure: Option<u64>,
    source: Option<String>,
    model: Option<String>,
    unavailable: Option<String>,
    empty_calls: BTreeMap<i64, usize>,
    reported_none: usize,
}

impl Reading {
    fn context(&self) -> ContextReading {
        ContextReading {
            session: Some(self.session.clone()),
            read_at_ms: self.figure.map(|_| self.at_ms),
            reported_none: self.reported_none,
            model: self.model.clone(),
            context_tokens: None,
            window_tokens: None,
            live_sessions: Vec::new(),
        }
    }

    fn empty_call(&mut self, usage: &Usage) -> Result<(), String> {
        if usage.call.is_some() && (self.figure.is_none() || usage.at_ms >= self.at_ms) {
            let count = self.empty_calls.entry(usage.at_ms).or_default();
            *count = count
                .checked_add(1)
                .ok_or("empty context call count overflows")?;
            self.reported_none = self
                .reported_none
                .checked_add(1)
                .ok_or("empty context call total overflows")?;
        }
        self.latest_at_ms = self.latest_at_ms.max(usage.at_ms);
        Ok(())
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct Agent {
    sessions: BTreeMap<String, Reading>,
    figures: BTreeMap<u64, usize>,
    sources: BTreeMap<u64, BTreeMap<Option<String>, usize>>,
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
        let previous = self.sessions.get(session);
        if previous.as_ref().is_some_and(|reading| {
            reading.figure.is_some()
                && reading.at_ms > usage.at_ms
                && usage.context_percent.is_some()
        }) {
            return Ok(());
        }
        // Missing context describes this call, not a replacement reading.
        if usage.context_percent.is_none() && previous.is_some() {
            let reading = self
                .sessions
                .get_mut(session)
                .ok_or("context session is absent")?;
            if reading.figure.is_none() && usage.at_ms >= reading.latest_at_ms {
                reading.at_ms = usage.at_ms;
                if let Some(model) = usage.call.as_ref().and_then(|call| call.model.as_ref()) {
                    reading.model = Some(model.clone());
                }
                reading.unavailable = usage
                    .unavailable
                    .iter()
                    .find(|gap| gap.figure == "context_percent")
                    .map(|gap| gap.reason.clone());
            }
            reading.empty_call(usage)?;
            self.named_at = Some(self.named_at.map_or(usage.at_ms, |at| at.max(usage.at_ms)));
            return Ok(());
        }
        let previous = previous.cloned();
        let mut reading = Reading {
            session: session.clone(),
            at_ms: usage.at_ms,
            latest_at_ms: previous
                .as_ref()
                .map_or(usage.at_ms, |held| held.latest_at_ms.max(usage.at_ms)),
            figure: usage.context_percent,
            source: usage.context_source.clone(),
            model: usage.call.as_ref().and_then(|call| call.model.clone()),
            unavailable: usage
                .unavailable
                .iter()
                .find(|gap| gap.figure == "context_percent")
                .map(|gap| gap.reason.clone()),
            empty_calls: previous
                .as_ref()
                .map_or_else(BTreeMap::new, |held| held.empty_calls.clone()),
            reported_none: previous.as_ref().map_or(0, |held| held.reported_none),
        };
        if usage.context_percent.is_some() {
            reading.empty_calls.retain(|at, _| *at > usage.at_ms);
            reading.reported_none =
                reading
                    .empty_calls
                    .values()
                    .try_fold(0_usize, |sum, count| {
                        sum.checked_add(*count)
                            .ok_or("empty context call total overflows")
                    })?;
        } else {
            reading.empty_call(usage)?;
        }
        if let Some(reading) = previous
            && let Some(figure) = reading.figure
        {
            self.remove_source(figure, reading.source)?;
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
            let count = self
                .sources
                .entry(figure)
                .or_default()
                .entry(usage.context_source.clone())
                .or_default();
            *count = count
                .checked_add(1)
                .ok_or("context source count overflows")?;
            let count = self.figures.entry(figure).or_default();
            *count = count
                .checked_add(1)
                .ok_or("context figure count overflows")?;
            self.missing.remove(session);
        } else {
            self.missing.insert(session.clone());
        }
        self.sessions.insert(session.clone(), reading);
        self.named_at = Some(self.named_at.map_or(usage.at_ms, |at| at.max(usage.at_ms)));
        Ok(())
    }

    fn remove(&mut self, session: &str) -> Result<(), String> {
        if let Some(reading) = self.sessions.remove(session) {
            if let Some(figure) = reading.figure {
                self.remove_source(figure, reading.source)?;
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
            self.named_at = self
                .sessions
                .values()
                .map(|reading| reading.latest_at_ms)
                .max();
        }
        Ok(())
    }

    fn remove_source(&mut self, figure: u64, source: Option<String>) -> Result<(), String> {
        let sources = self
            .sources
            .get_mut(&figure)
            .ok_or("context source figure is absent")?;
        let std::collections::btree_map::Entry::Occupied(mut entry) = sources.entry(source) else {
            return Err("context source count is absent".to_owned());
        };
        let count = entry.get_mut();
        *count = count
            .checked_sub(1)
            .ok_or("context source count underflows")?;
        if *count == 0 {
            entry.remove();
        }
        if sources.is_empty() {
            self.sources.remove(&figure);
        }
        Ok(())
    }

    fn source(&self, figure: u64) -> Option<&str> {
        self.sources
            .get(&figure)
            .and_then(|sources| sources.keys().next_back())
            .and_then(Option::as_deref)
    }

    fn gap(&self, at_ms: i64) -> Option<&str> {
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
            self.missing
                .iter()
                .find_map(|session| {
                    self.sessions
                        .get(session)
                        .and_then(|reading| reading.unavailable.as_deref())
                })
                .or(Some(NO_CONTEXT))
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
    pub(crate) fn reading(&self, agent: &str, session: &str) -> Option<(i64, Option<u64>)> {
        self.agents
            .get(agent)
            .and_then(|agent| agent.sessions.get(session))
            .map(|reading| (reading.at_ms, reading.figure))
    }

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
        let mut figure: Option<&Reading> = None;
        let mut gap = None;
        let mut unmeasured = None;
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
                named_at = Some(
                    named_at.map_or(reading.latest_at_ms, |at: i64| at.max(reading.latest_at_ms)),
                );
                if reading.latest_at_ms > at_ms {
                    gap = Some(FUTURE);
                    unmeasured = Some(reading);
                    break;
                }
                let Some(current) = reading.figure else {
                    gap = Some(reading.unavailable.as_deref().unwrap_or(NO_CONTEXT));
                    unmeasured = Some(reading);
                    break;
                };
                let current = (Some(current), reading.source.as_deref(), reading.at_ms);
                if figure
                    .is_none_or(|held| current > (held.figure, held.source.as_deref(), held.at_ms))
                {
                    figure = Some(reading);
                }
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
        context_used(limit, figure, gap, unmeasured)
    }

    /// Read covered agents' current maximum, refusing to invent a figure for a gap.
    pub(crate) fn used(&self, limit: &Limit, agents: &BTreeSet<String>, at_ms: i64) -> Used {
        let mut figure: Option<&Reading> = None;
        for agent in agents.iter().filter_map(|agent| self.agents.get(agent)) {
            if let Some(reason) = agent.gap(at_ms) {
                let unmeasured = agent
                    .missing
                    .iter()
                    .filter_map(|session| agent.sessions.get(session))
                    .find(|reading| reading.unavailable.as_deref() == Some(reason))
                    .or_else(|| {
                        agent
                            .sessions
                            .values()
                            .max_by_key(|reading| reading.latest_at_ms)
                    });
                return context_used(limit, None, Some(reason), unmeasured);
            }
            if let Some((&highest, _)) = agent.figures.last_key_value() {
                // Equal percentages retain the greatest source name, so profile precedes model.
                let source = agent.source(highest);
                if let Some(reading) = agent
                    .sessions
                    .values()
                    .filter(|reading| {
                        reading.figure == Some(highest) && reading.source.as_deref() == source
                    })
                    .max_by_key(|reading| reading.at_ms)
                    && figure.is_none_or(|held| {
                        (reading.figure, source, reading.at_ms)
                            > (held.figure, held.source.as_deref(), held.at_ms)
                    })
                {
                    figure = Some(reading);
                }
            }
        }
        context_used(limit, figure, None, None)
    }
}

fn context_used(
    limit: &Limit,
    reading: Option<&Reading>,
    gap: Option<&str>,
    unmeasured: Option<&Reading>,
) -> Used {
    let reported = gap.is_none().then_some(reading).flatten();
    Used {
        unit: limit.unit,
        period: limit.period,
        figure: reported.and_then(|reading| reading.figure).map(Into::into),
        source: reported.and_then(|reading| reading.source.clone()),
        since_ms: None,
        unavailable: gap
            .or_else(|| reading.is_none().then_some(NO_CONTEXT))
            .map(str::to_owned),
        context: reading.or(unmeasured).map(Reading::context),
        reported: None,
        account: None,
        tokens: None,
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
