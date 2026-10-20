//! The actors, profiles and observations the contract tests use.

use lys_identity::{
    Actor, AuthMethod, IdentityError, LinkChange, LinkObservation, LoginBinding, OperationId,
    Profile, Provenance,
};

/// The configured administrator, as the service attests them.
///
/// # Errors
/// Returns the underlying named setup or API refusal.
pub fn administrator() -> Result<Actor, IdentityError> {
    Ok(Actor::new(
        LoginBinding::new("https://issuer.test", "administrator")?,
        Provenance::new(AuthMethod::Oidc, 1_790_000_000),
    ))
}

/// The authenticated link-audit source.
///
/// # Errors
/// Returns the underlying named setup or API refusal.
pub fn source() -> Result<Actor, IdentityError> {
    Ok(Actor::new(
        LoginBinding::new("https://issuer.test", "link-audit-source")?,
        Provenance::new(AuthMethod::Oidc, 1_790_000_000),
    ))
}

/// A fixed operation id.
#[must_use]
pub fn op(n: u8) -> OperationId {
    OperationId::from_bytes([n; 16])
}

/// A profile showing `name`.
///
/// # Errors
/// Returns the underlying named setup or API refusal.
pub fn shown(name: &str) -> Result<Profile, IdentityError> {
    Profile::new(name)
}

/// The organisation the fixtures found: a label, like every organisation's name.
///
/// # Errors
/// Returns the underlying named setup or API refusal.
pub fn organisation() -> Result<Profile, IdentityError> {
    Profile::new("Example organisation")
}

/// The organisation `person` was registered under (DIRECTORY-109 R1).
///
/// # Errors
/// Refuses a directory that cannot be read or a person under no organisation.
pub fn organisation_of<S: lys_log_store::LeafStore>(
    directory: &mut lys_identity::Directory<S>,
    person: lys_identity::PersonId,
) -> Result<lys_identity::OrganisationId, Box<dyn std::error::Error>> {
    Ok(directory
        .projection()?
        .organisation_of(lys_identity::IdentityId::Person(person))
        .ok_or("the person belongs to no organisation")?)
}

/// The one organisation `directory` holds, for a fixture that registers
/// people into a seeded directory.
///
/// # Errors
/// Refuses a directory that cannot be read, or one holding no organisation
/// or more than one.
pub fn the_organisation<S: lys_log_store::LeafStore>(
    directory: &mut lys_identity::Directory<S>,
) -> Result<lys_identity::OrganisationId, Box<dyn std::error::Error>> {
    let mut held = directory.projection()?.organisations().map(|(id, _)| id);
    let one = held.ok_or("the directory holds no organisation")?;
    if held.next().is_some() {
        return Err("the directory holds more than one organisation".into());
    }
    Ok(one)
}

/// A link the issuer observed under `source_operation_id`.
///
/// # Errors
/// Returns the underlying named setup or API refusal.
pub fn linked(source_operation_id: &str) -> Result<LinkObservation, IdentityError> {
    LinkObservation::new(
        source_operation_id,
        LinkChange::Linked,
        LoginBinding::new("https://accounts.test", "ada-elsewhere")?,
        "https://issuer.test",
        1_790_000_050,
    )
}
