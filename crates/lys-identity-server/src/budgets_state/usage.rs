//! Stored usage retains the source of each context percentage.

use serde::{Deserialize, Serialize};

use super::{Crossing, Held, Measure};

#[cfg(test)]
#[path = "usage_tests.rs"]
mod tests;

/// A measured use one agent made, as the runner's feed reported it.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Usage {
    /// The feed event's stable id: an event seen again charges nothing.
    pub event: String,
    /// The agent.
    pub agent: String,
    /// When, in milliseconds since the Unix epoch.
    pub at_ms: i64,
    /// Tokens used.
    pub tokens: u64,
    /// Running time, in milliseconds.
    pub running_ms: u64,
    /// Reported dollar spend; absent is not zero.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dollars_micros: Option<u64>,
    /// The account that the native record attributes the figures to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub account: Option<String>,
    /// Account windows observed by this snapshot; absent spend records do not replace them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan_windows: Option<Vec<lys_runner::tracking_budget::PlanWindow>>,
    /// The native cumulative running-time baseline, when reported.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reported_running_ms: Option<u64>,
    /// A native snapshot updates availability and account levels, even without spend.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub native_snapshot: bool,
    /// Each unavailable figure and its reported reason.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unavailable: Vec<lys_runner::tracking::Unavailable>,
    /// The session it was measured in, when one is named.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
    /// The session's context, in percent of its window, when measured.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_percent: Option<u64>,
    /// The session's context in tokens, when measured; absent in older records.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_tokens: Option<u64>,
    /// The declared window `context_percent` is of, in tokens; absent in older records.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_window: Option<u64>,
    /// The declaration that supplied this context window, absent in older records.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_source: Option<String>,
    /// The model call this use counts, when the proxy saw it pass.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub call: Option<Box<CallSeen>>,
    /// The budgets this use crossed, kept with it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub crossed: Vec<Crossing>,
}

impl Usage {
    /// A status snapshot that observed no account window must leave earlier window readings alone.
    pub(crate) fn clear_unobserved_windows(&mut self) {
        if self.plan_windows.as_ref().is_some_and(Vec::is_empty)
            && !self
                .unavailable
                .iter()
                .any(|gap| gap.figure == "plan_windows")
        {
            self.plan_windows = None;
        }
    }
}

impl Held {
    /// Count the same indexed calls that the authorised call list exposes, independently of crossings.
    ///
    /// # Errors
    /// Names a missing or mismatched indexed call, invalid page order, or an overflowing count.
    pub fn reported_calls(&self, agent: &str) -> Result<usize, String> {
        let mut before = None;
        let mut count = 0_usize;
        loop {
            // Paging counts every indexed call without allocating a vector for the entire history.
            let (page, more) = self.index.calls(agent, before, 128);
            if page.is_empty() && more {
                return Err("call index returned an empty page while older calls remain".to_owned());
            }
            count = count
                .checked_add(page.len())
                .ok_or("reported call count overflows")?;
            for position in page {
                let usage = self
                    .uses
                    .get(position)
                    .ok_or_else(|| format!("call index names missing record {position}"))?;
                if usage.agent != agent || usage.call.is_none() {
                    return Err(format!(
                        "call index names record {position} without a call for agent {agent}"
                    ));
                }
                let coordinate = (usage.at_ms, position);
                if before.is_some_and(|before| coordinate >= before) {
                    return Err(format!(
                        "call index record {position} is not older than its page boundary"
                    ));
                }
                before = Some(coordinate);
            }
            if !more {
                return Ok(count);
            }
        }
    }

    /// Retain each account's reported duration through gaps, until its provider's own reset.
    ///
    /// # Errors
    /// Names an invalid observation instant, inconsistent index, or missing paying account.
    pub fn account_windows(
        &self,
        agent: &str,
        at_ms: i64,
    ) -> Result<Vec<crate::budgets_figures::AccountWindows>, String> {
        use std::collections::{BTreeMap, BTreeSet};
        let now = u64::try_from(at_ms)
            .map_err(|error| format!("account observation is before the Unix epoch: {error}"))?;
        let agents = BTreeSet::from([agent.to_owned()]);
        let mut selected = BTreeMap::new();
        for position in self
            .index
            .positions(&agents, Measure::PlanPercent, at_ms, None)
        {
            let usage = self
                .uses
                .get(position)
                .ok_or_else(|| format!("account window index names missing record {position}"))?;
            let windows = usage
                .plan_windows
                .as_ref()
                .ok_or("account window index names a record without a plan observation")?;
            if windows.is_empty() {
                continue;
            }
            let account = usage
                .account
                .as_deref()
                .filter(|account| !account.is_empty())
                .ok_or("the reported plan window names no account")?;
            for window in windows {
                let latest = selected
                    .entry((account, window.duration_minutes))
                    .or_insert((usage.at_ms, position, window));
                if (usage.at_ms, position) > (latest.0, latest.1) {
                    *latest = (usage.at_ms, position, window);
                }
            }
        }
        let mut accounts: BTreeMap<&str, (i64, Vec<lys_runner::tracking_budget::PlanWindow>)> =
            BTreeMap::new();
        for ((account, _), (reported_at, _, window)) in selected {
            if window.resets_at_ms <= now {
                continue;
            }
            let (at, windows) = accounts.entry(account).or_insert((reported_at, Vec::new()));
            *at = (*at).max(reported_at);
            windows.push(window.clone());
        }
        Ok(accounts
            .into_iter()
            .map(
                |(account, (at_ms, windows))| crate::budgets_figures::AccountWindows {
                    account: account.to_owned(),
                    at_ms,
                    windows,
                },
            )
            .collect())
    }
}

/// A model call the proxy saw pass, as the use that counts it keeps it: what
/// a list of an agent's calls shows, and where the call is kept whole on the
/// machine it was made on. A figure the response did not report is absent,
/// never zero.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CallSeen {
    /// The proxy's id for the call.
    pub id: String,
    /// The model asked for.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Input tokens, as the provider counted them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_tokens: Option<u64>,
    /// Output tokens.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_tokens: Option<u64>,
    /// Tokens written to the cache.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_creation_tokens: Option<u64>,
    /// Tokens read from the cache.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_read_tokens: Option<u64>,
    /// The run key the call carried.
    pub run: String,
    /// Where the proxy keeps the call whole, when its usage line said.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub record: Option<lys_runner::tracking::RecordAt>,
}
