//! A grant may name a role (ACCESS-004 R1). A relation is written by the
//! absence of key 15, so every grant written before roles keeps its bytes and
//! its signature; a grant naming a role is key 15, `true`, and signs as event
//! envelope version 5, written only when used.
//!
//! The pre-role bytes are the connector fixture's, written by the code at main
//! 5215ed6c and never regenerated, so they are the old grants this test holds
//! fixed.

use std::collections::BTreeSet;
use std::error::Error;

use ciborium::Value as Cbor;
use lys_identity::grants::events::ROLE_ENVELOPE;
use lys_identity::grants::{
    Action, Grant, GrantChange, GrantError, GrantEvent, GrantId, GrantParts, Mode, PassOn,
    RecipientKind, Relation, Resource, Source, Window, decode_event_body, decode_grant,
    encode_event_body, encode_grant, sign_grant_event, verify_grant_event,
};
use lys_identity::signer::load_service_key;
use lys_identity::{AgentId, IdentityId, OperationId, PersonId};
use serde_json::Value;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE: &str = include_str!("fixtures/connector-5215ed6c.json");
const SERVICE_KEY: [u8; 32] = [0x5e; 32];
const PERSON: [u8; 16] = [0xd1; 16];
const AGENT: [u8; 16] = [0xa7; 16];
const SOURCE: [u8; 16] = [0x51; 16];
const ID: [u8; 16] = [0x61; 16];
const OPERATION: [u8; 16] = [0x0e; 16];

fn unhex(text: &str) -> Result<Vec<u8>, Box<dyn Error>> {
    if text.len() % 2 != 0 {
        return Err(format!("a fixture's hex has an odd length, {}", text.len()).into());
    }
    (0..text.len())
        .step_by(2)
        .map(|at| Ok(u8::from_str_radix(&text[at..at + 2], 16)?))
        .collect()
}

fn fixture_bytes(name: &str) -> Result<Vec<u8>, Box<dyn Error>> {
    let fixture: Value = serde_json::from_str(FIXTURE)?;
    let text = fixture[name]
        .as_str()
        .ok_or_else(|| format!("the fixture has no {name}"))?;
    unhex(text)
}

fn actions(names: &[&str]) -> Result<BTreeSet<Action>, GrantError> {
    names.iter().map(|name| Action::new(name)).collect()
}

/// The fixture's grant from the person to the agent, naming a relation.
fn to_agent() -> Result<Grant, GrantError> {
    Grant::new(GrantParts {
        id: GrantId::from_bytes(ID),
        issuer: IdentityId::Person(PersonId::from_bytes(PERSON)),
        holder: IdentityId::Agent(AgentId::from_bytes(AGENT)),
        responsible: PersonId::from_bytes(PERSON),
        resource: Resource::new("project", "alpha")?,
        relation: Relation::new("heron")?,
        actions: actions(&["read", "write"])?,
        pass_on: PassOn::to(
            actions(&["read"])?,
            [RecipientKind::Person, RecipientKind::Agent]
                .into_iter()
                .collect(),
        )?,
        source: Source::Grant(GrantId::from_bytes(SOURCE)),
        window: Window::new(1_000, Some(2_000))?,
        model_version: 3,
        operation: OperationId::from_bytes(OPERATION),
    })
}

fn issued(grant: Grant) -> Result<GrantEvent, GrantError> {
    GrantEvent::new(
        OperationId::from_bytes(OPERATION),
        IdentityId::Person(PersonId::from_bytes(PERSON)),
        1_500,
        GrantChange::Issue(Box::new(grant)),
    )
}

fn signed(event: GrantEvent) -> Result<(Vec<u8>, [u8; 32]), Box<dyn Error>> {
    let temp = tempfile::tempdir()?;
    let path = temp.path().join("service.key");
    std::fs::write(&path, SERVICE_KEY)?;
    let key = load_service_key(&path)?;
    let message = sign_grant_event(event, &key)?.bytes().to_vec();
    Ok((message, key.public_key_bytes()))
}

/// The keys of an encoded grant body, in order.
fn keys(encoded: &[u8]) -> Result<Vec<u64>, Box<dyn Error>> {
    let Cbor::Map(pairs) = ciborium::from_reader::<Cbor, _>(encoded)? else {
        return Err("a grant body is a map".into());
    };
    pairs
        .into_iter()
        .map(|(key, _)| {
            key.as_integer()
                .and_then(|key| u64::try_from(key).ok())
                .ok_or_else(|| "a grant's keys are unsigned integers".into())
        })
        .collect()
}

#[test]
fn a_grant_written_before_roles_names_no_role_and_keeps_its_bytes() -> TestResult {
    let old = fixture_bytes("grant_body_to_agent")?;
    let grant = decode_grant(&old)?;
    assert!(!grant.names_role());
    assert_eq!(
        encode_grant(&grant),
        old,
        "a relation is written by absence"
    );
    assert!(!keys(&old)?.contains(&15));
    Ok(())
}

#[test]
fn an_event_signed_before_roles_still_verifies_at_its_own_version() -> TestResult {
    let old = fixture_bytes("event_v1_person_to_agent")?;
    let (_, public) = signed(issued(to_agent()?)?)?;
    let read = verify_grant_event(&old, &public)?;
    assert_eq!(read.event().version(), 1);
    assert_eq!(read.bytes(), old.as_slice());
    Ok(())
}

#[test]
fn a_grant_naming_a_role_is_key_fifteen_and_round_trips() -> TestResult {
    let grant = to_agent()?.as_role();
    let encoded = encode_grant(&grant);
    assert_eq!(keys(&encoded)?.last(), Some(&15));
    let read = decode_grant(&encoded)?;
    assert!(read.names_role());
    assert_eq!(read, grant);
    let both = to_agent()?.with_mode(Mode::ByTwo).as_role();
    let encoded = encode_grant(&both);
    assert_eq!(
        keys(&encoded)?[12..],
        [14, 15],
        "the mode then the role, in key order"
    );
    assert_eq!(decode_grant(&encoded)?, both);
    Ok(())
}

#[test]
fn a_grant_naming_a_role_signs_as_version_five_and_verifies() -> TestResult {
    for grant in [
        to_agent()?.as_role(),
        to_agent()?.with_mode(Mode::ByDraft).as_role(),
    ] {
        let event = issued(grant)?;
        assert_eq!(event.version(), 5, "a role is envelope version 5");
        let (message, public) = signed(event.clone())?;
        let read = verify_grant_event(&message, &public)?;
        assert_eq!(read.event(), &event);
        assert!(
            message
                .windows(ROLE_ENVELOPE.len())
                .any(|window| window == ROLE_ENVELOPE.as_bytes()),
            "the protected header names the version 5 envelope"
        );
        assert_eq!(decode_event_body(&encode_event_body(&event))?, event);
    }
    Ok(())
}

#[test]
fn a_grant_naming_a_relation_never_writes_key_fifteen() -> TestResult {
    let grant = to_agent()?;
    assert!(!keys(&encode_grant(&grant))?.contains(&15));
    assert_eq!(issued(grant)?.version(), 1);
    let held = to_agent()?.with_mode(Mode::ByDraft);
    assert_eq!(
        issued(held)?.version(),
        4,
        "a held mode alone stays version 4"
    );
    Ok(())
}

#[test]
fn key_fifteen_written_as_anything_but_true_is_refused_by_name() -> TestResult {
    let mut encoded = encode_grant(&to_agent()?.as_role());
    let last = encoded.len() - 1;
    // 0xf4 is CBOR false and 0x01 the integer 1.
    for byte in [0xf4_u8, 0x01] {
        encoded[last] = byte;
        let refused = decode_grant(&encoded)
            .err()
            .ok_or("a bad role mark was read")?;
        assert!(
            matches!(refused, GrantError::GrantMalformed { .. }),
            "byte {byte:#x}: {refused:?}"
        );
    }
    Ok(())
}
