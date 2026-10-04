//! The master off switch's refusals: a start while everything is stopped,
//! an operation id spent on another pull or release, a release with nothing
//! stopped, and a cord record that cannot be read or kept.

use axum::http::StatusCode;

/// A refusal of the master off switch, or of a start it holds back.
#[derive(Debug, thiserror::Error)]
pub enum CordError {
    /// Everything is stopped, so no agent may start: the words say who
    /// stopped it, when and why.
    #[error("everything_stopped: {words}")]
    EverythingStopped {
        /// Who stopped everything, when and why, in plain words.
        words: String,
    },
    /// The operation id already names another pull or release.
    #[error(
        "cord_reused: operation `{operation}` already names another stop of everything or its release: send this one under a new operation id"
    )]
    Reused {
        /// The operation id.
        operation: String,
    },
    /// A release was asked while nothing is stopped.
    #[error("cord_not_pulled: nothing is stopped, so there is nothing to let start again")]
    NotPulled,
    /// The cord's record could not be read or kept.
    #[error("cord_unavailable: {reason}")]
    Unavailable {
        /// Why.
        reason: String,
    },
}

impl CordError {
    pub(crate) const fn name(&self) -> &'static str {
        match self {
            Self::EverythingStopped { .. } => "everything_stopped",
            Self::Reused { .. } => "cord_reused",
            Self::NotPulled => "cord_not_pulled",
            Self::Unavailable { .. } => "cord_unavailable",
        }
    }

    pub(crate) const fn status(&self) -> StatusCode {
        match self {
            Self::EverythingStopped { .. } | Self::Reused { .. } | Self::NotPulled => {
                StatusCode::CONFLICT
            }
            Self::Unavailable { .. } => StatusCode::SERVICE_UNAVAILABLE,
        }
    }
}
