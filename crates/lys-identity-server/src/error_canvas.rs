//! A person's canvas's refusals: a canvas that cannot be read or kept, and a
//! layout whose name cannot be kept.

use axum::http::StatusCode;

/// A refusal of a person's canvas.
#[derive(Debug, thiserror::Error)]
pub enum CanvasError {
    /// The person's canvas could not be read or kept.
    #[error("CanvasUnavailable: {reason}")]
    Unavailable {
        /// Why.
        reason: String,
    },
    /// What was sent cannot be kept as it is: the words say what to change.
    #[error("CanvasRefused: {words}")]
    Refused {
        /// What to change.
        words: String,
    },
}

impl CanvasError {
    pub(crate) const fn name(&self) -> &'static str {
        match self {
            Self::Unavailable { .. } => "CanvasUnavailable",
            Self::Refused { .. } => "CanvasRefused",
        }
    }

    pub(crate) const fn status(&self) -> StatusCode {
        match self {
            Self::Unavailable { .. } => StatusCode::SERVICE_UNAVAILABLE,
            Self::Refused { .. } => StatusCode::BAD_REQUEST,
        }
    }
}
