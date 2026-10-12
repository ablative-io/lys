//! Stored usage retains the source of each context percentage.

use serde::{Deserialize, Serialize};

use super::Crossing;

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
