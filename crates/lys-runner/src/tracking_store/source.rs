//! Accounting cursors keep an observed native conversation as separate evidence.

use serde::{Deserialize, Serialize};

use super::{Pending, Totals};
use crate::error::RunnerError;
use crate::tracking::Figures;

/// A session's stream, as far as it has been read.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceState {
    /// The file followed.
    pub path: String,
    /// Its generation: raised when it is truncated or replaced.
    pub generation: u64,
    /// The bytes read: every byte before it is a whole line consumed.
    pub offset: u64,
    /// The file's device and inode, when it has been opened.
    pub identity: Option<String>,
    /// The accounting stream identity: a native id or the proxy run key.
    pub bound: String,
    /// The native conversation observed independently of the accounting stream.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub native_conversation: Option<String>,
    /// The turn under way, when the source named one.
    pub turn: Option<String>,
    /// A response not yet shown whole.
    pub pending: Option<Pending>,
    /// The last totals a Codex rollout gave.
    pub totals: Option<Totals>,
    /// The last status-line snapshot kept.
    pub snapshot: Option<Figures>,
    /// The latest reported cost, retained when a later snapshot omits it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reported_cost_micros: Option<u64>,
    /// The latest account windows observed in a rollout.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub plan_windows: Vec<crate::tracking_budget::PlanWindow>,
    /// The attribution of the last status-line snapshot.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snapshot_account: Option<String>,
    /// The attribution of the last rollout window report.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan_account: Option<String>,
}

impl SourceState {
    pub(crate) fn check_native_conversation(&self, conversation: &str) -> Result<(), RunnerError> {
        if self
            .native_conversation
            .as_deref()
            .is_some_and(|kept| kept != conversation)
        {
            return Err(RunnerError::refused(
                "native_conversation_changed",
                "the harness named a different native conversation than the one already recorded",
            ));
        }
        Ok(())
    }
}

/// What a unit consumed, kept with it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Commit {
    /// A stream's state after the unit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<SourceState>,
    /// A refusal's attempt, so a repeat reuses it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attempt: Option<String>,
}

/// Where a session's coverage stands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Coverage {
    /// The state, by name: `tracking_started`, `trust_recorded`,
    /// `trust_answered`, `trust_dialog_unanswered`,
    /// `source_bound`, `source_generation`, `record_unreadable`,
    /// `source_refused`, `coverage_incomplete` or `audit_incomplete`.
    pub state: String,
    /// The stream it concerns, when it concerns one.
    pub source: Option<String>,
    /// The stream's generation.
    pub generation: u64,
    /// The byte offset it concerns, when it concerns one.
    pub offset: Option<u64>,
    /// Why, in words.
    pub words: String,
    /// The executable launched, when it was.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub executable: Option<String>,
    /// The version the executable reported.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub harness_version: Option<String>,
    /// The adapter that reads it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adapter: Option<String>,
}

impl Coverage {
    /// Coverage `state` of `source` at `offset`, in `words`.
    #[must_use]
    pub fn of(state: &str, source: &SourceState, offset: Option<u64>, words: String) -> Self {
        Self {
            state: state.to_owned(),
            source: Some(source.path.clone()),
            generation: source.generation,
            offset,
            words,
            executable: None,
            harness_version: None,
            adapter: None,
        }
    }
}
