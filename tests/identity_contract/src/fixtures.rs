//! The actors, profiles and observations the contract tests use.

use lys_identity::{
    Actor, AuthMethod, IdentityError, LinkChange, LinkObservation, LoginBinding, OperationId,
    Profile, Provenance,
};

/// The configured administrator, as the service attests them.
pub fn administrator() -> Result<Actor, IdentityError> {
    Ok(Actor::new(
        LoginBinding::new("https://issuer.test", "administrator")?,
        Provenance::new(AuthMethod::Oidc, 1_790_000_000),
    ))
}

/// The authenticated link-audit source.
pub fn source() -> Result<Actor, IdentityError> {
    Ok(Actor::new(
        LoginBinding::new("https://issuer.test", "link-audit-source")?,
        Provenance::new(AuthMethod::Oidc, 1_790_000_000),
    ))
}

/// A fixed operation id.
pub fn op(n: u8) -> OperationId {
    OperationId::from_bytes([n; 16])
}

/// A profile showing `name`.
pub fn shown(name: &str) -> Result<Profile, IdentityError> {
    Profile::new(name)
}

/// A link the issuer observed under `source_operation_id`.
pub fn linked(source_operation_id: &str) -> Result<LinkObservation, IdentityError> {
    LinkObservation::new(
        source_operation_id,
        LinkChange::Linked,
        LoginBinding::new("https://accounts.test", "ada-elsewhere")?,
        "https://issuer.test",
        1_790_000_050,
    )
}
