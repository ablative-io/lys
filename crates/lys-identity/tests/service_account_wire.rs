//! Old event bytes keep v1; new principals require the v2 signed domain.
use lys_identity::encoding::{decode_body, encode_body};
use lys_identity::event::{Change, IdentityEvent};
use lys_identity::signer::{load_service_key, sign_event, verify_event};
use lys_identity::{
    Actor, AgentId, AuthMethod, IdentityId, LoginBinding, OperationId, PersonId, Profile,
    Provenance, ServiceAccountId,
};
use std::error::Error;

#[test]
fn provenance_is_service_account_and_cannot_be_downgraded_to_v1() -> Result<(), Box<dyn Error>> {
    let account = ServiceAccountId::from_bytes([71; 16]);
    let event = IdentityEvent::new(
        OperationId::from_bytes([72; 16]),
        Actor::new(
            LoginBinding::new("https://issuer.test", "admin")?,
            Provenance::new(AuthMethod::ServiceAccountBearer(account), 10),
        ),
        IdentityId::Agent(AgentId::from_bytes([73; 16])),
        11,
        Change::RegisterAgent {
            responsible: PersonId::from_bytes([74; 16]),
            profile: Profile::new("fixture")?,
        },
    )?;
    assert_eq!(event.version(), 2);
    assert_eq!(event.actor().provenance().service_account(), Some(account));
    assert_eq!(event.actor().provenance().agent(), None);
    let body = encode_body(&event);
    assert_eq!(decode_body(&body)?, event);
    let mut downgraded: ciborium::Value = ciborium::from_reader(body.as_slice())?;
    let ciborium::Value::Map(fields) = &mut downgraded else {
        return Err("not a map".into());
    };
    let version = fields
        .iter_mut()
        .find(|(key, _)| *key == ciborium::Value::Integer(1.into()))
        .ok_or("no version")?;
    version.1 = ciborium::Value::Integer(1.into());
    let mut bytes = Vec::new();
    ciborium::into_writer(&downgraded, &mut bytes)?;
    assert!(decode_body(&bytes).is_err());
    let temp = tempfile::tempdir()?;
    let key_path = temp.path().join("key");
    std::fs::write(&key_path, [7; 32])?;
    let key = load_service_key(&key_path)?;
    let signed = sign_event(event.clone(), &key)?;
    assert_eq!(
        verify_event(signed.bytes(), &key.public_key_bytes())?.event(),
        &event
    );
    let mut altered = signed.bytes().to_vec();
    let index = altered
        .windows(7)
        .position(|part| part == b"v2+cbor")
        .ok_or("no v2 envelope")?;
    altered[index + 1] = b'1';
    assert!(verify_event(&altered, &key.public_key_bytes()).is_err());
    Ok(())
}
