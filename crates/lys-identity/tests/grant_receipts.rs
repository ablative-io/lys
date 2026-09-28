//! R3: an independently verified grant receipt, and its envelope
//! (`GRANT_AUDIT`, with the signed half of `GRANT_WIRE_BOUNDARY`).

mod support;

use std::error::Error;

use ciborium::Value;
use lys_core::Ed25519Identity;
use lys_identity::grants::{
    GRANT_ENVELOPE, Grant, GrantChange, GrantError, GrantEvent, GrantId, GrantParts,
    MemoryRelationships, PassOn, RecipientKind, Resource, Route, Source, decode_event_body,
    encode_event_body, verify_grant_event, verify_grant_receipt,
};
use lys_identity::{IdentityId, verify_event};
use support::{T0, World, actions, alpha, pass};

type TestResult = Result<(), Box<dyn Error>>;

/// One change to an issued grant's members.
type Tampering<'a> = Box<dyn Fn(&mut GrantParts) + 'a>;

const BOTH: [RecipientKind; 2] = [RecipientKind::Person, RecipientKind::Agent];

fn service_key(world: &World) -> Result<[u8; 32], Box<dyn Error>> {
    Ok(Ed25519Identity::load(&world.dir.path().join("service.key"))?.public_key_bytes())
}

/// The four parts of a signed message.
fn parts(message: &[u8]) -> Result<Vec<Value>, Box<dyn Error>> {
    let Value::Tag(18, inner) = ciborium::from_reader::<Value, _>(message)? else {
        return Err("not a tagged COSE_Sign1".into());
    };
    let Value::Array(items) = *inner else {
        return Err("not an array".into());
    };
    Ok(items)
}

fn bytes_of(value: &Value) -> Result<Vec<u8>, Box<dyn Error>> {
    match value {
        Value::Bytes(raw) => Ok(raw.clone()),
        _ => Err("not a byte string".into()),
    }
}

/// `message` with its issued grant changed by `change`, the caller changed to
/// its issuer, and the original protected header and signature kept.
fn tampered(message: &[u8], change: impl Fn(&mut GrantParts)) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut items = parts(message)?;
    let event = decode_event_body(&bytes_of(&items[2])?)?;
    let GrantChange::Issue(grant) = event.change() else {
        return Err("not an issued grant".into());
    };
    let mut changed = grant.parts().clone();
    change(&mut changed);
    let forged = GrantEvent::new(
        event.operation(),
        changed.issuer,
        event.recorded_at(),
        GrantChange::Issue(Box::new(Grant::new(changed)?)),
    )?;
    items[2] = Value::Bytes(encode_event_body(&forged));
    let mut out = Vec::new();
    ciborium::into_writer(&Value::Tag(18, Box::new(Value::Array(items))), &mut out)?;
    Ok(out)
}

#[test]
fn grant_audit_a_receipt_verifies_and_every_tampering_is_rejected() -> TestResult {
    let mut world = World::new()?;
    let (dana, tom) = (
        IdentityId::Person(world.dana),
        IdentityId::Person(world.tom),
    );
    let root = world.root(world.dana, "kite", pass(&["read"], &BOTH)?, None)?;
    let request = world.request(dana, root, tom, "tern", pass(&["read"], &BOTH)?, None)?;
    let recorded = world.delegate(&request)?;
    let lent = recorded.event.grant();
    let to_agent = world.request(
        tom,
        lent,
        IdentityId::Agent(world.tom_agent),
        "tern",
        PassOn::UseOnly,
        None,
    )?;
    world.delegate(&to_agent)?;
    let key = service_key(&world)?;
    let index = usize::try_from(recorded.index)?;
    let message = world.grants.events()?[index].0.bytes().to_vec();
    let checkpoint = world.grants.ledger().head()?;
    let proof = world.grants.ledger().inclusion_proof(recorded.index)?;
    verify_grant_receipt(&recorded.receipt, &message, &key, checkpoint, &proof)?;
    assert!(recorded.event.recorded_at() > T0);

    let other = GrantId::from_bytes([0x44; 16]);
    let tamperings: [(&str, Tampering<'_>); 5] = [
        (
            "actor",
            Box::new(|parts| parts.issuer = IdentityId::Person(world.lee)),
        ),
        (
            "source grant",
            Box::new(move |parts| parts.source = Source::Grant(other)),
        ),
        (
            "recipient",
            Box::new(|parts| {
                parts.holder = IdentityId::Agent(world.dana_agent);
                parts.responsible = world.dana;
            }),
        ),
        (
            "resource",
            Box::new(|parts| {
                if let Ok(beta) = Resource::new("project", "beta") {
                    parts.resource = beta;
                }
            }),
        ),
        (
            "actions",
            Box::new(|parts| {
                if let Ok(wider) = actions(&["read", "write"]) {
                    parts.actions = wider;
                }
            }),
        ),
    ];
    for (what, change) in &tamperings {
        let forged = tampered(&message, change)?;
        assert_ne!(forged, message, "{what} was changed");
        assert_eq!(
            verify_grant_event(&forged, &key),
            Err(GrantError::SignatureInvalid),
            "{what}"
        );
        assert_eq!(
            verify_grant_receipt(&recorded.receipt, &forged, &key, checkpoint, &proof),
            Err(GrantError::SignatureInvalid),
            "{what}"
        );
    }
    let mut resequenced = recorded.receipt.clone();
    resequenced.coordinate.index += 1;
    assert!(
        matches!(
            verify_grant_receipt(&resequenced, &message, &key, checkpoint, &proof),
            Err(GrantError::ReceiptInvalid { .. })
        ),
        "sequence"
    );
    let mut resigned = message;
    if let Some(last) = resigned.last_mut() {
        *last ^= 0x01;
    }
    assert_eq!(
        verify_grant_event(&resigned, &key),
        Err(GrantError::SignatureInvalid),
        "signature"
    );
    assert!(matches!(
        verify_grant_receipt(&recorded.receipt, &resigned, &key, checkpoint, &proof),
        Err(GrantError::SignatureInvalid)
    ));
    assert_eq!(tamperings.len(), 5);
    Ok(())
}

#[test]
fn grant_audit_a_refusal_never_appears_as_a_grant_event() -> TestResult {
    let mut world = World::new()?;
    let (dana, tom) = (
        IdentityId::Person(world.dana),
        IdentityId::Person(world.tom),
    );
    let root = world.root(world.dana, "kite", pass(&["read"], &BOTH)?, None)?;
    let request = world.request(dana, root, tom, "tern", PassOn::UseOnly, None)?;
    let lent = world.delegate(&request)?.event.grant();
    let refused = world.request(
        tom,
        lent,
        IdentityId::Agent(world.tom_agent),
        "tern",
        PassOn::UseOnly,
        None,
    )?;
    assert_eq!(
        world.delegate(&refused),
        Err(GrantError::UseOnly {
            grant: lent.to_string()
        })
    );
    let before = world.events();
    world.reopen(MemoryRelationships::default())?;
    assert_eq!(world.events(), before);
    let named = world
        .grants
        .events()?
        .iter()
        .filter(|(signed, _)| signed.event().operation() == refused.operation)
        .count();
    assert_eq!(named, 0, "the refused operation has no event in the log");
    assert_eq!(world.grants.book().operation(refused.operation), None);
    assert!(
        world
            .exercise(IdentityId::Agent(world.tom_agent), "read", Route::Tool)
            .is_err()
    );
    Ok(())
}

#[test]
fn grant_wire_boundary_a_signed_grant_names_its_envelope_and_no_other_is_read() -> TestResult {
    let mut world = World::new()?;
    let root = world.root(world.dana, "kite", pass(&["read"], &BOTH)?, None)?;
    let message = world.grants.events()?[0].0.bytes().to_vec();
    let protected: Value = ciborium::from_reader(bytes_of(&parts(&message)?[0])?.as_slice())?;
    let Value::Map(header) = protected else {
        return Err("the protected header is not a map".into());
    };
    assert!(header.contains(&(
        Value::Integer(3.into()),
        Value::Text(GRANT_ENVELOPE.to_owned())
    )));
    let key = service_key(&world)?;
    let identity_event = world.directory.log()?.leaf(0)?.ok_or("no identity event")?;
    assert!(
        matches!(
            verify_grant_event(&identity_event, &key),
            Err(GrantError::EnvelopeMismatch { .. })
        ),
        "an identity event is not read as a grant"
    );
    assert!(
        verify_event(&message, &key).is_err(),
        "a grant is not read as an identity event"
    );
    let event = verify_grant_event(&message, &key)?;
    let GrantChange::Issue(grant) = event.event().change() else {
        return Err("not an issued grant".into());
    };
    assert_eq!((grant.id(), grant.resource()), (root, &alpha()?));
    Ok(())
}
