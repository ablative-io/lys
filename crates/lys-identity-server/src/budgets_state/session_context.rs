//! Canvas context reads the newest live session without changing budget enforcement.

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;
use serde_json::Number;

use crate::budgets_context::{ContextReading, NO_CONTEXT};
use crate::budgets_feed::Windows;
use crate::budgets_limits::Limit;
use crate::budgets_state::{CallSeen, Held, Measure, Usage};
use crate::budgets_usage::Used;

#[cfg(test)]
#[path = "session_context_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "session_side_work_tests.rs"]
mod side_work_tests;

/// One live session's own last reported context and later incomplete calls.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
pub struct SessionReading {
    /// The live runtime session.
    pub session: String,
    /// When its context counters were reported, absent until a complete call.
    pub read_at_ms: Option<i64>,
    /// Later calls that omitted at least one context counter.
    pub reported_none: usize,
    /// Whether a call completed; unknown when its source did not record an outcome.
    pub completed_call: Option<bool>,
    /// The reading's model, or the latest incomplete call's model.
    pub model: Option<String>,
    /// Input and cached input tokens, excluding output.
    pub context_tokens: Option<u64>,
    /// The model's currently declared context window.
    pub window_tokens: Option<u64>,
    /// The percentage of that declared window.
    #[schema(value_type = Option<f64>)]
    pub figure: Option<Number>,
    /// The named missing input or invalid declaration.
    pub unavailable: Option<String>,
}

struct Session {
    started_ms: i64,
    reading: SessionReading,
    source: Option<String>,
    saw_call: bool,
    complete: bool,
}

impl Session {
    fn new(id: &str, started_ms: i64) -> Self {
        Self {
            started_ms,
            reading: SessionReading {
                session: id.to_owned(),
                read_at_ms: None,
                reported_none: 0,
                completed_call: Some(false),
                model: None,
                context_tokens: None,
                window_tokens: None,
                figure: None,
                unavailable: Some(NO_CONTEXT.to_owned()),
            },
            source: None,
            saw_call: false,
            complete: false,
        }
    }

    fn observe(&mut self, usage: &Usage, windows: &Windows) -> Result<(), String> {
        let call = usage
            .call
            .as_ref()
            .ok_or("context call index selected a record without a call")?;
        if self.reading.completed_call != Some(true) {
            let completed = call
                .record
                .as_ref()
                .map(|record| record.status == "complete");
            self.reading.completed_call = match (self.reading.completed_call, completed) {
                (_, Some(true)) => Some(true),
                (Some(false), Some(false)) => Some(false),
                _ => None,
            };
        }
        let context = context_tokens(call)?;
        let model = call.model.as_deref();
        let model_window = model.and_then(|model| windows.get(model));
        if usage.context_source.as_deref() != Some("profile") && matches!(model_window, Some(None))
        {
            // Side work reports its own input, never the session's agent context.
            return self.unreported(
                call,
                "the call's model is declared as side work, which never sets an agent's context"
                    .to_owned(),
            );
        }
        let Some(context) = context else {
            return self.unreported(call, missing_counters(call));
        };
        self.saw_call = true;
        self.complete = true;
        self.reading.read_at_ms = Some(usage.at_ms);
        self.reading.model.clone_from(&call.model);
        self.reading.context_tokens = Some(context);
        if usage.context_source.as_deref() == Some("profile") {
            self.reading.unavailable =
                Some("the call's profile context window declaration was not retained".to_owned());
            return Ok(());
        }
        match model_window.and_then(Option::as_ref) {
            Some(0) => return Err("model_windows must declare positive tokens".to_owned()),
            Some(window) => {
                self.reading.window_tokens = Some(*window);
                if context > *window {
                    self.reading.unavailable = Some(format!(
                        "the call's context of {context} tokens is larger than the window of {window} declared for its model"
                    ));
                } else {
                    self.reading.figure = Some(context_percent(context, *window)?);
                    self.reading.unavailable = None;
                    self.source = Some("model".to_owned());
                }
            }
            None => {
                self.reading.unavailable = Some(format!(
                    "no context window is declared for model {}",
                    model.unwrap_or("(not named)")
                ));
            }
        }
        Ok(())
    }

    fn unreported(&mut self, call: &CallSeen, reason: String) -> Result<(), String> {
        self.reading.reported_none = self
            .reading
            .reported_none
            .checked_add(1)
            .ok_or("empty context call count overflows")?;
        if !self.saw_call {
            self.reading.model.clone_from(&call.model);
            self.reading.unavailable = Some(reason);
        }
        self.saw_call = true;
        Ok(())
    }
}

fn context_tokens(call: &CallSeen) -> Result<Option<u64>, String> {
    match (
        call.input_tokens,
        call.cache_creation_tokens,
        call.cache_read_tokens,
    ) {
        (Some(input), Some(created), Some(read)) => input
            .checked_add(created)
            .and_then(|tokens| tokens.checked_add(read))
            .map(Some)
            .ok_or_else(|| "reported context input and cache token counters overflow".to_owned()),
        _ => Ok(None),
    }
}

fn missing_counters(call: &CallSeen) -> String {
    let missing: Vec<_> = [
        ("input_tokens", call.input_tokens),
        ("cache_creation_tokens", call.cache_creation_tokens),
        ("cache_read_tokens", call.cache_read_tokens),
    ]
    .into_iter()
    .filter_map(|(name, figure)| figure.is_none().then_some(name))
    .collect();
    format!("context counters not reported: {}", missing.join(", "))
}

fn context_percent(context: u64, window: u64) -> Result<Number, String> {
    let micros = u128::from(context)
        .checked_mul(100_000_000)
        .and_then(|scaled| scaled.checked_div(u128::from(window)))
        .and_then(|micros| u64::try_from(micros).ok())
        .ok_or("context percentage cannot be represented for its declared window")?;
    crate::budgets_usage::number(Measure::ContextPercent, micros)
}

fn validate_call(usage: &Usage, agent: &str, position: usize) -> Result<(), String> {
    let call = usage.call.as_ref().ok_or_else(|| {
        format!("context call index names record {position} without a call for agent {agent}")
    })?;
    if usage.agent != agent {
        return Err(format!(
            "context call index names record {position} for a different agent"
        ));
    }
    if usage.session.as_deref().is_none_or(str::is_empty) {
        return Err(format!(
            "context call index record {position} names no session"
        ));
    }
    if call.id.is_empty() || call.model.as_deref().is_some_and(str::is_empty) {
        return Err(format!(
            "context call index record {position} has an empty call or model attribution"
        ));
    }
    if usage
        .context_source
        .as_deref()
        .is_some_and(|source| !matches!(source, "model" | "profile"))
    {
        return Err(format!(
            "context call index record {position} has an unknown context declaration source"
        ));
    }
    Ok(())
}

impl Held {
    /// Read each live session's own context, selecting the session that started last.
    ///
    /// # Errors
    /// Names invalid live attribution, an inconsistent call index, overflowing counters,
    /// or an invalid declared model window. This display does not alter enforcement.
    pub fn session_context(
        &self,
        limit: &Limit,
        live: &BTreeMap<String, BTreeMap<String, u64>>,
        windows: &Windows,
        at_ms: i64,
    ) -> Result<Used, String> {
        if limit.unit != Measure::ContextPercent || limit.period.is_some() {
            return Err("session context requires context_percent without a period".to_owned());
        }
        let now = u64::try_from(at_ms)
            .map_err(|error| format!("context observation is before the Unix epoch: {error}"))?;
        let mut seen = BTreeSet::new();
        let mut readings = Vec::new();
        for (agent, live_sessions) in live {
            if agent.is_empty() {
                return Err("live context attribution names an empty agent".to_owned());
            }
            let mut sessions = BTreeMap::new();
            for (session, started_at) in live_sessions {
                if session.is_empty() || !seen.insert(session.as_str()) {
                    return Err("live context session attribution is empty or repeated".to_owned());
                }
                let started_ms = started_at
                    .checked_mul(1000)
                    .ok_or("live context session start overflows milliseconds")?;
                if started_ms > now {
                    return Err(format!(
                        "live context session {session} starts after the observation time"
                    ));
                }
                sessions.insert(
                    session.as_str(),
                    Session::new(
                        session,
                        i64::try_from(started_ms).map_err(|error| {
                            format!("live context start cannot be represented: {error}")
                        })?,
                    ),
                );
            }
            self.read_context_calls(agent, &mut sessions, windows, at_ms)?;
            for (session, reading) in sessions {
                let started = live_sessions
                    .get(session)
                    .ok_or("live context session has no runtime start")?;
                readings.push((*started, reading));
            }
        }
        readings.sort_by(|(left_at, left), (right_at, right)| {
            (right_at, &right.reading.session).cmp(&(left_at, &left.reading.session))
        });
        let selected = readings.first().map(|(_, session)| session);
        Ok(Used {
            unit: limit.unit,
            period: limit.period,
            figure: selected.and_then(|session| session.reading.figure.clone()),
            source: selected.and_then(|session| session.source.clone()),
            context: selected.map(|session| ContextReading {
                session: Some(session.reading.session.clone()),
                read_at_ms: session.reading.read_at_ms,
                reported_none: session.reading.reported_none,
                completed_call: session.reading.completed_call,
                model: session.reading.model.clone(),
                context_tokens: session.reading.context_tokens,
                window_tokens: session.reading.window_tokens,
                live_sessions: readings
                    .iter()
                    .map(|(_, session)| session.reading.clone())
                    .collect(),
            }),
            tokens: None,
            since_ms: None,
            unavailable: selected.map_or_else(
                || Some("the agent has no live session to report context".to_owned()),
                |session| session.reading.unavailable.clone(),
            ),
            reported: None,
            account: None,
        })
    }

    fn read_context_calls(
        &self,
        agent: &str,
        sessions: &mut BTreeMap<&str, Session>,
        windows: &Windows,
        at_ms: i64,
    ) -> Result<(), String> {
        let mut before = Some((at_ms, usize::MAX));
        while sessions.values().any(|session| !session.complete) {
            let (page, more) = self.index.calls(agent, before, 128);
            if page.is_empty() && more {
                return Err(
                    "context call index returned an empty page while older calls remain".to_owned(),
                );
            }
            for position in page {
                let usage = self
                    .uses
                    .get(position)
                    .ok_or_else(|| format!("context call index names missing record {position}"))?;
                let earliest = sessions
                    .values()
                    .filter(|session| !session.complete)
                    .map(|session| session.started_ms)
                    .min();
                if earliest.is_none_or(|started| usage.at_ms < started) {
                    return Ok(());
                }
                let coordinate = (usage.at_ms, position);
                if usage.at_ms > at_ms || before.is_some_and(|before| coordinate >= before) {
                    return Err(format!(
                        "context call index record {position} is ahead of its page boundary"
                    ));
                }
                before = Some(coordinate);
                if let Some(session) = usage
                    .session
                    .as_deref()
                    .and_then(|session| sessions.get_mut(session))
                    && !session.complete
                {
                    validate_call(usage, agent, position)?;
                    session.observe(usage, windows)?;
                }
            }
            if !more {
                break;
            }
        }
        Ok(())
    }
}
