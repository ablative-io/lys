//! The connector is the fourth identity kind (DIRECTORY-080 R1): code 4
//! wherever a kind is numbered, and named by every reader that meets a kind
//! it does not take. Each body here is the pinned service-account grant body
//! of `connector_compatibility.rs` with only its kind codes changed.

use std::error::Error;

use lys_identity::grants::permission::relationships_of;
use lys_identity::grants::{
    GrantChange, GrantError, GrantEvent, SCHEMA, decode_event_body, decode_grant,
    encode_event_body, encode_grant, sign_grant_event, verify_grant_event,
};
use lys_identity::message_service::parse_identity;
use lys_identity::{IdentityError, IdentityId, OperationId, PersonId};
use serde_json::Value;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE: &str = include_str!("fixtures/connector-5215ed6c.json");
/// The service account's identity map as the pinned body holds it.
const ACCOUNT_HOLDER: &str = "03a2010302505a";
/// The pinned body's recipients: a person, an agent and a service account.
const ACCOUNT_RECIPIENTS: &str = "0283010203";

fn unhex(text: &str) -> Result<Vec<u8>, Box<dyn Error>> {
    (0..text.len())
        .step_by(2)
        .map(|at| Ok(u8::from_str_radix(&text[at..at + 2], 16)?))
        .collect()
}

/// The pinned service-account body with its holder's kind code and an added
/// recipient kind code set to `code`.
fn body_with_kind(code: u8) -> Result<Vec<u8>, Box<dyn Error>> {
    let fixture: Value = serde_json::from_str(FIXTURE)?;
    let pinned = fixture["grant_body_to_service_account"]
        .as_str()
        .ok_or("the fixture has no grant_body_to_service_account")?;
    assert_eq!(pinned.matches(ACCOUNT_HOLDER).count(), 1);
    assert_eq!(pinned.matches(ACCOUNT_RECIPIENTS).count(), 1);
    let text = pinned
        .replacen(ACCOUNT_HOLDER, &format!("03a201{code:02x}02505a"), 1)
        .replacen(ACCOUNT_RECIPIENTS, &format!("0284010203{code:02x}"), 1);
    unhex(&text)
}

#[test]
fn a_grant_held_by_and_passable_to_a_connector_is_kind_four_both_ways() -> TestResult {
    let bytes = body_with_kind(4)?;
    let grant = decode_grant(&bytes)?;
    assert_eq!(
        grant.holder().to_string(),
        "connector-5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a"
    );
    let lys_identity::grants::PassOn::To { recipients, .. } = grant.pass_on() else {
        return Err("the body passes on to four kinds".into());
    };
    let named: Vec<String> = recipients.iter().map(ToString::to_string).collect();
    assert_eq!(named, ["person", "agent", "service_account", "connector"]);
    assert_eq!(encode_grant(&grant), bytes, "today's writer writes code 4");
    Ok(())
}

#[test]
fn a_grant_naming_a_fifth_kind_is_refused_with_the_four_kinds_named() -> TestResult {
    let refused = decode_grant(&body_with_kind(5)?);
    assert!(
        matches!(
            &refused,
            Err(GrantError::GrantMalformed { reason }) if *reason
                == "an identity kind code is 1 for a person, 2 for an agent, 3 for a service account or 4 for a connector"
        ),
        "{refused:?}"
    );
    Ok(())
}

#[test]
fn a_grant_event_naming_a_connector_is_signed_as_version_three_and_read_back() -> TestResult {
    let grant = decode_grant(&body_with_kind(4)?)?;
    let event = GrantEvent::new(
        OperationId::from_bytes([0x0e; 16]),
        IdentityId::Person(PersonId::from_bytes([0xd1; 16])),
        1_500,
        GrantChange::Issue(Box::new(grant)),
    )?;
    assert_eq!(event.version(), 3);
    let temp = tempfile::tempdir()?;
    let path = temp.path().join("service.key");
    std::fs::write(&path, [0x5e; 32])?;
    let key = lys_identity::signer::load_service_key(&path)?;
    let signed = sign_grant_event(event.clone(), &key)?;
    let envelope = b"application/vnd.lys.grant-event.v3+cbor";
    assert!(
        signed
            .bytes()
            .windows(envelope.len())
            .any(|window| window == envelope),
        "the protected header names the version 3 content type"
    );
    let read = verify_grant_event(signed.bytes(), &key.public_key_bytes())?;
    assert_eq!(read.event(), &event);
    assert_eq!(decode_event_body(&encode_event_body(&event))?, event);
    Ok(())
}

#[test]
fn a_connector_holding_a_grant_is_a_connector_subject_of_the_holder_relation() -> TestResult {
    let grant = decode_grant(&body_with_kind(4)?)?;
    let holder = relationships_of(&grant)
        .into_iter()
        .find(|relationship| relationship.relation == "holder")
        .ok_or("a grant is written with its holder")?;
    assert_eq!(holder.subject.kind, "connector");
    assert_eq!(
        holder.subject.id,
        "connector-5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a"
    );
    assert!(SCHEMA.contains("\ndefinition connector {}\n"));
    assert!(SCHEMA.contains("| connector | connector with unexpired\n"));
    Ok(())
}

#[test]
fn a_message_service_binding_names_a_person_or_an_agent_and_refuses_any_other_kind() {
    for text in [
        "connector-5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a",
        "op-5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a",
        "team-5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a",
    ] {
        let refused = parse_identity(text);
        assert!(
            matches!(
                &refused,
                Err(IdentityError::IdentifierMalformed { kind, text: named })
                    if *kind == "person or agent" && named == text
            ),
            "{text}: {refused:?}"
        );
    }
    assert!(matches!(
        parse_identity("agent-5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a"),
        Ok(IdentityId::Agent(_))
    ));
}
