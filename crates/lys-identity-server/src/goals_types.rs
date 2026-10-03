//! Goal kinds, admission refusals and changes kept as log events.

use axum::http::StatusCode;
use serde::{Deserialize, Serialize};

/// Everything the goals refuse, each by name.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum GoalError {
    /// The goals are not configured, or their log could not be read or written.
    #[error("goals_unavailable: {reason}")]
    Unavailable {
        /// Why.
        reason: String,
    },
    /// No goal by that id is visible to the caller.
    #[error("goal_unknown: no goal by that id is visible to the caller")]
    Unknown,
    /// The operation id already names a goal act in other words.
    #[error(
        "goal_reused: operation `{operation}` already names a goal act in other words: send this act under a new operation id"
    )]
    Reused {
        /// The operation id.
        operation: String,
    },
    /// The goal is no longer open and takes no other mark.
    #[error("goal_closed: goal `{goal}` is already {standing}")]
    Closed {
        /// The goal.
        goal: String,
        /// Its standing.
        standing: &'static str,
    },
    /// The agent a goal judges asked to mark that goal.
    #[error(
        "not_your_judgement: agent `{agent}` is judged by this goal and may not mark it; its responsible person or a holder of the relation the goal names does"
    )]
    NotYourJudgement {
        /// The agent.
        agent: String,
    },
    /// A reminder before the deadline has no deadline to refer to.
    #[error("reminder_needs_deadline: a reminder before the deadline needs a deadline")]
    ReminderNeedsDeadline,
    /// New reminder words are empty or contain control characters.
    #[error("goal_words_malformed: {why}")]
    WordsMalformed {
        /// Why the words cannot be reminded.
        why: &'static str,
    },
    /// A deliverable names no evidence, or is marked met without a claim of it.
    #[error("evidence_missing: {why}")]
    EvidenceMissing {
        /// What is missing.
        why: &'static str,
    },
}

impl GoalError {
    /// How the refusal is answered over HTTP.
    pub(crate) fn status(&self) -> StatusCode {
        match self {
            Self::Unavailable { .. } => StatusCode::SERVICE_UNAVAILABLE,
            Self::Unknown => StatusCode::NOT_FOUND,
            Self::Reused { .. } | Self::Closed { .. } => StatusCode::CONFLICT,
            Self::NotYourJudgement { .. } => StatusCode::FORBIDDEN,
            Self::EvidenceMissing { .. }
            | Self::ReminderNeedsDeadline
            | Self::WordsMalformed { .. } => StatusCode::BAD_REQUEST,
        }
    }
}

/// What an item is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[schema(as = GoalKind)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    /// An outcome.
    Goal,
    /// A standard the holder's work is held to.
    Expectation,
    /// A named thing handed over, with the evidence that proves it.
    Deliverable,
}

/// What proves a deliverable was handed over.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[schema(as = GoalEvidence)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceKind {
    /// A landed commit.
    Commit,
    /// A document.
    Document,
    /// A passing check.
    Check,
}

/// Where an item stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[schema(as = GoalStanding)]
#[serde(rename_all = "snake_case")]
pub enum Standing {
    /// Not yet judged.
    Open,
    /// Judged met.
    Met,
    /// Judged missed.
    Missed,
    /// Dropped by its responsible person or judge.
    Dropped,
}

impl Standing {
    /// Its name.
    pub fn name(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Met => "met",
            Self::Missed => "missed",
            Self::Dropped => "dropped",
        }
    }
}

/// What holds an item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[schema(as = GoalHolderKind)]
#[serde(rename_all = "snake_case")]
pub enum HolderKind {
    /// An agent.
    Agent,
    /// A team.
    Team,
}

/// The agent or team an item is held on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[schema(as = GoalHolder)]
#[serde(deny_unknown_fields)]
pub struct Holder {
    /// An agent or a team.
    pub kind: HolderKind,
    /// Its id.
    pub id: String,
}

/// An event a reminder waits on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[schema(as = GoalEvent)]
#[serde(rename_all = "snake_case")]
pub enum Event {
    /// A session of the holder compacted its context.
    Compaction,
}

/// When a reminder falls due.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[schema(as = GoalReminder)]
#[serde(tag = "when", rename_all = "snake_case", deny_unknown_fields)]
pub enum Remind {
    /// Once, this many seconds before the deadline, and never before the
    /// item was set.
    Before {
        /// Seconds before the deadline.
        seconds: u64,
    },
    /// Every this many seconds from when the item was set, until closed or its optional deadline.
    Every {
        /// Seconds between reminders.
        seconds: u64,
    },
    /// Each time the event is kept for the item.
    On {
        /// The event.
        event: Event,
    },
}

/// A change to a standing aim, kept separately from the words originally set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(tag = "field", rename_all = "snake_case", deny_unknown_fields)]
pub enum Change {
    /// Suspend or resume reminders without judging the aim.
    Active {
        /// Whether the aim is active.
        active: bool,
    },
    /// Replace the words used by later reminders.
    Words {
        /// The new reminder-safe words.
        words: String,
    },
}

impl Change {
    /// Refuse new words that cannot be reminded as one line.
    pub fn check(&self) -> Result<(), GoalError> {
        if let Self::Words { words } = self {
            if words.trim().is_empty() {
                return Err(GoalError::WordsMalformed {
                    why: "words are empty",
                });
            }
            if words.chars().any(char::is_control) {
                return Err(GoalError::WordsMalformed {
                    why: "words are one line of text: a reminder types them into a session, so no newline or control character",
                });
            }
        }
        Ok(())
    }
}

/// One authorized, idempotent change to an aim.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct Changed {
    /// The operation id naming the change.
    pub operation: String,
    /// The aim changed.
    pub goal: String,
    /// Its new activity or words.
    pub change: Change,
    /// The identity admitted to change it.
    pub by: String,
    /// When it changed, in seconds since the Unix epoch.
    pub at: u64,
}

pub(crate) fn active_default() -> bool {
    true
}
