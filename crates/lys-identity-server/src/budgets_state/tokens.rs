//! Token components retain their reported subtotals and each component's missing calls.

use std::collections::BTreeSet;

use serde::Serialize;

use crate::budgets_limits::Limit;
use crate::budgets_state::{Held, Measure, Usage};

use super::start;

#[cfg(test)]
#[path = "tokens_tests.rs"]
mod tests;

/// What calls reported for one token component, independently of the other components.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
pub struct TokenComponent {
    /// Sum of the figures actually reported; absent when no call reported this component.
    pub reported: Option<u64>,
    /// Calls that reported no figure for this component.
    pub missing_calls: usize,
}

impl TokenComponent {
    fn add(&mut self, value: Option<u64>, name: &str) -> Result<(), String> {
        if let Some(value) = value {
            self.reported = Some(match self.reported {
                Some(earlier) => earlier
                    .checked_add(value)
                    .ok_or_else(|| format!("reported {name} subtotal overflows"))?,
                None => value,
            });
        } else {
            self.missing_calls = self
                .missing_calls
                .checked_add(1)
                .ok_or_else(|| format!("{name} missing call count overflows"))?;
        }
        Ok(())
    }
}

/// The separate token components and recorded call coverage of one selected period.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
pub struct TokenComponents {
    /// Newly supplied input tokens, as the call record reported them.
    pub input_tokens: TokenComponent,
    /// Generated output tokens.
    pub output_tokens: TokenComponent,
    /// Tokens written to the cache.
    pub cache_creation_tokens: TokenComponent,
    /// Tokens read from the cache.
    pub cache_read_tokens: TokenComponent,
    /// All recorded model calls in the period, including calls that reported no figures.
    pub calls: usize,
    /// Distinct named sessions, counted separately for each agent.
    pub sessions: usize,
    /// Calls whose session was not named; the number of their distinct sessions is unknown.
    pub calls_without_session: usize,
}

impl TokenComponents {
    fn add<'a>(
        &mut self,
        usage: &'a Usage,
        sessions: &mut BTreeSet<(&'a str, &'a str)>,
    ) -> Result<(), String> {
        let call = usage
            .call
            .as_ref()
            .ok_or("token component index names a record without a call")?;
        self.calls = self
            .calls
            .checked_add(1)
            .ok_or("token component call count overflows")?;
        if let Some(session) = usage.session.as_ref().filter(|session| !session.is_empty()) {
            sessions.insert((usage.agent.as_str(), session.as_str()));
        } else {
            self.calls_without_session = self
                .calls_without_session
                .checked_add(1)
                .ok_or("token component unnamed-session call count overflows")?;
        }
        self.input_tokens.add(call.input_tokens, "input_tokens")?;
        self.output_tokens
            .add(call.output_tokens, "output_tokens")?;
        self.cache_creation_tokens
            .add(call.cache_creation_tokens, "cache_creation_tokens")?;
        self.cache_read_tokens
            .add(call.cache_read_tokens, "cache_read_tokens")
    }

    fn agent<'a>(
        &mut self,
        held: &'a Held,
        agent: &str,
        since: i64,
        at_ms: i64,
        sessions: &mut BTreeSet<(&'a str, &'a str)>,
    ) -> Result<(), String> {
        let mut before = Some((at_ms, usize::MAX));
        loop {
            let (page, more) = held.index.calls(agent, before, 128);
            if page.is_empty() && more {
                return Err(
                    "token component index returned an empty page while older calls remain"
                        .to_owned(),
                );
            }
            for position in page {
                let usage = held.uses.get(position).ok_or_else(|| {
                    format!("token component index names missing record {position}")
                })?;
                if usage.agent != agent || usage.call.is_none() {
                    return Err(format!(
                        "token component index names record {position} without a call for agent {agent}"
                    ));
                }
                let coordinate = (usage.at_ms, position);
                if before.is_some_and(|before| coordinate >= before) {
                    return Err(format!(
                        "token component index record {position} is not older than its page boundary"
                    ));
                }
                before = Some(coordinate);
                if usage.at_ms < since {
                    return Ok(());
                }
                self.add(usage, sessions)?;
            }
            if !more {
                return Ok(());
            }
        }
    }
}

impl Held {
    /// Read separate reported token components from indexed calls in the limit's calendar period.
    ///
    /// # Errors
    /// Names a non-token or periodless request, invalid observation or period boundary, corrupt call index,
    /// or a component subtotal or coverage count that overflows.
    pub fn token_components(
        &self,
        limit: &Limit,
        agents: &BTreeSet<String>,
        zone: &str,
        at_ms: i64,
    ) -> Result<TokenComponents, String> {
        if limit.unit != Measure::Tokens {
            return Err("token components require the tokens unit".to_owned());
        }
        if at_ms < 0 {
            return Err("token component observation is before the Unix epoch".to_owned());
        }
        let since =
            start(limit, zone, at_ms)?.ok_or("token components require a declared period")?;
        let mut components = TokenComponents::default();
        let mut sessions = BTreeSet::new();
        for agent in agents {
            components.agent(self, agent, since, at_ms, &mut sessions)?;
        }
        components.sessions = sessions.len();
        Ok(components)
    }
}
