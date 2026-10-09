//! A grant carries one mode: outright, by draft or by two approvals
//! (ACCESS-001 R1). Outright is written by its absence, so every grant written
//! before the mode existed keeps its bytes and its signature; a held mode is
//! key 14 of the grant body and signs as event envelope version 4.
//!
//! The pre-mode bytes are the connector fixture's, written by the code at main
//! 5215ed6c and never regenerated: they are the bytes a grant had before this
//! brief, so they are the old grants this test holds fixed.

use std::collections::BTreeSet;
use std::error::Error;

use ciborium::Value as Cbor;
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

/// The fixture's grant from the person to the agent, with no mode named.
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
fn a_grant_written_before_the_mode_reads_as_outright_and_keeps_its_bytes() -> TestResult {
    let old = fixture_bytes("grant_body_to_agent")?;
    let grant = decode_grant(&old)?;
    assert_eq!(grant.mode(), Mode::Outright);
    assert_eq!(
        encode_grant(&grant),
        old,
        "outright is written by its absence"
    );
    assert!(!keys(&old)?.contains(&14));
    Ok(())
}

#[test]
fn an_event_signed_before_the_mode_still_verifies_and_reads_as_outright() -> TestResult {
    let old = fixture_bytes("event_v1_person_to_agent")?;
    let (_, public) = signed(issued(to_agent()?)?)?;
    let read = verify_grant_event(&old, &public)?;
    assert_eq!(read.event().version(), 1);
    let GrantChange::Issue(grant) = read.event().change() else {
        return Err("the fixture's event issues a grant".into());
    };
    assert_eq!(grant.mode(), Mode::Outright);
    Ok(())
}

#[test]
fn a_held_mode_is_key_fourteen_and_round_trips() -> TestResult {
    for (mode, code) in [(Mode::ByDraft, 1_u64), (Mode::ByTwo, 2)] {
        let grant = to_agent()?.with_mode(mode);
        assert_eq!(grant.mode(), mode);
        let encoded = encode_grant(&grant);
        assert_eq!(
            keys(&encoded)?.last(),
            Some(&14),
            "{mode:?}: key 14 is last"
        );
        let read = decode_grant(&encoded)?;
        assert_eq!(read.mode(), mode, "{mode:?}: read back");
        assert_eq!(read, grant);
        let Cbor::Map(pairs) = ciborium::from_reader::<Cbor, _>(encoded.as_slice())? else {
            return Err("a grant body is a map".into());
        };
        let written = pairs
            .iter()
            .find(|(key, _)| key.as_integer() == Some(14.into()))
            .and_then(|(_, value)| value.as_integer())
            .and_then(|value| u64::try_from(value).ok());
        assert_eq!(written, Some(code), "{mode:?}: its code");
    }
    Ok(())
}

#[test]
fn a_by_two_grant_signs_as_version_four_and_verifies() -> TestResult {
    let event = issued(to_agent()?.with_mode(Mode::ByTwo))?;
    assert_eq!(event.version(), 4, "a held mode is envelope version 4");
    let (message, public) = signed(event.clone())?;
    let read = verify_grant_event(&message, &public)?;
    assert_eq!(read.event(), &event);
    let GrantChange::Issue(grant) = read.event().change() else {
        return Err("the event issues a grant".into());
    };
    assert_eq!(grant.mode(), Mode::ByTwo);
    let body = encode_event_body(&event);
    assert_eq!(decode_event_body(&body)?, event);
    Ok(())
}

#[test]
fn an_outright_grant_never_writes_key_fourteen() -> TestResult {
    let grant = to_agent()?.with_mode(Mode::Outright);
    assert_eq!(encode_grant(&grant), encode_grant(&to_agent()?));
    assert_eq!(issued(grant)?.version(), 1);
    Ok(())
}

#[test]
fn a_mode_written_as_outright_or_unknown_is_refused_by_name() -> TestResult {
    let mut encoded = encode_grant(&to_agent()?.with_mode(Mode::ByDraft));
    let last = encoded.len() - 1;
    for code in [0_u8, 3] {
        encoded[last] = code;
        let refused = decode_grant(&encoded)
            .err()
            .ok_or("a bad mode code was read")?;
        assert!(
            matches!(refused, GrantError::GrantMalformed { .. }),
            "code {code}: {refused:?}"
        );
    }
    Ok(())
}

#[test]
fn the_mode_names_itself_in_words() {
    assert_eq!(Mode::Outright.as_str(), "outright");
    assert_eq!(Mode::ByDraft.as_str(), "by_draft");
    assert_eq!(Mode::ByTwo.as_str(), "by_two");
}
