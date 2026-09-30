//! The link-audit receiver answers a redelivery with its first receipt only
//! when the redelivery is the observation it first accepted. A source
//! operation id delivered again with another person or another observation is
//! refused, so one operation's acknowledgement is never handed to another.

#![cfg(unix)]

use std::error::Error;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use lys_core::Ed25519Identity;
use lys_identity::{
    Actor, AuthMethod, Directory, IdentityError, IdentityId, LinkChange, LinkObservation,
    LoginBinding, OperationId, PersonId, Profile, Provenance,
};
use lys_log_store::FileLeafStore;

type TestResult = Result<(), Box<dyn Error>>;

const ISSUER: &str = "https://issuer.test";

struct Receiver {
    dir: tempfile::TempDir,
    directory: Directory<FileLeafStore>,
    source: Actor,
    ada: PersonId,
    grace: PersonId,
}

fn receiver() -> Result<Receiver, Box<dyn Error>> {
    let dir = tempfile::TempDir::new()?;
    let log: PathBuf = dir.path().join("log");
    FileLeafStore::create(&log, "example.test/lys/directory")?;
    let key = dir.path().join("service.key");
    std::fs::write(&key, [11_u8; 32])?;
    std::fs::set_permissions(&key, std::fs::Permissions::from_mode(0o600))?;
    let reopen = Box::new(move || FileLeafStore::open(&log));
    let mut directory = Directory::open(reopen, Ed25519Identity::load(&key)?)?;
    let administrator = Actor::new(
        LoginBinding::new(ISSUER, "administrator")?,
        Provenance::new(AuthMethod::Oidc, 1_790_000_000),
    );
    let (ada, _) = directory.register_person(
        administrator.clone(),
        OperationId::from_bytes([1; 16]),
        Profile::new("Ada")?,
        10,
    )?;
    let (grace, _) = directory.register_person(
        administrator,
        OperationId::from_bytes([2; 16]),
        Profile::new("Grace")?,
        11,
    )?;
    let source = Actor::new(
        LoginBinding::new(ISSUER, "lys-link-audit")?,
        Provenance::new(AuthMethod::Oidc, 1_790_000_100),
    );
    Ok(Receiver {
        dir,
        directory,
        source,
        ada,
        grace,
    })
}

const PROVIDER: &str = "https://accounts.provider.test";
const OBSERVER: &str = "issuer.test";
const OBSERVED_AT: u64 = 1_790_000_200;

fn observed(change: LinkChange, subject: &str) -> Result<LinkObservation, Box<dyn Error>> {
    reported(change, PROVIDER, subject, OBSERVER, OBSERVED_AT)
}

fn reported(
    change: LinkChange,
    issuer: &str,
    subject: &str,
    observer: &str,
    observed_at: u64,
) -> Result<LinkObservation, Box<dyn Error>> {
    Ok(LinkObservation::new(
        "source-operation-1",
        change,
        LoginBinding::new(issuer, subject)?,
        observer,
        observed_at,
    )?)
}

fn events_of(receiver: &mut Receiver, person: PersonId) -> Result<usize, Box<dyn Error>> {
    Ok(receiver
        .directory
        .record(IdentityId::Person(person))?
        .ok_or("the person cannot be read")?
        .events()
        .len())
}

fn refused_as_seen(outcome: Result<lys_identity::receipt::Receipt, IdentityError>) -> TestResult {
    match outcome {
        Err(IdentityError::LinkSourceSeen {
            source_operation_id,
        }) => {
            assert_eq!(source_operation_id, "source-operation-1");
            Ok(())
        }
        Err(other) => Err(format!("refused by another name: {other}").into()),
        Ok(receipt) => Err(format!(
            "a different delivery was acknowledged with the receipt at index {}",
            receipt.coordinate().index
        )
        .into()),
    }
}

#[test]
fn a_redelivered_observation_is_answered_with_its_first_receipt() -> TestResult {
    let mut receiver = receiver()?;
    assert!(receiver.dir.path().join("log").exists());
    let (source, ada) = (receiver.source.clone(), receiver.ada);
    let first = receiver.directory.accept_link_audit(
        source.clone(),
        ada,
        observed(LinkChange::Linked, "subject-1")?,
        20,
    )?;
    let recorded = events_of(&mut receiver, ada)?;
    let again = receiver.directory.accept_link_audit(
        source,
        ada,
        observed(LinkChange::Linked, "subject-1")?,
        21,
    )?;
    assert_eq!(again.coordinate().index, first.coordinate().index);
    assert_eq!(again.operation(), first.operation());
    assert_eq!(events_of(&mut receiver, ada)?, recorded);
    Ok(())
}

#[test]
fn a_source_operation_delivered_again_with_another_observation_is_refused() -> TestResult {
    let mut receiver = receiver()?;
    let (source, ada) = (receiver.source.clone(), receiver.ada);
    receiver.directory.accept_link_audit(
        source.clone(),
        ada,
        observed(LinkChange::Linked, "subject-1")?,
        20,
    )?;
    let recorded = events_of(&mut receiver, ada)?;
    refused_as_seen(receiver.directory.accept_link_audit(
        source.clone(),
        ada,
        observed(LinkChange::Linked, "subject-2")?,
        21,
    ))?;
    refused_as_seen(receiver.directory.accept_link_audit(
        source.clone(),
        ada,
        observed(LinkChange::Unlinked, "subject-1")?,
        22,
    ))?;
    let changed = [
        reported(
            LinkChange::Linked,
            "https://other.provider.test",
            "subject-1",
            OBSERVER,
            OBSERVED_AT,
        )?,
        reported(
            LinkChange::Linked,
            PROVIDER,
            "subject-1",
            "another.observer.test",
            OBSERVED_AT,
        )?,
        reported(
            LinkChange::Linked,
            PROVIDER,
            "subject-1",
            OBSERVER,
            OBSERVED_AT + 1,
        )?,
    ];
    for observation in changed {
        refused_as_seen(receiver.directory.accept_link_audit(
            source.clone(),
            ada,
            observation,
            23,
        ))?;
    }
    assert_eq!(events_of(&mut receiver, ada)?, recorded);
    Ok(())
}

#[test]
fn a_source_operation_delivered_again_for_another_person_is_refused() -> TestResult {
    let mut receiver = receiver()?;
    let (source, ada, grace) = (receiver.source.clone(), receiver.ada, receiver.grace);
    receiver.directory.accept_link_audit(
        source.clone(),
        ada,
        observed(LinkChange::Linked, "subject-1")?,
        20,
    )?;
    let recorded = events_of(&mut receiver, grace)?;
    refused_as_seen(receiver.directory.accept_link_audit(
        source,
        grace,
        observed(LinkChange::Linked, "subject-1")?,
        21,
    ))?;
    assert_eq!(events_of(&mut receiver, grace)?, recorded);
    Ok(())
}
