//! Refusals of a provider revocation's confirmation.

/// Why a provider revocation could not be confirmed.
#[derive(Debug, thiserror::Error)]
pub enum RevocationRefusal {
    /// The provider was to be asked before use stopped here.
    #[error(
        "RevocationBeforeDrop: handle {handle} still admits uses here (act: drop the handle first; the provider is asked after)"
    )]
    BeforeDrop {
        /// The handle.
        handle: String,
    },
    /// No provider revocation of the handle waits to be confirmed.
    #[error(
        "RevocationNotPending: handle {handle} has no unconfirmed provider revocation (act: drop the handle with --revoke-upstream first)"
    )]
    NotPending {
        /// The handle.
        handle: String,
    },
    /// The provider's answer is for another grant.
    #[error(
        "ProviderMismatch: the answer for handle {handle} names {answered}, not the grant's provider subject (act: confirm with the provider's answer for this grant)"
    )]
    ProviderMismatch {
        /// The handle.
        handle: String,
        /// The subject the answer named.
        answered: String,
    },
}

impl RevocationRefusal {
    /// The refusal's name.
    pub fn name(&self) -> &'static str {
        match self {
            Self::BeforeDrop { .. } => "RevocationBeforeDrop",
            Self::NotPending { .. } => "RevocationNotPending",
            Self::ProviderMismatch { .. } => "ProviderMismatch",
        }
    }
}
