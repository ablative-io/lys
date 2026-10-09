//! The machine is the fifth identity kind (ACCESS-005 R1): code 5 wherever a
//! kind is numbered, `machine` in the permission schema, and an event naming
//! one signs as grant event version 6 under its own envelope. Every byte
//! written before it stays as it was: the pinned grant bodies and signed
//! events of `connector_compatibility.rs` and `grant_mode_compatibility.rs`
//! are read and written here again, unchanged. Each machine body is the
//! pinned service-account grant body with only its kind codes changed.

use std::error::Error;

use lys_identity::grants::permission::{ObjectRef, relationships_of};
use lys_identity::grants::{
    GrantChange, GrantError, GrantEvent, PassOn, RecipientKind, SCHEMA, decode_event_body,
    decode_grant, encode_event_body, encode_grant, sign_grant_event, verify_grant_event,
};
use lys_identity::{IdentityError, IdentityId, MachineId, OperationId, PersonId};
use serde_json::Value;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE: &str = include_str!("fixtures/connector-5215ed6c.json");
const FIXTURE_V3: &str = include_str!("fixtures/connector-v3.json");
const SERVICE_KEY: [u8; 32] = [0x5e; 32];
/// The service account's identity map as the pinned body holds it.
const ACCOUNT_HOLDER: &str = "03a2010302505a";
/// The pinned body's recipients: a person, an agent and a service account.
const ACCOUNT_RECIPIENTS: &str = "0283010203";
const MACHINE_ENVELOPE: &[u8] = b"application/vnd.lys.grant-event.v6+cbor";

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(char::from(DIGITS[usize::from(byte >> 4)]));
        out.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    out
}

fn unhex(text: &str) -> Result<Vec<u8>, Box<dyn Error>> {
    if text.len() % 2 != 0 {
        return Err(format!("a fixture's hex has an odd length, {}", text.len()).into());
    }
    (0..text.len())
        .step_by(2)
        .map(|at| Ok(u8::from_str_radix(&text[at..at + 2], 16)?))
        .collect()
}

fn pinned(fixture: &str, name: &str) -> Result<Vec<u8>, Box<dyn Error>> {
    let fixture: Value = serde_json::from_str(fixture)?;
    let text = fixture[name]
        .as_str()
        .ok_or_else(|| format!("the fixture has no {name}"))?;
    unhex(text)
}

/// The pinned service-account body with its holder's kind code and an added
/// recipient kind code set to `code`.
fn body_with_kind(code: u8) -> Result<Vec<u8>, Box<dyn Error>> {
    let fixture: Value = serde_json::from_str(FIXTURE)?;
    let body = fixture["grant_body_to_service_account"]
        .as_str()
        .ok_or("the fixture has no grant_body_to_service_account")?;
    assert_eq!(body.matches(ACCOUNT_HOLDER).count(), 1);
    assert_eq!(body.matches(ACCOUNT_RECIPIENTS).count(), 1);
    let text = body
        .replacen(ACCOUNT_HOLDER, &format!("03a201{code:02x}02505a"), 1)
        .replacen(ACCOUNT_RECIPIENTS, &format!("0284010203{code:02x}"), 1);
    unhex(&text)
}

fn service_key() -> Result<(tempfile::TempDir, lys_core::Ed25519Identity), Box<dyn Error>> {
    let temp = tempfile::tempdir()?;
    let path = temp.path().join("service.key");
    std::fs::write(&path, SERVICE_KEY)?;
    let key = lys_identity::signer::load_service_key(&path)?;
    Ok((temp, key))
}

#[test]
fn a_machine_id_is_machine_and_hex_from_the_secure_random_source() -> TestResult {
    let first = MachineId::generate()?;
    let second = MachineId::generate()?;
    assert_ne!(first, second);
    let text = first.to_string();
    let digits = text
        .strip_prefix("machine-")
        .ok_or("a machine id begins machine-")?;
    assert_eq!(digits.len(), 32);
    assert!(
        digits
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
    );
    assert_eq!(text.parse::<MachineId>()?, first);
    assert_eq!(MachineId::from_bytes(*first.as_bytes()), first);
    let identity = IdentityId::Machine(first);
    assert_eq!(identity.to_string(), text);
    assert_eq!(RecipientKind::of(identity), RecipientKind::Machine);
    assert_eq!(RecipientKind::Machine.to_string(), "machine");
    let refused = "connector-5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a".parse::<MachineId>();
    assert!(
        matches!(
            &refused,
            Err(IdentityError::IdentifierMalformed {
                kind: "machine",
                ..
            })
        ),
        "{refused:?}"
    );
    Ok(())
}

#[test]
fn every_grant_and_event_written_before_the_machine_keeps_its_bytes() -> TestResult {
    let (_temp, key) = service_key()?;
    let public = key.public_key_bytes();
    for name in ["grant_body_to_agent", "grant_body_to_service_account"] {
        let old = pinned(FIXTURE, name)?;
        assert_eq!(encode_grant(&decode_grant(&old)?), old, "{name}");
    }
    let old = pinned(FIXTURE_V3, "grant_body_to_connector")?;
    assert_eq!(
        encode_grant(&decode_grant(&old)?),
        old,
        "the connector body"
    );
    for (fixture, name, version) in [
        (FIXTURE, "event_v1_person_to_agent", 1),
        (FIXTURE, "event_v2_person_to_service_account", 2),
        (FIXTURE_V3, "event_v3_person_to_connector", 3),
    ] {
        let old = pinned(fixture, name)?;
        let read = verify_grant_event(&old, &public)?;
        assert_eq!(read.event().version(), version, "{name}");
        assert_eq!(read.bytes(), old.as_slice(), "{name}");
        let again = sign_grant_event(read.event().clone(), &key)?;
        assert_eq!(again.bytes(), old.as_slice(), "{name}: re-signed the same");
    }
    Ok(())
}

#[test]
fn a_grant_held_by_and_passable_to_a_machine_is_kind_five_both_ways() -> TestResult {
    let bytes = body_with_kind(5)?;
    let grant = decode_grant(&bytes)?;
    assert_eq!(
        grant.holder().to_string(),
        "machine-5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a"
    );
    assert!(matches!(grant.holder(), IdentityId::Machine(_)));
    let PassOn::To { recipients, .. } = grant.pass_on() else {
        return Err("the body passes on to four kinds".into());
    };
    let named: Vec<String> = recipients.iter().map(ToString::to_string).collect();
    assert_eq!(named, ["person", "agent", "service_account", "machine"]);
    assert_eq!(encode_grant(&grant), bytes, "today's writer writes code 5");
    Ok(())
}

#[test]
fn a_grant_naming_a_sixth_kind_is_refused_with_the_five_kinds_named() -> TestResult {
    let refused = decode_grant(&body_with_kind(6)?);
    assert!(
        matches!(
            &refused,
            Err(GrantError::GrantMalformed { reason }) if *reason
                == "an identity kind code is 1 for a person, 2 for an agent, 3 for a service account, 4 for a connector or 5 for a machine"
        ),
        "{refused:?}"
    );
    Ok(())
}

#[test]
fn a_grant_event_naming_a_machine_signs_as_version_six_and_reads_back() -> TestResult {
    let grant = decode_grant(&body_with_kind(5)?)?;
    let event = GrantEvent::new(
        OperationId::from_bytes([0x0e; 16]),
        IdentityId::Person(PersonId::from_bytes([0xd1; 16])),
        1_500,
        GrantChange::Issue(Box::new(grant)),
    )?;
    assert_eq!(event.version(), 6, "a machine is envelope version 6");
    let (_temp, key) = service_key()?;
    let signed = sign_grant_event(event.clone(), &key)?;
    assert!(
        signed
            .bytes()
            .windows(MACHINE_ENVELOPE.len())
            .any(|window| window == MACHINE_ENVELOPE),
        "the protected header names the version 6 content type"
    );
    let read = verify_grant_event(signed.bytes(), &key.public_key_bytes())?;
    assert_eq!(read.event(), &event);
    assert_eq!(decode_event_body(&encode_event_body(&event))?, event);
    Ok(())
}

#[test]
fn an_event_that_names_no_machine_keeps_its_older_version() -> TestResult {
    let grant = decode_grant(&pinned(FIXTURE, "grant_body_to_service_account")?)?;
    let event = GrantEvent::new(
        OperationId::from_bytes([0x0e; 16]),
        IdentityId::Person(PersonId::from_bytes([0xd1; 16])),
        1_500,
        GrantChange::Issue(Box::new(grant)),
    )?;
    assert_eq!(event.version(), 2);
    let (_temp, key) = service_key()?;
    let signed = sign_grant_event(event, &key)?;
    assert!(
        !signed
            .bytes()
            .windows(MACHINE_ENVELOPE.len())
            .any(|window| window == MACHINE_ENVELOPE)
    );
    Ok(())
}

#[test]
fn a_version_six_body_under_an_older_envelope_is_refused() -> TestResult {
    let grant = decode_grant(&body_with_kind(5)?)?;
    let event = GrantEvent::new(
        OperationId::from_bytes([0x0e; 16]),
        IdentityId::Person(PersonId::from_bytes([0xd1; 16])),
        1_500,
        GrantChange::Issue(Box::new(grant)),
    )?;
    let (_temp, key) = service_key()?;
    let signed = sign_grant_event(event, &key)?;
    // The same message with its content type replaced by version 3's, which
    // has the same length: a machine is never read under an older envelope.
    let older = b"application/vnd.lys.grant-event.v3+cbor";
    assert_eq!(older.len(), MACHINE_ENVELOPE.len());
    let mut bytes = signed.bytes().to_vec();
    let at = bytes
        .windows(MACHINE_ENVELOPE.len())
        .position(|window| window == MACHINE_ENVELOPE)
        .ok_or("the message names its envelope")?;
    bytes[at..at + older.len()].copy_from_slice(older);
    let refused = verify_grant_event(&bytes, &key.public_key_bytes());
    assert!(refused.is_err(), "{refused:?}");
    Ok(())
}

#[test]
fn a_machine_holding_a_grant_is_a_machine_subject_of_the_holder_relation() -> TestResult {
    let grant = decode_grant(&body_with_kind(5)?)?;
    let holder = relationships_of(&grant)
        .into_iter()
        .find(|relationship| relationship.relation == "holder")
        .ok_or("a grant is written with its holder")?;
    assert_eq!(holder.subject.kind, "machine");
    assert_eq!(
        holder.subject.id,
        "machine-5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a"
    );
    assert_eq!(
        ObjectRef::identity(IdentityId::Machine(MachineId::from_bytes([1; 16]))).kind,
        "machine"
    );
    assert!(SCHEMA.contains("\ndefinition machine {}\n"));
    assert!(SCHEMA.contains("| machine | machine with unexpired |"));
    Ok(())
}

#[test]
fn the_pinned_bodies_name_no_machine() -> TestResult {
    // A guard on the fixtures themselves: no pinned old byte carries code 5,
    // so every read above is of bytes written before the machine existed.
    let fixture: Value = serde_json::from_str(FIXTURE)?;
    let body = fixture["grant_body_to_service_account"]
        .as_str()
        .ok_or("the fixture has no grant_body_to_service_account")?;
    assert!(!body.contains("03a2010502"));
    assert_eq!(hex(&unhex(body)?), body);
    Ok(())
}
