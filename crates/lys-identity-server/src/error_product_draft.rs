//! The product drafts' refusals (ACCESS-001 R3): a draft asked on a grant
//! that cannot hold it, words whose digest differs, an approver the draft
//! refuses, another app's connector, and a close that is not the draft's one
//! close of an approved draft.

use axum::http::StatusCode;

/// A refusal of a product draft route.
#[derive(Debug, thiserror::Error)]
pub enum ProductDraftError {
    /// The grant named is not a live held grant of the caller's that reaches
    /// the target: only a right whose mode is held makes a draft.
    #[error(
        "product_draft_grant_refused: {reason} (act: ask Lys for the grant the refusal names, or act at once if the right is outright)"
    )]
    GrantRefused {
        /// Which of the grant's conditions failed.
        reason: String,
    },
    /// The request digest is not SHA-256 of the exact words sent.
    #[error(
        "product_draft_digest_mismatch: request_digest is not the lowercase-hex SHA-256 of the words' exact UTF-8 bytes"
    )]
    DigestMismatch,
    /// The signed-in person may not decide this draft.
    #[error("product_draft_approver_refused: {reason}")]
    ApproverRefused {
        /// Why this person may not decide it.
        reason: &'static str,
    },
    /// A connector asked about or closed another app's drafts.
    #[error(
        "product_draft_not_your_app: the connector of `{caller}` reads and closes only its own app's drafts, not `{app}`'s"
    )]
    NotYourApp {
        /// The app the connector acts for.
        caller: String,
        /// The app it named.
        app: String,
    },
    /// The draft is already closed in other words.
    #[error(
        "product_draft_closed: draft `{draft}` is already closed as its product first recorded; a close is recorded once"
    )]
    Closed {
        /// The draft.
        draft: String,
    },
    /// The draft is not approved, or was refused, so it cannot be closed.
    #[error(
        "product_draft_not_approved: draft `{draft}` is not approved, so its product cannot record it executed or refused"
    )]
    NotApproved {
        /// The draft.
        draft: String,
    },
}

impl ProductDraftError {
    pub(crate) const fn name(&self) -> &'static str {
        match self {
            Self::GrantRefused { .. } => "product_draft_grant_refused",
            Self::DigestMismatch => "product_draft_digest_mismatch",
            Self::ApproverRefused { .. } => "product_draft_approver_refused",
            Self::NotYourApp { .. } => "product_draft_not_your_app",
            Self::Closed { .. } => "product_draft_closed",
            Self::NotApproved { .. } => "product_draft_not_approved",
        }
    }

    pub(crate) const fn status(&self) -> StatusCode {
        match self {
            Self::GrantRefused { .. } | Self::ApproverRefused { .. } | Self::NotYourApp { .. } => {
                StatusCode::FORBIDDEN
            }
            Self::DigestMismatch => StatusCode::BAD_REQUEST,
            Self::Closed { .. } | Self::NotApproved { .. } => StatusCode::CONFLICT,
        }
    }
}
