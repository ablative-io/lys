//! Budget and organisation setting refusals keep their response words and status together.

use axum::http::StatusCode;

/// Everything a budget or organisation setting change or read can refuse.
#[derive(Debug, thiserror::Error)]
pub enum BudgetError {
    /// The budgets cannot be kept or read.
    #[error("BudgetsUnavailable: {reason}")]
    BudgetsUnavailable {
        /// Why.
        reason: String,
    },
    /// The organisation setting cannot be kept or read.
    #[error("ConfigurationUnavailable: {reason}")]
    ConfigurationUnavailable {
        /// Why.
        reason: String,
    },
    /// A zone changed after the administrator read it.
    #[error(
        "ConfigurationVersionConflict: the setting is at version {held}, not {expected}; read it again"
    )]
    ConfigurationVersionConflict {
        /// The version held.
        held: u64,
        /// The version the caller read.
        expected: u64,
    },
    /// A current stop limit prevents a fresh start before runner admission.
    #[error("BudgetExhausted: {words}")]
    BudgetExhausted {
        /// The exhausted limit and its reported figure and reset.
        words: String,
    },
    /// Another change to the budget came between the caller's read and its
    /// change.
    #[error(
        "BudgetVersionConflict: the budget is at version {held}, not {expected}: read it again and send the change on the version read"
    )]
    BudgetVersionConflict {
        /// The version held.
        held: u64,
        /// The version the caller read.
        expected: u64,
    },
    /// A budget of the wrong shape, refused by name.
    #[error("{refusal}: {words}")]
    BudgetRefused {
        /// The refusal's name.
        refusal: &'static str,
        /// Why, in words.
        words: String,
    },
}

impl BudgetError {
    pub(crate) fn status(&self) -> StatusCode {
        match self {
            Self::BudgetsUnavailable { .. } | Self::ConfigurationUnavailable { .. } => {
                StatusCode::SERVICE_UNAVAILABLE
            }
            Self::ConfigurationVersionConflict { .. }
            | Self::BudgetExhausted { .. }
            | Self::BudgetVersionConflict { .. } => StatusCode::CONFLICT,
            Self::BudgetRefused { .. } => StatusCode::BAD_REQUEST,
        }
    }
}
