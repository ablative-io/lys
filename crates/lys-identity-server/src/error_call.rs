//! Why an agent's model call could not be read.

use axum::http::StatusCode;

/// A refusal of the calls routes.
#[derive(Debug, thiserror::Error)]
pub enum CallError {
    /// No model call by that id is kept for the agent.
    #[error("CallUnknown: no model call by that id is kept for the agent")]
    Unknown,
    /// The call is kept whole on another computer.
    #[error(
        "CallKeptElsewhere: the call is kept whole on computer `{machine}`, where it was made; this service reads the calls kept on its own computer"
    )]
    KeptElsewhere {
        /// The computer whose runner reported the call.
        machine: String,
    },
    /// The proxy's call records are not configured or could not be read.
    #[error("CallRecordsUnavailable: {reason}")]
    RecordsUnavailable {
        /// What failed.
        reason: String,
    },
}

impl CallError {
    /// The refusal's stable name.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Unknown => "CallUnknown",
            Self::KeptElsewhere { .. } => "CallKeptElsewhere",
            Self::RecordsUnavailable { .. } => "CallRecordsUnavailable",
        }
    }

    /// The status the refusal is answered with.
    #[must_use]
    pub const fn status(&self) -> StatusCode {
        match self {
            Self::Unknown => StatusCode::NOT_FOUND,
            Self::KeptElsewhere { .. } => StatusCode::CONFLICT,
            Self::RecordsUnavailable { .. } => StatusCode::SERVICE_UNAVAILABLE,
        }
    }
}
