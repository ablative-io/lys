//! R4: one logical event per source operation, observations kept apart, provenance kept on replay.

use std::error::Error;

use identity_contract::fixtures::{administrator, linked, op, shown, source};
use identity_contract::harness::Harness;
use lys_identity::{Change, IdentityError, IdentityId, LinkChange, LinkObservation, LoginBinding};

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
        .log()?
        .leaf(first.coordinate().index)?
        .ok_or("leaf missing")?
        .to_vec();
    let event = lys_identity::verify_event(&leaf, &restarted.service_key())?;
    assert_eq!(
        event.event().actor(),
        &source()?,
        "the source's provenance survives replay"
    );
    assert!(matches!(event.event().change(), Change::LinkAudit(_)));
    assert_eq!(restarted.log()?.len()?, 2);
    Ok(())
}

#[test]
fn an_observer_longer_than_an_issuer_is_refused_before_it_is_signed() -> TestResult {
    let binding = LoginBinding::new("https://accounts.test", "ada-elsewhere")?;
    let observer = format!("https://{}", "o".repeat(2048));
    let refused = LinkObservation::new("op-1", LinkChange::Linked, binding, &observer, 1);
    assert!(matches!(refused, Err(IdentityError::ChangeMismatch { .. })));
    let harness = Harness::new(7)?;
    let mut directory = harness.open()?;
    let (person, _) = directory.register_person(administrator()?, op(1), shown("Ada")?, 10)?;
    let widest = format!("https://{}", "o".repeat(2040));
    let observation = LinkObservation::new(
        "op-1",
        LinkChange::Linked,
        LoginBinding::new("https://accounts.test", "ada-elsewhere")?,
        &widest,
        1,
    )?;
    directory.accept_link_audit(source()?, person, observation, 20)?;
    drop(directory);
    assert_eq!(
        harness.open()?.log()?.len()?,
        2,
        "the widest observer allowed is signed, logged and read back"
    );
    Ok(())
}
