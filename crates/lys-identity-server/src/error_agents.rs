//! The agents cluster's refusals: budgets, goals, words, variables, schedules,
//! seats, seat imports and agents' passes to apps. Each owner keeps its own
//! status and name; this enum is one variant of `ServerError` so `error.rs`
//! stays under the length gate (ADR-111) as the cluster grows, and a `?` on
//! any owner's error still lands in `ServerError` through the `From` impls
//! below.

use axum::http::StatusCode;

use crate::error::ServerError;
use crate::error_app_pass::AppPassError;
use crate::error_budget::BudgetError;
use crate::error_seat::SeatError;
use crate::error_seat_import::SeatImportError;
use crate::goals_state::GoalError;
use crate::schedules_state::SchedulesError;
use crate::variables_state::VariablesError;
use crate::words_state::WordsError;

/// One owner's refusal, preserving its status and response words.
#[derive(Debug, thiserror::Error)]
pub enum AgentsError {
    /// A budget or organisation setting refusal.
    #[error(transparent)]
    Budget(#[from] BudgetError),
    /// A goals refusal: an item, its judgement or its reminders.
    #[error(transparent)]
    Goal(#[from] GoalError),
    /// A words refusal (AGENTS-001 R1): a slot, a layer, a template or a stale save.
    #[error(transparent)]
    Words(#[from] WordsError),
    /// A variables refusal (AGENTS-001 R2): a scope, a name or a stale patch.
    #[error(transparent)]
    Variables(#[from] VariablesError),
    /// A schedules refusal (AGENTS-001 R4): a schedule, its change or its stop.
    #[error(transparent)]
    Schedules(#[from] SchedulesError),
    /// A seat's refusal (AGENTS-002).
    #[error(transparent)]
    Seat(#[from] SeatError),
    /// A seat import's refusal (AGENTS-003).
    #[error(transparent)]
    SeatImport(#[from] SeatImportError),
    /// An agent's pass to an app refused (AGENTS-006).
    #[error(transparent)]
    AppPass(#[from] AppPassError),
}

impl AgentsError {
    /// The owner's own status for this refusal.
    pub(crate) fn status(&self) -> StatusCode {
        match self {
            Self::Budget(error) => error.status(),
            Self::Goal(error) => error.status(),
            Self::Words(error) => error.status(),
            Self::Variables(error) => error.status(),
            Self::Schedules(error) => error.status(),
            Self::Seat(error) => error.status(),
            Self::SeatImport(error) => error.status(),
            Self::AppPass(error) => error.status(),
        }
    }
}

macro_rules! lands {
    ($($owner:ty => $variant:ident),* $(,)?) => {$(
        impl From<$owner> for ServerError {
            fn from(error: $owner) -> Self {
                Self::Agents(AgentsError::$variant(error))
            }
        }
    )*};
}

lands! {
    BudgetError => Budget,
    GoalError => Goal,
    WordsError => Words,
    VariablesError => Variables,
    SchedulesError => Schedules,
    SeatError => Seat,
    SeatImportError => SeatImport,
    AppPassError => AppPass,
}
