#![cfg(test)]
//! The setup kind's byte vector is stated independently of the encoder and existing kinds.

use std::error::Error;

use lys_identity::encoding::{decode_body, encode_body};
use lys_identity::{
    Actor, AgentId, AuthMethod, Change, IdentityEvent, IdentityId, LoginBinding, OperationId,
    PersonId, Profile, Provenance,
};

type Result = std::result::Result<(), Box<dyn Error>>;

#[test]
fn setup_has_one_frozen_body_and_cannot_name_an_agent() -> Result {
    let actor = Actor::new(
        LoginBinding::new("https://i", "a")?,
        Provenance::new(AuthMethod::Oidc, 1),
    );
    let change = Change::SetupPerson {
        profile: Profile::new("N")?,
    };
    let event = IdentityEvent::new(
        OperationId::from_bytes([7; 16]),
        actor.clone(),
        IdentityId::Person(PersonId::from_bytes([1; 16])),
        2,
        change.clone(),
    )?;
    // Seven body members, version 1, fixed operation, OIDC actor, person, time 2,
    // kind 7 and a one-member profile. RFC 8949 shortest heads, ascending keys.
    let expected = [
        0xa7, 0x01, 0x01, 0x02, 0x50, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 0x03, 0xa4,
        0x01, 0x69, b'h', b't', b't', b'p', b's', b':', b'/', b'/', b'i', 0x02, 0x61, b'a', 0x03,
        0x01, 0x04, 0x01, 0x04, 0xa2, 0x01, 0x01, 0x02, 0x50, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
        1, 1, 1, 1, 0x05, 0x02, 0x06, 0x07, 0x07, 0xa1, 0x01, 0xa1, 0x01, 0x61, b'N',
    ];
    assert_eq!(encode_body(&event), expected);
    assert_eq!(decode_body(&expected)?, event);
    assert!(
        IdentityEvent::new(
            OperationId::from_bytes([8; 16]),
            actor,
            IdentityId::Agent(AgentId::from_bytes([1; 16])),
            2,
            change,
        )
        .is_err()
    );
    Ok(())
}
