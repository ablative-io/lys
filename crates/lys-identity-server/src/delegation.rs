//! Pure preparation for delegated reads. No routes, token minting, durable
//! encoding, store writes or runtime authorization are enabled by this module.
//! Callers must eventually supply authenticated, freshly settled facts and
//! atomically append a reviewed signed record before acting on a new plan.

mod binding;
mod consent;
mod nonce;

pub use binding::{Binding, ClientSource, authenticate};
pub use consent::{
    Choice, Current, DecisionCommand, DecisionPlan, DecisionRecord, Operation, Owner, Request,
    RevocationCommand, RevocationPlan, RevocationRecord, plan_decision, plan_revocation, standing,
};
pub use nonce::ActionNonce;

/// Named, input-independent refusals; no credential or submitted value is echoed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Refusal {
    /// The request did not present exactly one client authentication method.
    #[error("DelegationClientAuthenticationAmbiguous")]
    ClientAuthentication,
    /// Configured OpenID clients do not confer approved-app delegation authority.
    #[error("DelegationClientAuthorityRefused")]
    ClientAuthority,
    /// No current approved app with valid authority coordinates was supplied.
    #[error("DelegationBindingInvalid")]
    BindingInvalid,
    /// A current authority coordinate differs from the consent's binding.
    #[error("DelegationBindingChanged")]
    BindingChanged,
    /// Client authentication or redirect matching failed.
    #[error("DelegationClientRefused")]
    ClientRefused,
    /// A candidate consent or operation record is malformed.
    #[error("DelegationRecordInvalid")]
    RecordInvalid,
    /// The original or current owner is not authenticated and active.
    #[error("DelegationOwnerRefused")]
    OwnerRefused,
    /// A request, session or action nonce has expired or is not yet valid.
    #[error("DelegationExpired")]
    Expired,
    /// A one-use action proof is absent or belongs to another action/request.
    #[error("DelegationNonceRefused")]
    NonceRefused,
    /// An existing operation or consent cannot be replaced by different words.
    #[error("DelegationOperationReused")]
    OperationReused,
    /// The historical consent was declined or has since been withdrawn.
    #[error("DelegationNotStanding")]
    NotStanding,
}

fn exact(text: &str) -> bool {
    !text.is_empty() && text.trim() == text && !text.chars().any(char::is_control)
}

fn same(left: &[u8], right: &[u8]) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .fold(0_u8, |differ, (a, b)| differ | (a ^ b))
            == 0
}

#[cfg(test)]
#[path = "delegation/test_fixture.rs"]
mod fixture;
