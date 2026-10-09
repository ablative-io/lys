//! Why the grant change stream or a pass binding was refused (DIRECTORY-089
//! R1, R2); `error_status` answers each through [`GrantStreamError::status`].

use axum::http::StatusCode;

/// A refusal of the grant change stream or of a pass binding.
#[derive(Debug, thiserror::Error)]
pub enum GrantStreamError {
    /// The grant log's identity could not be read or durably recorded; no
    /// replacement is minted for an identity that cannot be read.
    #[error("grant_log_identity_unavailable: {reason}")]
    IdentityUnavailable {
        /// What failed.
        reason: String,
    },
    /// The token request asks for a binding version Lys does not produce.
    #[error(
        "grant_binding_unsupported: binding version {asked} is not produced; Lys produces {served}"
    )]
    BindingUnsupported {
        /// The version asked for.
        asked: String,
        /// The version produced.
        served: u32,
    },
    /// The grants moved while the pass's rights were decided, so no one
    /// coherent revision justifies them; nothing was issued.
    #[error(
        "grant_binding_revision_moved: the rights were decided from revision {decided} and the grants stand at {now}; ask again"
    )]
    BindingRevisionMoved {
        /// The revision the decision began at.
        decided: u64,
        /// The revision the grants stand at now.
        now: u64,
    },
    /// A right was decided from a degraded permission reading, which is never
    /// signed as a current allow; nothing was issued.
    #[error("grant_binding_degraded: a right was decided from a degraded reading: {refusal}")]
    BindingDegraded {
        /// The reading's original refusal name.
        refusal: String,
    },
}

impl GrantStreamError {
    /// The refusal's stable name.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::IdentityUnavailable { .. } => "grant_log_identity_unavailable",
            Self::BindingUnsupported { .. } => "grant_binding_unsupported",
            Self::BindingRevisionMoved { .. } => "grant_binding_revision_moved",
            Self::BindingDegraded { .. } => "grant_binding_degraded",
        }
    }

    /// The status the refusal is answered with.
    #[must_use]
    pub const fn status(&self) -> StatusCode {
        match self {
            Self::IdentityUnavailable { .. } | Self::BindingDegraded { .. } => {
                StatusCode::SERVICE_UNAVAILABLE
            }
            Self::BindingUnsupported { .. } => StatusCode::BAD_REQUEST,
            Self::BindingRevisionMoved { .. } => StatusCode::CONFLICT,
        }
    }
}
