//! The certificate log: issuance and withdrawal are entered and never
//! edited, a restart reads only the leaves after the snapshot, and an entry
//! is proven against the log's root.

use std::error::Error;
use std::path::Path;
use std::sync::Arc;

use lys_core::Ed25519Identity;
use lys_core::merkle::verify_inclusion_raw;
use lys_identity_server::certificates_store::{CertificateStore, Issued, Withdrawn};
use lys_identity_server::error::ServerError;
use lys_log_store::Start;

type TestResult = Result<(), Box<dyn Error>>;

fn key(dir: &Path) -> Result<Arc<Ed25519Identity>, Box<dyn Error>> {
    Ok(Arc::new(Ed25519Identity::load_or_generate(
        &dir.join("certificates.key"),
    )?))
}

fn issued(serial: &str, der: &str) -> Issued {
    Issued {
        serial: serial.to_owned(),
        agent: "agent-1".to_owned(),
        person: "person-1".to_owned(),
        claims: serde_json::json!({ "role": "builder", "version": 1 }),
        der: der.to_owned(),
        issued_at: 10,
    }
}

fn withdrawn(serial: &str, by: &str) -> Withdrawn {
    Withdrawn {
        serial: serial.to_owned(),
        by: by.to_owned(),
        reason: "role changed".to_owned(),
        withdrawn_at: 20,
    }
}

#[test]
fn a_withdrawal_is_entered_and_the_issuance_is_kept_unchanged() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("certificates");
    let mut store = CertificateStore::open(&path, key(dir.path())?)?;
    store.issue(issued("s-1", "AAAA"))?;
    let before = store.prove("s-1")?.leaf_bytes;
    store.withdraw(withdrawn("s-1", "person-1"))?;
    store.withdraw(withdrawn("s-1", "person-1"))?;
    let refused = store.withdraw(withdrawn("s-1", "person-2"));
    assert!(
        matches!(&refused, Err(ServerError::CertificateWithdrawn { by, .. }) if by == "person-1"),
        "{refused:?}"
    );
    let proven = store.prove("s-1")?;
    assert_eq!(
        proven.leaf_bytes, before,
        "the issuance leaf is never edited"
    );
    assert_eq!(proven.tree_size, 2);
    assert!(proven.entered.withdrawn.is_some());
    Ok(())
}

#[test]
fn a_serial_names_one_certificate() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mut store = CertificateStore::open(&dir.path().join("c"), key(dir.path())?)?;
    store.issue(issued("s-1", "AAAA"))?;
    store.issue(issued("s-1", "AAAA"))?;
    let refused = store.issue(issued("s-1", "BBBB"));
    assert!(
        matches!(refused, Err(ServerError::CertificateReused { .. })),
        "{refused:?}"
    );
    let unknown = store.withdraw(withdrawn("s-9", "person-1"));
    assert!(
        matches!(unknown, Err(ServerError::CertificateUnknown { .. })),
        "{unknown:?}"
    );
    Ok(())
}

#[test]
fn a_restart_reads_only_the_entries_after_the_snapshot() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("certificates");
    drop(CertificateStore::open(&path, key(dir.path())?)?);
    let mut store = CertificateStore::open(&path, key(dir.path())?)?;
    store.issue(issued("s-1", "AAAA"))?;
    store.issue(issued("s-2", "BBBB"))?;
    store.withdraw(withdrawn("s-1", "person-1"))?;
    drop(store);

    let store = CertificateStore::open(&path, key(dir.path())?)?;
    assert_eq!(
        store.start(),
        &Start::Resumed {
            size: 0,
            replayed: 3
        }
    );
    assert_eq!(store.certificates().count(), 2);
    assert!(
        store
            .certificate("s-1")
            .is_some_and(|c| c.withdrawn.is_some())
    );
    Ok(())
}

#[test]
fn an_entry_is_proven_against_the_logs_root() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mut store = CertificateStore::open(&dir.path().join("c"), key(dir.path())?)?;
    for serial in ["s-1", "s-2", "s-3"] {
        store.issue(issued(serial, serial))?;
    }
    let proven = store.prove("s-2")?;
    verify_inclusion_raw(
        &proven.root,
        &proven.leaf_bytes,
        proven.entered.leaf,
        &proven.proof,
    )?;
    Ok(())
}
