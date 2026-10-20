//! The build before organisations refuses a log brought under one
//! (DIRECTORY-109 R2): the install is one-way, and says so.

use super::{decode_body, decode_body_before_organisations, encode_body};
use crate::{
    Actor, AuthMethod, Change, IdentityError, IdentityEvent, IdentityId, LoginBinding, OperationId,
    OrganisationId, PersonId, Profile, Provenance,
};

fn event(identity: IdentityId, change: Change) -> Result<IdentityEvent, IdentityError> {
    IdentityEvent::new(
        OperationId::from_bytes([3; 16]),
        Actor::new(
            LoginBinding::new("https://issuer.test", "administrator")?,
            Provenance::new(AuthMethod::Oidc, 1),
        ),
        identity,
        1,
        change,
    )
}

fn refused(change: Change, identity: IdentityId) -> Result<String, IdentityError> {
    let body = encode_body(&event(identity, change)?);
    decode_body(&body)?;
    match decode_body_before_organisations(&body) {
        Err(IdentityError::EventMalformed { reason }) => Ok(reason.to_owned()),
        Err(other) => Err(other),
        Ok(_) => Ok("read".to_owned()),
    }
}

#[test]
fn a_founding_reads_to_this_build_and_not_to_the_one_before() -> Result<(), IdentityError> {
    let organisation = OrganisationId::from_bytes([5; 16]);
    let founder = PersonId::from_bytes([6; 16]);
    assert_eq!(
        refused(
            Change::FoundOrganisation {
                profile: Profile::new("Example organisation")?,
                founder,
            },
            IdentityId::Organisation(organisation),
        )?,
        "a change kind is 1 to 10"
    );
    assert_eq!(
        refused(
            Change::AttachOrganisation { organisation },
            IdentityId::Person(founder),
        )?,
        "a change kind is 1 to 10"
    );
    assert_eq!(
        refused(
            Change::RegisterPerson {
                profile: Profile::new("Bea")?,
                organisation: Some(organisation),
            },
            IdentityId::Person(founder),
        )?,
        "a person registration is a map of key 1"
    );
    assert_eq!(
        refused(
            Change::RegisterPerson {
                profile: Profile::new("Bea")?,
                organisation: None,
            },
            IdentityId::Person(founder),
        )?,
        "read",
        "a registration written before organisations reads to both builds"
    );
    Ok(())
}
