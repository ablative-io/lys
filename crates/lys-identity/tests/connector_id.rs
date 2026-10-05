//! A connector's id is the word `connector-` and 32 hex digits from the
//! secure random source (DIRECTORY-080 R1).

use std::error::Error;

use lys_identity::grants::RecipientKind;
use lys_identity::{
    Actor, AgentId, AuthMethod, Change, ConnectorId, IdentityError, IdentityEvent, IdentityId,
    LoginBinding, OperationId, PersonId, Profile, Provenance,
};

#[test]
fn a_connector_id_is_connector_and_hex_from_the_secure_random_source() -> Result<(), Box<dyn Error>>
{
    let first = ConnectorId::generate()?;
    let second = ConnectorId::generate()?;
    assert_ne!(first, second);
    let text = first.to_string();
    let hex = text
        .strip_prefix("connector-")
        .ok_or("a connector id begins connector-")?;
    assert_eq!(hex.len(), 32);
    assert!(
        hex.bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
    );
    assert_eq!(text.parse::<ConnectorId>()?, first);
    let identity = IdentityId::Connector(first);
    assert_eq!(identity.to_string(), text);
    assert_eq!(RecipientKind::of(identity), RecipientKind::Connector);
    assert_eq!(RecipientKind::Connector.to_string(), "connector");
    let refused = "op-5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a".parse::<ConnectorId>();
    assert!(
        matches!(
            &refused,
            Err(IdentityError::IdentifierMalformed {
                kind: "connector",
                ..
            })
        ),
        "{refused:?}"
    );
    Ok(())
}

#[test]
fn a_directory_event_never_names_a_connector_as_its_identity_or_a_reporting_target()
-> Result<(), Box<dyn Error>> {
    let connector = IdentityId::Connector(ConnectorId::from_bytes([0x5a; 16]));
    let person = PersonId::from_bytes([0xd1; 16]);
    let actor = Actor::new(
        LoginBinding::new("https://issuer.test", "owner")?,
        Provenance::new(AuthMethod::Oidc, 1),
    );
    let as_identity = IdentityEvent::new(
        OperationId::from_bytes([1; 16]),
        actor.clone(),
        connector,
        1,
        Change::RegisterPerson {
            profile: Profile::new("app")?,
        },
    );
    assert!(
        matches!(
            &as_identity,
            Err(IdentityError::ChangeMismatch { reason })
                if *reason == "a connector is recorded with its app's approval in the apps log"
        ),
        "{as_identity:?}"
    );
    let as_target = IdentityEvent::new(
        OperationId::from_bytes([2; 16]),
        actor,
        IdentityId::Agent(AgentId::from_bytes([0xa7; 16])),
        2,
        Change::ReportingRegistration {
            responsible: person,
            profile: Profile::new("seat")?,
            reports_to: connector,
        },
    );
    assert!(
        matches!(
            &as_target,
            Err(IdentityError::ChangeMismatch { reason })
                if *reason == "a reporting target must be a person or agent"
        ),
        "{as_target:?}"
    );
    Ok(())
}
