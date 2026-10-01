//! An actor cannot act on what is absent from its own current holding: a
//! budget it gives, a team it changes, a person or account it writes.

use axum::http::StatusCode;

/// A request reaches beyond the authenticated actor's current holding.
#[derive(Debug, thiserror::Error)]
pub enum HoldingError {
    /// The actor holds nothing that covers the request; the reason names what.
    #[error("HoldingNotHeld: {giver} does not hold what it is acting on: {reason}")]
    NotHeld {
        /// The authenticated giver, never a supplied substitute.
        giver: String,
        /// What is uncovered: a budget dimension or amount, a team, a person.
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
