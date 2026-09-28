//! R5: verification against the log with N and the tolerance as inputs, and a
//! revoked certificate refused naming its revocation leaf.

mod revocation_support;

use std::error::Error;

use chrono::{DateTime, Duration, Utc};
use lys_core::ca::{IssuedCertificate, verify_certificate_chain, verify_certificate_chain_at};
use lys_identity::revocation::{
    CertificateHash, CertificateLeaf, Evidence, Live, Revocation, RevocationError,
    append_revocation, verify_certificate,
};
use lys_log_store::{FileLeafStore, Log};
use revocation_support::{Issuer, TestResult, issue, new_issuer, open_log, seed};
use tempfile::TempDir;

/// Seven bytes no leaf decodes from.
const UNREADABLE: [u8; 7] = [9; 7];

/// An instant half an hour before the certificate's notAfter, inside its
/// one-hour window.
fn inside(cert: &IssuedCertificate) -> DateTime<Utc> {
    cert.expires_at - Duration::minutes(30)
}

fn evidence(n: u64, tolerance: u64) -> Evidence {
    Evidence { n, tolerance }
}

/// An issuer, a second authority, and certificates A, B and D the issuer issued.
struct Fixture {
    dir: TempDir,
    issuer: Issuer,
    other: Issuer,
    a: IssuedCertificate,
    b: IssuedCertificate,
    d: IssuedCertificate,
    logs: std::cell::Cell<u32>,
}

fn fixture() -> Result<Fixture, Box<dyn Error>> {
    let dir = TempDir::new()?;
    let issuer = new_issuer(dir.path(), "issuer", seed())?;
    let other = new_issuer(dir.path(), "other", seed())?;
    let a = issue(&issuer, "agent-a", vec![])?;
    let b = issue(&issuer, "agent-b", vec![])?;
    let d = issue(&issuer, "agent-d", vec![])?;
    Ok(Fixture {
        dir,
        issuer,
        other,
        a,
        b,
        d,
        logs: std::cell::Cell::new(0),
    })
}

impl Fixture {
    fn key(&self) -> [u8; 32] {
        self.issuer.public_key()
    }

    /// A fresh log holding `leaves`, appended in order.
    fn log(&self, leaves: &[Vec<u8>]) -> Result<Log<FileLeafStore>, Box<dyn Error>> {
        let number = self.logs.get();
        self.logs.set(number + 1);
        let mut log = open_log(&self.dir.path().join(format!("log-{number}")))?;
        for leaf in leaves {
            log.append(leaf)?;
        }
        Ok(log)
    }

    /// A revocation of `cert` signed by `key` for the origin every log here has.
    fn revocation(
        &self,
        cert: &IssuedCertificate,
        key: &Issuer,
    ) -> Result<Vec<u8>, Box<dyn Error>> {
        let origin = self.log(&[])?.origin().to_owned();
        let revocation =
            Revocation::sign(&origin, CertificateHash::of_der(&cert.der_bytes), &key.key);
        Ok(CertificateLeaf::Revocation(revocation).encode())
    }

    fn verify(
        &self,
        log: &Log<FileLeafStore>,
        cert: &IssuedCertificate,
        at: DateTime<Utc>,
        evidence: Evidence,
    ) -> Result<Live, RevocationError> {
        verify_certificate(log.store(), &self.key(), &cert.der_bytes, at, evidence)
    }

    /// Leg 1's log: issuance of A at 0 and a valid revocation of A at 1.
    fn revoked_a(&self) -> Result<Log<FileLeafStore>, Box<dyn Error>> {
        let mut log = self.log(&[issuance(&self.a)])?;
        let index = append_revocation(
            &mut log,
            CertificateHash::of_der(&self.a.der_bytes),
            &self.issuer.key,
        )?;
        assert_eq!(index, 1);
        Ok(log)
    }
}

fn issuance(cert: &IssuedCertificate) -> Vec<u8> {
    CertificateLeaf::Issuance {
        der: cert.der_bytes.clone(),
    }
    .encode()
}

#[test]
fn d013_r5_ac1_a_live_certificate_is_refused_once_its_revocation_is_appended() -> TestResult {
    let fixture = fixture()?;
    let mut log = fixture.log(&[issuance(&fixture.a)])?;
    let live = fixture.verify(&log, &fixture.a, inside(&fixture.a), evidence(1, 0))?;
    assert_eq!(live.folded_size, 1);
    assert_eq!(live.n, 1);
    assert_eq!(live.issuance_index, 0);
    let index = append_revocation(
        &mut log,
        CertificateHash::of_der(&fixture.a.der_bytes),
        &fixture.issuer.key,
    )?;
    assert_eq!(index, 1);
    let refusal = fixture.verify(&log, &fixture.a, inside(&fixture.a), evidence(2, 0));
    assert_eq!(
        refusal,
        Err(RevocationError::CertificateRevoked {
            index: 1,
            folded_size: 2,
        })
    );
    Ok(())
}

#[test]
fn d013_r5_ac2_stale_below_the_tolerance_live_within_it_and_revoked_whatever_n() -> TestResult {
    let fixture = fixture()?;
    let (mut stale, mut live, mut revoked) = (0, 0, 0);
    let three = fixture.log(&[
        issuance(&fixture.a),
        issuance(&fixture.b),
        issuance(&fixture.d),
    ])?;
    match fixture.verify(&three, &fixture.a, inside(&fixture.a), evidence(5, 1)) {
        Err(RevocationError::FoldStale {
            folded_size: 3,
            n: 5,
            tolerance: 1,
        }) => stale += 1,
        other => panic!("tolerance 1 answered {other:?}"),
    }
    match fixture.verify(&three, &fixture.a, inside(&fixture.a), evidence(5, 2)) {
        Ok(Live {
            folded_size: 3,
            n: 5,
            ..
        }) => live += 1,
        other => panic!("tolerance 2 answered {other:?}"),
    }
    let revoked_log = fixture.log(&[
        issuance(&fixture.a),
        issuance(&fixture.b),
        fixture.revocation(&fixture.a, &fixture.issuer)?,
    ])?;
    match fixture.verify(
        &revoked_log,
        &fixture.a,
        inside(&fixture.a),
        evidence(50, 0),
    ) {
        Err(RevocationError::CertificateRevoked {
            index: 2,
            folded_size: 3,
        }) => revoked += 1,
        other => panic!("the revoked certificate at N 50 answered {other:?}"),
    }
    assert_eq!((stale, live, revoked), (1, 1, 1));
    Ok(())
}

#[test]
fn d013_r5_ac3_unreadable_not_in_log_and_a_revocation_by_another_key() -> TestResult {
    let fixture = fixture()?;
    let (mut refusals, mut live) = (0, 0);
    let unreadable = fixture.log(&[issuance(&fixture.a), UNREADABLE.to_vec()])?;
    match fixture.verify(&unreadable, &fixture.a, inside(&fixture.a), evidence(2, 0)) {
        Err(RevocationError::FoldUnreadableLeaf {
            index: 1,
            folded_size: 2,
        }) => refusals += 1,
        other => panic!("the unreadable log answered {other:?}"),
    }
    let without_a = fixture.log(&[issuance(&fixture.b), issuance(&fixture.d)])?;
    match fixture.verify(&without_a, &fixture.a, inside(&fixture.a), evidence(2, 0)) {
        Err(RevocationError::CertificateNotInLog { folded_size: 2 }) => refusals += 1,
        other => panic!("the log without A answered {other:?}"),
    }
    let other_key = fixture.log(&[
        issuance(&fixture.a),
        fixture.revocation(&fixture.a, &fixture.other)?,
    ])?;
    match fixture.verify(&other_key, &fixture.a, inside(&fixture.a), evidence(2, 0)) {
        Ok(Live {
            folded_size: 2,
            n: 2,
            ..
        }) => live += 1,
        other => panic!("the other-key revocation answered {other:?}"),
    }
    assert_eq!((refusals, live), (2, 1));
    Ok(())
}

#[test]
fn d013_r5_ac4_chain_invalid_comes_before_any_fold_answer() -> TestResult {
    let fixture = fixture()?;
    let (mut chain_invalid, mut revoked) = (0, 0);
    let revoked_a = fixture.revoked_a()?;
    let after = fixture.a.expires_at + Duration::seconds(1);
    match fixture.verify(&revoked_a, &fixture.a, after, evidence(2, 0)) {
        Err(RevocationError::CertificateChainInvalid { .. }) => chain_invalid += 1,
        Err(RevocationError::CertificateRevoked { .. }) => revoked += 1,
        other => panic!("the expired revoked certificate answered {other:?}"),
    }
    let c = issue(&fixture.other, "agent-c", vec![])?;
    let with_c = fixture.log(&[issuance(&fixture.a), issuance(&c)])?;
    match fixture.verify(&with_c, &c, inside(&c), evidence(2, 0)) {
        Err(RevocationError::CertificateChainInvalid { .. }) => chain_invalid += 1,
        Err(RevocationError::CertificateRevoked { .. }) => revoked += 1,
        other => panic!("the second authority's certificate answered {other:?}"),
    }
    assert_eq!((chain_invalid, revoked), (2, 0));
    Ok(())
}

#[test]
fn d013_r5_ac5_the_no_log_forms_keep_passing_a_revoked_certificate() -> TestResult {
    let fixture = fixture()?;
    let revoked_a = fixture.revoked_a()?;
    assert_eq!(
        fixture.verify(&revoked_a, &fixture.a, inside(&fixture.a), evidence(2, 0)),
        Err(RevocationError::CertificateRevoked {
            index: 1,
            folded_size: 2,
        })
    );
    verify_certificate_chain_at(&fixture.a.der_bytes, &fixture.key(), inside(&fixture.a))?;
    verify_certificate_chain(&fixture.a.der_bytes, &fixture.key())?;
    Ok(())
}
