//! Requests and public operation receipts.
use crate::protocol::Ended;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// What an operation asks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "request", rename_all = "snake_case", deny_unknown_fields)]
pub enum OperationRequest {
    /// Type the harness's compaction command at the next turn boundary.
    Compact {
        /// The command.
        text: String,
    },
    /// Compact one context crossing while keeping normal input held.
    ContextCompact {
        /// The configured compaction command.
        text: String,
        /// The existing crossing identity.
        crossing: String,
    },
    /// Queue a stable goal occurrence for current boundary authority.
    GoalReminder {
        /// The initial reminder words.
        text: String,
        /// The saved goal revision and occurrence.
        reference: crate::harness_control::ReminderReference,
    },
    /// Apply the service's current decision to one owned boundary.
    BoundaryReply {
        /// The exact generation and boundary decision.
        reply: crate::harness_control::BoundaryReply,
    },
    /// Type a notice to the session at the next turn boundary.
    Notice {
        /// The notice.
        text: String,
    },
    /// Type a reminder at the next turn boundary.
    Reminder {
        /// The reminder.
        text: String,
    },
    /// Rendered wake words held until the named compaction is confirmed.
    WakeUp {
        /// The words to type at the following boundary.
        text: String,
        /// The compaction whose confirmation releases these words.
        compaction: String,
    },
    /// End the session's process.
    Stop,
}

impl OperationRequest {
    /// Its name.
    #[must_use]
    pub fn name(&self) -> &'static str {
        match self {
            Self::Compact { .. } => "compact",
            Self::ContextCompact { .. } => "context_compact",
            Self::GoalReminder { .. } => "goal_reminder",
            Self::BoundaryReply { .. } => "boundary_reply",
            Self::Notice { .. } => "notice",
            Self::Reminder { .. } => "reminder",
            Self::WakeUp { .. } => "wake_up",
            Self::Stop => "stop",
        }
    }

    pub(super) fn text(&self) -> Option<&str> {
        match self {
            Self::Compact { text }
            | Self::ContextCompact { text, .. }
            | Self::GoalReminder { text, .. }
            | Self::Notice { text }
            | Self::Reminder { text }
            | Self::WakeUp { text, .. } => Some(text),
            Self::Stop | Self::BoundaryReply { .. } => None,
        }
    }
}

/// An operation as it is asked.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Operation {
    /// Its stable id, chosen and kept by the server.
    pub operation: String,
    /// The session.
    pub session: String,
    /// What it asks.
    pub request: OperationRequest,
}

/// Where an operation stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum OperationState {
    /// Accepted, waiting for its boundary.
    Accepted,
    /// Being typed; a runner stopped here leaves it uncertain.
    Delivering,
    /// Typed, or for a stop, the end sent.
    Delivered,
    /// Seen done: a compaction begun, a stop's exit seen.
    Confirmed,
    /// Its delivery cannot be known; it is never typed again.
    Uncertain,
    /// Not done, by name.
    Refused,
}

/// Text as the record keeps it: its length and digest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct TextDigest {
    /// Its length in bytes.
    pub length: usize,
    /// The lowercase hex SHA-256 of its bytes.
    pub sha256: String,
}

impl TextDigest {
    /// `text`'s length and digest.
    #[must_use]
    pub fn of(text: &str) -> Self {
        Self {
            length: text.len(),
            sha256: crate::protocol::hex(&Sha256::digest(text.as_bytes())),
        }
    }
}

/// An operation's outcome.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperationOutcome {
    /// The operation.
    pub operation: String,
    /// The session.
    pub session: String,
    /// What it asked, by name.
    pub request: String,
    /// Where it stands.
    pub state: OperationState,
    /// When it came to stand there, in milliseconds since the Unix epoch.
    pub at: u64,
    /// Why, in words.
    pub words: String,
    /// The text it types, as length and digest.
    pub text: Option<TextDigest>,
    /// The session's end, for a confirmed stop.
    pub ended: Option<Ended>,
}
