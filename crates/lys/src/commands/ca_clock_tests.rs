#![cfg(test)]

//! Stored issuer certificates read the creation clock only when first created.

use std::sync::atomic::{AtomicUsize, Ordering};

use super::*;
use lys_core::clock::{Clock, ClockError};

#[derive(Debug)]
struct RefusingClock {
    reads: AtomicUsize,
}

impl Clock for RefusingClock {
    fn now(&self) -> Result<DateTime<Utc>, ClockError> {
        self.reads.fetch_add(1, Ordering::SeqCst);
        Err(ClockError::Unavailable {
            reason: "creation read refused".to_owned(),
        })
    }
}

fn failing_authority(key: &Path) -> CliResult<(CertificateAuthority, Arc<RefusingClock>)> {
    let clock = Arc::new(RefusingClock {
        reads: AtomicUsize::new(0),
    });
    let provider = Arc::clone(&clock);
    let authority = CertificateAuthority::with_clock(load_identity(key)?, provider);
    Ok((authority, clock))
}

#[test]
fn stored_issuer_reuse_does_not_read_a_failing_creation_clock() {
    let directory = tempfile::tempdir().unwrap();
    let key = directory.path().join("issuer.key");
    let identity = lys_core::Ed25519Identity::load_or_generate(&key).unwrap();
    let original = CertificateAuthority::new(identity)
        .issuer_certificate_der()
        .unwrap();
    let original = pem::encode_certificate(&original).into_bytes();
    std::fs::write(stored_issuer_certificate_path(&key), &original).unwrap();
    let (authority, clock) = failing_authority(&key).unwrap();
    let stored = stored_issuer_certificate(&key, &authority).unwrap();
    assert_eq!(stored.pem, original);
    assert!(stored.built.is_none());
    assert_eq!(clock.reads.load(Ordering::SeqCst), 0);
    assert_eq!(
        std::fs::read(stored_issuer_certificate_path(&key)).unwrap(),
        original
    );
}

#[test]
fn invalid_stored_issuer_refuses_without_rebuilding_or_reading_time() {
    let directory = tempfile::tempdir().unwrap();
    let key = directory.path().join("issuer.key");
    lys_core::Ed25519Identity::load_or_generate(&key).unwrap();
    let path = stored_issuer_certificate_path(&key);
    std::fs::write(&path, b"invalid stored issuer").unwrap();
    let (authority, clock) = failing_authority(&key).unwrap();
    let result = stored_issuer_certificate(&key, &authority);
    assert!(matches!(
        result,
        Err(CliError::StoredIssuerCertificateInvalid { .. })
    ));
    assert_eq!(clock.reads.load(Ordering::SeqCst), 0);
    assert_eq!(std::fs::read(path).unwrap(), b"invalid stored issuer");
}

#[test]
fn absent_stored_issuer_clock_failure_writes_no_certificate() {
    let directory = tempfile::tempdir().unwrap();
    let key = directory.path().join("issuer.key");
    lys_core::Ed25519Identity::load_or_generate(&key).unwrap();
    let (authority, clock) = failing_authority(&key).unwrap();
    let output = directory.path().join("export.pem");
    let provider = Arc::clone(&clock);
    let result = issuer_cert_with_clock(&key, &output, true, ClockSource::Supplied(provider));
    assert!(matches!(
        result,
        Err(CliError::Trust(TrustError::CertificateGeneration { .. }))
    ));
    assert_eq!(clock.reads.load(Ordering::SeqCst), 1);
    assert!(!stored_issuer_certificate_path(&key).exists());
    assert!(!output.exists());
    assert_eq!(
        authority.public_key_bytes(),
        load_identity(&key).unwrap().public_key_bytes()
    );
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 1);
}
