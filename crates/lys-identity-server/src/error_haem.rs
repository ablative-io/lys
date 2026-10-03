//! What the door to a haematite store can answer besides the store's own
//! result: the store is not one this door reaches, the store's service said
//! no, or the store's service did not answer.

use axum::http::StatusCode;

/// A refusal of the door to a haematite store.
#[derive(Debug, thiserror::Error)]
pub enum HaemError {
    /// The configuration names no store by this name.
    #[error("HaemStoreUnknown: this door reaches no haematite store named {store}")]
    StoreUnknown {
        /// The name asked for.
        store: String,
    },
    /// The store's service answered the verb with a refusal of its own. It
    /// is a definite answer: nothing of the request is outstanding.
    #[error("HaemRefused: {code}: {message}")]
    Refused {
        /// The service's own name for its refusal.
        code: String,
        /// The service's own words.
        message: String,
    },
    /// The store's service did not answer: its socket could not be reached,
    /// or it closed or broke its framing before an answer came. Whether a
    /// change that was sent has landed is not known.
    #[error("HaemUnreachable: the service of store {store} did not answer: {reason}")]
    Unreachable {
        /// The store asked for.
        store: String,
        /// What failed.
        reason: String,
    },
}

impl HaemError {
    pub(crate) const fn name(&self) -> &'static str {
        match self {
            Self::StoreUnknown { .. } => "HaemStoreUnknown",
            Self::Refused { .. } => "HaemRefused",
            Self::Unreachable { .. } => "HaemUnreachable",
        }
    }

    pub(crate) const fn status(&self) -> StatusCode {
        match self {
            Self::StoreUnknown { .. } => StatusCode::NOT_FOUND,
            Self::Refused { .. } => StatusCode::CONFLICT,
            Self::Unreachable { .. } => StatusCode::BAD_GATEWAY,
        }
    }
}
