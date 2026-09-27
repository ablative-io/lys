//! R4: one logical event per source operation, observations kept apart, provenance kept on replay.

use std::error::Error;

use identity_contract::fixtures::{administrator, linked, op, shown, source};
use identity_contract::harness::Harness;
use lys_identity::{Change, IdentityId};

type TestResult = Result<(), Box<dyn Error>>;

#[test]
fn a_redelivered_source_operation_is_answered_once_and_survives_a_restart() -> TestResult {
    let harness = Harness::new(7)?;
    let mut directory = harness.open()?;
    let (person, _) = directory.register_person(administrator()?, op(1), shown("Ada")?, 10)?;
    let first = directory.accept_link_audit(source()?, person, linked("rauthy-op-1")?, 20)?;
    let again = directory.accept_link_audit(source()?, person, linked("rauthy-op-1")?, 21)?;
    assert_eq!(
        first, again,
        "a duplicate delivery answers the first receipt"
    );
    drop(directory);

    let mut restarted = harness.open()?;
    let replayed = restarted.accept_link_audit(source()?, person, linked("rauthy-op-1")?, 22)?;
    assert_eq!(
        replayed, first,
        "a delivery after a restart is still answered once"
    );
    let record = restarted
        .record(IdentityId::Person(person))?
        .ok_or("person missing")?;
    assert!(
        record.bindings().is_empty(),
        "an observation binds no login"
    );
    let leaf = restarted
        .log()
        .leaf(first.coordinate().index)
        .ok_or("leaf missing")?;
    let event = lys_identity::verify_event(leaf, &restarted.service_key())?;
    assert_eq!(
        event.event().actor(),
        &source()?,
        "the source's provenance survives replay"
    );
    assert!(matches!(event.event().change(), Change::LinkAudit(_)));
    assert_eq!(restarted.log().len(), 2);
    Ok(())
}
