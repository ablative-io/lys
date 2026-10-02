//! A giver cannot convey a budget absent from its own current holding.

use axum::http::StatusCode;

/// A budget giving request exceeds the authenticated giver's holding.
#[derive(Debug, thiserror::Error)]
pub enum HoldingError {
    /// The giver has no covering current limit, or its amount is smaller.
    #[error("HoldingNotHeld: {giver} does not hold the budget it is giving: {reason}")]
    NotHeld {
        /// The authenticated giver, never a supplied substitute.
        giver: String,
        /// The uncovered dimension or amount.
        reason: String,
    },
}

impl HoldingError {
    pub(crate) const fn name(&self) -> &'static str {
        match self {
            Self::NotHeld { .. } => "HoldingNotHeld",
        }
    }

    pub(crate) const fn status(&self) -> StatusCode {
        match self {
            Self::NotHeld { .. } => StatusCode::FORBIDDEN,
        }
    }
}
