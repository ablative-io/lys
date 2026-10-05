//! The bytes written before the connector kind, held fixed (DIRECTORY-048 R1,
//! DIRECTORY-080 R1). Each fixture was written by the code at main 5215ed6c
//! from the fixed inputs below and is never regenerated: every pin proves
//! today's reader reads its fixture and today's writer writes the same bytes
//! from the same inputs.

use std::collections::BTreeSet;
use std::error::Error;

use lys_core::merkle::{InclusionProof, raw_leaf_hash};
use lys_identity::grants::{
    Action, Grant, GrantChange, GrantError, GrantEvent, GrantId, GrantParts, GrantReceipt, PassOn,
    RecipientKind, Relation, Resource, Source, Window, decode_event_body, decode_grant,
    encode_event_body, encode_grant, sign_grant_event, verify_grant_event, verify_grant_receipt,
};
use lys_identity::log::Coordinate;
use lys_identity::signer::load_service_key;
use lys_identity::{AgentId, ConnectorId, IdentityId, OperationId, PersonId, ServiceAccountId};
use serde_json::Value;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE: &str = include_str!("fixtures/connector-5215ed6c.json");
const SERVICE_KEY: [u8; 32] = [0x5e; 32];
const PERSON: [u8; 16] = [0xd1; 16];
const AGENT: [u8; 16] = [0xa7; 16];
const ACCOUNT: [u8; 16] = [0x5a; 16];
const SOURCE: [u8; 16] = [0x51; 16];
const ID: [u8; 16] = [0x61; 16];
const OPERATION: [u8; 16] = [0x0e; 16];

fn fixture() -> Result<Value, Box<dyn Error>> {
    let fixture: Value = serde_json::from_str(FIXTURE)?;
    assert_eq!(
        fixture["source_commit"],
        "5215ed6c36f057aa11f51757b0e2a9ce61204d19"
    );
    Ok(fixture)
}

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

/// The fixture's bytes under `name`, after proving `written` is the same.
fn pinned(fixture: &Value, name: &str, written: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
    let text = fixture[name]
        .as_str()
        .ok_or_else(|| format!("the fixture has no {name}"))?;
    assert_eq!(
        text,
        hex(written),
        "{name}: today's writer no longer writes the fixture's bytes"
    );
    unhex(text)
}

fn actions(names: &[&str]) -> Result<BTreeSet<Action>, GrantError> {
    names.iter().map(|name| Action::new(name)).collect()
}

/// A grant from the person to `holder`, passable to `recipients`.
fn grant(holder: IdentityId, recipients: &[RecipientKind]) -> Result<Grant, GrantError> {
    Grant::new(GrantParts {
        id: GrantId::from_bytes(ID),
        issuer: IdentityId::Person(PersonId::from_bytes(PERSON)),
        holder,
        responsible: PersonId::from_bytes(PERSON),
        resource: Resource::new("project", "alpha")?,
        relation: Relation::new("heron")?,
        actions: actions(&["read", "write"])?,
        pass_on: PassOn::to(actions(&["read"])?, recipients.iter().copied().collect())?,
        source: Source::Grant(GrantId::from_bytes(SOURCE)),
        window: Window::new(1_000, Some(2_000))?,
        model_version: 3,
        operation: OperationId::from_bytes(OPERATION),
    })
}

fn to_agent() -> Result<Grant, GrantError> {
    grant(
        IdentityId::Agent(AgentId::from_bytes(AGENT)),
        &[RecipientKind::Person, RecipientKind::Agent],
    )
}

fn to_account() -> Result<Grant, GrantError> {
    grant(
        IdentityId::ServiceAccount(ServiceAccountId::from_bytes(ACCOUNT)),
        &[
            RecipientKind::Person,
            RecipientKind::Agent,
            RecipientKind::ServiceAccount,
        ],
    )
}

fn issued(grant: Grant) -> Result<GrantEvent, GrantError> {
    GrantEvent::new(
        OperationId::from_bytes(OPERATION),
        IdentityId::Person(PersonId::from_bytes(PERSON)),
        1_500,
        GrantChange::Issue(Box::new(grant)),
    )
}

/// The event signed with the fixed service key, and that key's public half.
fn signed(event: GrantEvent) -> Result<(Vec<u8>, [u8; 32]), Box<dyn Error>> {
    let temp = tempfile::tempdir()?;
    let path = temp.path().join("service.key");
    std::fs::write(&path, SERVICE_KEY)?;
    let key = load_service_key(&path)?;
    let message = sign_grant_event(event, &key)?.bytes().to_vec();
    Ok((message, key.public_key_bytes()))
}

#[test]
fn a_grant_body_to_an_agent_and_to_a_service_account_keeps_its_bytes() -> TestResult {
    let fixture = fixture()?;
    for (name, grant) in [
        ("grant_body_to_agent", to_agent()?),
        ("grant_body_to_service_account", to_account()?),
    ] {
        let bytes = pinned(&fixture, name, &encode_grant(&grant))?;
        assert_eq!(decode_grant(&bytes)?, grant, "{name}: today's reader");
    }
    Ok(())
}

#[test]
fn a_signed_grant_event_of_each_envelope_version_keeps_its_bytes() -> TestResult {
    let fixture = fixture()?;
    for (name, version, event) in [
        ("event_v1_person_to_agent", 1, issued(to_agent()?)?),
        (
            "event_v2_person_to_service_account",
            2,
            issued(to_account()?)?,
        ),
    ] {
        assert_eq!(event.version(), version, "{name}: its envelope version");
        let (message, public) = signed(event.clone())?;
        assert_eq!(fixture["public_key"], hex(&public), "the fixed service key");
        let bytes = pinned(&fixture, name, &message)?;
        let read = verify_grant_event(&bytes, &public)?;
        assert_eq!(read.event(), &event, "{name}: today's reader");
        assert_eq!(read.bytes(), bytes.as_slice());
        let body = encode_event_body(&event);
        assert_eq!(decode_event_body(&body)?, event);
    }
    Ok(())
}

/// The version 3 event, written by the code that first signed a connector
/// (box 10 part 2) from the same inputs, and never regenerated.
const FIXTURE_V3: &str = include_str!("fixtures/connector-v3.json");
const CONNECTOR: [u8; 16] = [0xc0; 16];

fn to_connector() -> Result<Grant, GrantError> {
    grant(
        IdentityId::Connector(ConnectorId::from_bytes(CONNECTOR)),
        &[
            RecipientKind::Person,
            RecipientKind::Agent,
            RecipientKind::ServiceAccount,
            RecipientKind::Connector,
        ],
    )
}

#[test]
fn a_grant_body_and_signed_event_to_a_connector_keep_their_version_three_bytes() -> TestResult {
    let fixture: Value = serde_json::from_str(FIXTURE_V3)?;
    let grant = to_connector()?;
    let body = pinned(&fixture, "grant_body_to_connector", &encode_grant(&grant))?;
    assert_eq!(decode_grant(&body)?, grant, "today's reader");
    let event = issued(grant)?;
    assert_eq!(event.version(), 3);
    let (message, public) = signed(event.clone())?;
    assert_eq!(fixture["public_key"], hex(&public), "the fixed service key");
    let bytes = pinned(&fixture, "event_v3_person_to_connector", &message)?;
    let read = verify_grant_event(&bytes, &public)?;
    assert_eq!(read.event(), &event, "today's reader");
    Ok(())
}

#[test]
fn a_grant_receipt_keeps_every_member_and_verifies_against_its_fixture() -> TestResult {
    let fixture = fixture()?;
    let event = issued(to_agent()?)?;
    let (message, public) = signed(event)?;
    let message = pinned(&fixture, "event_v1_person_to_agent", &message)?;
    let leaf = raw_leaf_hash(&message);
    let coordinate = Coordinate {
        index: 0,
        tree_size: 1,
        root: leaf,
        leaf_hash: leaf,
    };
    let receipt = GrantReceipt::of(&verify_grant_event(&message, &public)?, coordinate);
    let written = format!(
        "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
        receipt.version,
        receipt.operation,
        receipt.caller,
        receipt.grant,
        receipt.change_kind,
        hex(&receipt.payload_commitment),
        receipt.coordinate.index,
        receipt.coordinate.tree_size,
        hex(&receipt.coordinate.root),
        hex(&receipt.coordinate.leaf_hash),
    );
    assert_eq!(
        fixture["receipt_v1"], written,
        "receipt_v1: today's receipt no longer carries the fixture's members"
    );
    verify_grant_receipt(
        &receipt,
        &message,
        &public,
        (1, leaf),
        &InclusionProof::try_from_bytes(Vec::new())?,
    )?;
    Ok(())
}
