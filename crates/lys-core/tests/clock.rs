#![cfg(test)]

use std::sync::atomic::{AtomicI64, AtomicUsize, Ordering};
use std::sync::{Arc, Barrier};
use std::time::{Duration, SystemTime};

use chrono::{DateTime, Timelike, Utc};
use lys_core::ca::{CertificateAuthority, create_certificate_request, verify_certificate_chain_at};
use lys_core::clock::{Clock, ClockError, SystemClock, unix_seconds};
use lys_core::{Ed25519Identity, TrustError};
use rcgen::CustomExtension;
use x509_parser::prelude::{FromDer, X509Certificate};
use zeroize::Zeroizing;

const FIRST: i64 = 1_700_000_000;
const ISSUER_END: i64 = 253_402_300_799;

#[derive(Debug)]
struct SuppliedClock {
    seconds: AtomicI64,
    reads: AtomicUsize,
    fail_on: usize,
    nanos: u32,
}

impl SuppliedClock {
    fn at(seconds: i64) -> Self {
        Self {
            seconds: AtomicI64::new(seconds),
            reads: AtomicUsize::new(0),
            fail_on: 0,
            nanos: 0,
        }
    }

    fn advance_to(&self, seconds: i64) {
        self.seconds.store(seconds, Ordering::SeqCst);
    }

    fn reads(&self) -> usize {
        self.reads.load(Ordering::SeqCst)
    }
}

impl Clock for SuppliedClock {
    fn now(&self) -> Result<DateTime<Utc>, ClockError> {
        let read = self.reads.fetch_add(1, Ordering::SeqCst) + 1;
        if self.fail_on == read {
            return Err(ClockError::Unavailable {
                reason: "supplied read refused".to_owned(),
            });
        }
        DateTime::from_timestamp(self.seconds.load(Ordering::SeqCst), self.nanos)
            .ok_or(ClockError::InstantOutOfRange { source: None })
    }
}

fn supplied_authority(clock: &Arc<SuppliedClock>) -> CertificateAuthority {
    let provider = Arc::clone(clock);
    CertificateAuthority::with_clock(
        Ed25519Identity::from_seed(&Zeroizing::new([17; 32])),
        provider,
    )
}

fn instant(seconds: i64) -> DateTime<Utc> {
    DateTime::from_timestamp(seconds, 0).unwrap()
}

fn parsed(der: &[u8]) -> X509Certificate<'_> {
    let (rest, certificate) = X509Certificate::from_der(der).unwrap();
    assert!(rest.is_empty());
    certificate
}

fn clock_refusal(error: &TrustError) {
    assert!(matches!(error, TrustError::CertificateGeneration { reason }
        if reason.contains("clock unavailable") && reason.contains("supplied read refused")));
}

#[test]
fn system_clock_and_existing_authority_constructor_use_system_time() {
    let before = DateTime::<Utc>::from(SystemTime::now());
    let supplied = SystemClock.now().unwrap();
    let authority = CertificateAuthority::new(Ed25519Identity::ephemeral());
    let der = authority.issuer_certificate_der().unwrap();
    let after = DateTime::<Utc>::from(SystemTime::now());
    assert!(before <= supplied && supplied <= after);
    let certificate = parsed(&der);
    assert!(certificate.validity().not_before.timestamp() >= before.timestamp());
    assert!(certificate.validity().not_before.timestamp() <= after.timestamp());
    assert_eq!(certificate.validity().not_after.timestamp(), ISSUER_END);
}

#[test]
fn unsigned_seconds_reject_pre_epoch_instants_and_preserve_whole_seconds() {
    assert!(matches!(
        unix_seconds(instant(-1)),
        Err(ClockError::InstantOutOfRange { .. })
    ));
    let just_before_epoch = instant(-1).with_nanosecond(999_999_999).unwrap();
    assert!(matches!(
        unix_seconds(just_before_epoch),
        Err(ClockError::InstantOutOfRange { .. })
    ));
    for supplied in [instant(-1), just_before_epoch] {
        let error = unix_seconds(supplied).unwrap_err();
        assert!(
            std::error::Error::source(&error)
                .is_some_and(<(dyn std::error::Error + 'static)>::is::<std::num::TryFromIntError>)
        );
    }
    assert_eq!(unix_seconds(instant(0)).unwrap(), 0);
    assert_eq!(unix_seconds(instant(FIRST)).unwrap(), 1_700_000_000);
    let fractional = instant(FIRST).with_nanosecond(999_999_999).unwrap();
    assert_eq!(unix_seconds(fractional).unwrap(), 1_700_000_000);
    let failing = SuppliedClock {
        fail_on: 1,
        ..SuppliedClock::at(FIRST)
    };
    assert!(matches!(
        failing.unix_seconds(),
        Err(ClockError::Unavailable { .. })
    ));
    assert_eq!(failing.reads(), 1);
}

#[test]
fn issuer_creation_obeys_supplied_time_and_detects_regeneration() {
    let first = Arc::new(SuppliedClock::at(FIRST));
    let later = Arc::new(SuppliedClock::at(FIRST + 2));
    let first_authority = supplied_authority(&first);
    let later_authority = supplied_authority(&later);
    let first_der = first_authority.issuer_certificate_der().unwrap();
    let later_der = later_authority.issuer_certificate_der().unwrap();
    let first_certificate = parsed(&first_der);
    let later_certificate = parsed(&later_der);
    assert_eq!(first_certificate.validity().not_before.timestamp(), FIRST);
    assert_eq!(
        later_certificate.validity().not_before.timestamp(),
        FIRST + 2
    );
    assert_eq!(
        first_certificate.validity().not_after.timestamp(),
        ISSUER_END
    );
    assert_eq!(
        later_certificate.validity().not_after.timestamp(),
        ISSUER_END
    );
    assert_eq!(
        first_certificate
            .public_key()
            .subject_public_key
            .data
            .as_ref(),
        later_certificate
            .public_key()
            .subject_public_key
            .data
            .as_ref()
    );
    assert_eq!(
        first_authority.public_key_bytes(),
        later_authority.public_key_bytes()
    );
    assert_ne!(first_der, later_der);
    assert_eq!(first.reads(), 1);
    assert_eq!(later.reads(), 1);
}

#[test]
fn advancing_one_owner_does_not_change_another_owners_creation_time() {
    let first = Arc::new(SuppliedClock::at(FIRST));
    let second = Arc::new(SuppliedClock::at(FIRST + 20));
    let first_authority = supplied_authority(&first);
    let second_authority = supplied_authority(&second);
    first.advance_to(FIRST + 4);
    let first_der = first_authority.issuer_certificate_der().unwrap();
    let second_der = second_authority.issuer_certificate_der().unwrap();
    assert_eq!(
        parsed(&first_der).validity().not_before.timestamp(),
        FIRST + 4
    );
    assert_eq!(
        parsed(&second_der).validity().not_before.timestamp(),
        FIRST + 20
    );
    assert_eq!(first.reads(), 1);
    assert_eq!(second.reads(), 1);
}

#[test]
fn parallel_authorities_keep_their_instance_clocks() {
    let ready = Arc::new(Barrier::new(3));
    let first = Arc::new(SuppliedClock::at(FIRST));
    let second = Arc::new(SuppliedClock::at(FIRST + 20));
    std::thread::scope(|scope| {
        let first_ready = Arc::clone(&ready);
        let first_authority = supplied_authority(&first);
        let first_child = scope.spawn(move || {
            first_ready.wait();
            first_authority.issuer_certificate_der().unwrap()
        });
        let second_ready = Arc::clone(&ready);
        let second_authority = supplied_authority(&second);
        let second_child = scope.spawn(move || {
            second_ready.wait();
            second_authority.issuer_certificate_der().unwrap()
        });
        first.advance_to(FIRST + 4);
        ready.wait();
        assert_eq!(
            parsed(&first_child.join().unwrap())
                .validity()
                .not_before
                .timestamp(),
            FIRST + 4
        );
        assert_eq!(
            parsed(&second_child.join().unwrap())
                .validity()
                .not_before
                .timestamp(),
            FIRST + 20
        );
    });
    assert_eq!(first.reads(), 1);
    assert_eq!(second.reads(), 1);
}

#[test]
fn generated_leaf_uses_supplied_time_and_preserves_der_and_reported_expiry() {
    let clock = Arc::new(SuppliedClock {
        nanos: 987_654_321,
        ..SuppliedClock::at(FIRST)
    });
    let authority = supplied_authority(&clock);
    let issued = authority
        .issue_certificate("generated", Duration::from_secs(61), vec![])
        .unwrap();
    let certificate = parsed(&issued.der_bytes);
    assert_eq!(certificate.validity().not_before.timestamp(), FIRST);
    assert_eq!(certificate.validity().not_after.timestamp(), FIRST + 61);
    assert_eq!(issued.expires_at, instant(FIRST + 61));
    assert_eq!(
        certificate.public_key().subject_public_key.data.as_ref(),
        issued.subject_verifying_key.as_bytes()
    );
    assert_eq!(clock.reads(), 2);
    verify_certificate_chain_at(
        &issued.der_bytes,
        &authority.public_key_bytes(),
        instant(FIRST),
    )
    .unwrap();
    verify_certificate_chain_at(
        &issued.der_bytes,
        &authority.public_key_bytes(),
        instant(FIRST + 61),
    )
    .unwrap();
    assert!(
        verify_certificate_chain_at(
            &issued.der_bytes,
            &authority.public_key_bytes(),
            instant(FIRST - 1)
        )
        .is_err()
    );
    assert!(
        verify_certificate_chain_at(
            &issued.der_bytes,
            &authority.public_key_bytes(),
            instant(FIRST + 62)
        )
        .is_err()
    );
}

#[test]
fn presented_leaf_uses_supplied_time_and_preserves_key_and_extensions() {
    let clock = Arc::new(SuppliedClock::at(FIRST + 7));
    let authority = supplied_authority(&clock);
    let subject = Arc::new(Ed25519Identity::ephemeral());
    let request = create_certificate_request(&subject, "presented").unwrap();
    let content = vec![0x04, 0x02, 0x12, 0x34];
    let extension =
        CustomExtension::from_oid_content(&[1, 3, 6, 1, 4, 1, 66364, 9], content.clone());
    let issued = authority
        .issue_certificate_for_request(
            &request,
            "presented",
            Duration::from_secs(60),
            vec![extension],
        )
        .unwrap();
    let certificate = parsed(&issued.der_bytes);
    assert_eq!(certificate.validity().not_before.timestamp(), FIRST + 7);
    assert_eq!(certificate.validity().not_after.timestamp(), FIRST + 67);
    assert_eq!(issued.expires_at, instant(FIRST + 67));
    assert_eq!(issued.subject_public_key, subject.public_key_bytes());
    assert_eq!(
        certificate.public_key().subject_public_key.data.as_ref(),
        subject.public_key_bytes()
    );
    let extension = certificate
        .extensions()
        .iter()
        .find(|extension| extension.oid.to_id_string() == "1.3.6.1.4.1.66364.9")
        .unwrap();
    assert_eq!(extension.value, content);
    assert!(!extension.critical);
    assert_eq!(clock.reads(), 2);
    verify_certificate_chain_at(
        &issued.der_bytes,
        &authority.public_key_bytes(),
        instant(FIRST + 8),
    )
    .unwrap();
}

#[test]
fn unavailable_creation_time_refuses_every_creation_path_without_fallback() {
    for path in 0..3 {
        let clock = Arc::new(SuppliedClock {
            fail_on: 1,
            ..SuppliedClock::at(FIRST)
        });
        let authority = supplied_authority(&clock);
        let error = match path {
            0 => authority.issuer_certificate_der().unwrap_err(),
            1 => authority
                .issue_certificate("generated", Duration::from_secs(60), vec![])
                .unwrap_err(),
            _ => {
                let request = create_certificate_request(
                    &Arc::new(Ed25519Identity::ephemeral()),
                    "presented",
                )
                .unwrap();
                authority
                    .issue_certificate_for_request(
                        &request,
                        "presented",
                        Duration::from_secs(60),
                        vec![],
                    )
                    .unwrap_err()
            }
        };
        clock_refusal(&error);
        assert_eq!(clock.reads(), 1);
    }
}

#[test]
fn a_failed_issuer_read_after_the_leaf_read_does_not_return_a_certificate() {
    for presented in [false, true] {
        let clock = Arc::new(SuppliedClock {
            fail_on: 2,
            ..SuppliedClock::at(FIRST)
        });
        let authority = supplied_authority(&clock);
        let error = if presented {
            let request =
                create_certificate_request(&Arc::new(Ed25519Identity::ephemeral()), "presented")
                    .unwrap();
            authority
                .issue_certificate_for_request(
                    &request,
                    "presented",
                    Duration::from_secs(60),
                    vec![],
                )
                .unwrap_err()
        } else {
            authority
                .issue_certificate("generated", Duration::from_secs(60), vec![])
                .unwrap_err()
        };
        clock_refusal(&error);
        assert_eq!(clock.reads(), 2);
    }
}

#[test]
fn invalid_subject_and_ttl_keep_their_refusals_before_reading_time() {
    let clock = Arc::new(SuppliedClock {
        fail_on: 1,
        ..SuppliedClock::at(FIRST)
    });
    let authority = supplied_authority(&clock);
    for (subject, ttl) in [
        (" ", Duration::from_secs(60)),
        ("generated", Duration::ZERO),
    ] {
        let error = authority
            .issue_certificate(subject, ttl, vec![])
            .unwrap_err();
        assert!(
            matches!(error, TrustError::CertificateGeneration { reason } if !reason.contains("clock unavailable"))
        );
    }
    assert_eq!(clock.reads(), 0);
}

#[test]
fn ttl_range_and_expiry_overflow_retain_named_generation_errors() {
    let clock = Arc::new(SuppliedClock::at(FIRST));
    let authority = supplied_authority(&clock);
    let error = authority
        .issue_certificate("generated", Duration::MAX, vec![])
        .unwrap_err();
    assert!(
        matches!(error, TrustError::CertificateGeneration { reason } if reason.contains("TTL is out of representable range"))
    );
    let clock = Arc::new(SuppliedClock::at(DateTime::<Utc>::MAX_UTC.timestamp()));
    let authority = supplied_authority(&clock);
    let error = authority
        .issue_certificate("generated", Duration::from_secs(60), vec![])
        .unwrap_err();
    assert!(
        matches!(error, TrustError::CertificateGeneration { reason } if reason.contains("expiry overflowed"))
    );
}

#[test]
fn supplied_dates_outside_der_range_keep_the_named_generation_error() {
    let clock = Arc::new(SuppliedClock::at(DateTime::<Utc>::MAX_UTC.timestamp()));
    let authority = supplied_authority(&clock);
    let error = authority.issuer_certificate_der().unwrap_err();
    assert!(
        matches!(error, TrustError::CertificateGeneration { reason } if reason.contains("validity instant is out of range"))
    );
    assert_eq!(clock.reads(), 1);
}

#[test]
fn creation_clock_does_not_replace_explicit_verification_time() {
    let clock = Arc::new(SuppliedClock {
        fail_on: 3,
        ..SuppliedClock::at(FIRST)
    });
    let authority = supplied_authority(&clock);
    let issued = authority
        .issue_certificate("generated", Duration::from_secs(60), vec![])
        .unwrap();
    clock.advance_to(FIRST + 100);
    verify_certificate_chain_at(
        &issued.der_bytes,
        &authority.public_key_bytes(),
        instant(FIRST + 1),
    )
    .unwrap();
    assert!(
        verify_certificate_chain_at(
            &issued.der_bytes,
            &authority.public_key_bytes(),
            instant(FIRST + 61)
        )
        .is_err()
    );
    assert_eq!(clock.reads(), 2);
    clock_refusal(&authority.issuer_certificate_der().unwrap_err());
}
