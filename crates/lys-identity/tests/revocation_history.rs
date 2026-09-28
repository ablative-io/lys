//! R6: a revoked certificate's history keeps verifying: its proofs, its
//! issuance record, and the attestations logged before its revocation.

mod revocation_support;

use std::error::Error;

use chrono::Duration;
use lys_core::Ed25519Identity;
use lys_core::attestation::sign_attestation;
use lys_core::ca::{IssuedCertificate, verify_certificate_chain_at};
use lys_core::merkle::{verify_consistency, verify_inclusion_raw};
use lys_identity::revocation::{
    AttestationStanding, CertificateHash, CertificateLeaf, Evidence, Revocation, RevocationError,
    verify_attestation_against_log,
};
use lys_log_store::{FileLeafStore, Log};
use revocation_support::fixtures::identity;
use revocation_support::{Issuer, TestResult, issue, new_issuer, open_log, seed};
use tempfile::TempDir;

/// Seven bytes no leaf decodes from.
const UNREADABLE: [u8; 7] = [9; 7];

const P1: &[u8] = b"the first payload";
const P2: &[u8] = b"the second payload";
const P3: &[u8] = b"the third payload";

/// An issuer, certificates A and B and the subject identities that attest.
struct Fixture {
    dir: TempDir,
    issuer: Issuer,
    a: IssuedCertificate,
    b: IssuedCertificate,
    a_key: Ed25519Identity,
    b_key: Ed25519Identity,
}

fn fixture() -> Result<Fixture, Box<dyn Error>> {
    let dir = TempDir::new()?;
    let issuer = new_issuer(dir.path(), "issuer", seed())?;
    let a = issue(&issuer, "agent-a", vec![])?;
    let b = issue(&issuer, "agent-b", vec![])?;
    let a_key = identity(dir.path(), "agent-a", a.subject_signing_key.to_bytes())?;
    let b_key = identity(dir.path(), "agent-b", b.subject_signing_key.to_bytes())?;
    Ok(Fixture {
        dir,
        issuer,
        a,
        b,
        a_key,
        b_key,
    })
}

fn hash(cert: &IssuedCertificate) -> CertificateHash {
    CertificateHash::of_der(&cert.der_bytes)
}

fn issuance(cert: &IssuedCertificate) -> Vec<u8> {
    CertificateLeaf::Issuance {
        der: cert.der_bytes.clone(),
    }
    .encode()
}

fn entry(cert: &IssuedCertificate, cose: &[u8]) -> Vec<u8> {
    CertificateLeaf::AttestationEntry {
        certificate: hash(cert),
        cose: cose.to_vec(),
    }
    .encode()
}

fn evidence(n: u64) -> Evidence {
    Evidence { n, tolerance: 0 }
}

/// The COSE bytes of an attestation over `payload` by `key`.
fn attest(key: &Ed25519Identity, payload: &[u8]) -> Vec<u8> {
    sign_attestation(payload, key).to_cose_bytes()
}

/// Leg 1's log and the two attestations by A's key it places.
struct History {
    log: Log<FileLeafStore>,
    p1: Vec<u8>,
    p2: Vec<u8>,
}

impl Fixture {
    /// A log at `name` holding `leaves`; a revocation of A is signed for it
    /// where `leaves` has `None`.
    fn log(
        &self,
        name: &str,
        leaves: &[Option<Vec<u8>>],
    ) -> Result<Log<FileLeafStore>, Box<dyn Error>> {
        let mut log = open_log(&self.dir.path().join(name))?;
        let origin = log.origin().to_owned();
        for leaf in leaves {
            match leaf {
                Some(bytes) => log.append(bytes)?,
                None => log.append(
                    &CertificateLeaf::Revocation(Revocation::sign(
                        &origin,
                        hash(&self.a),
                        &self.issuer.key,
                    ))
                    .encode(),
                )?,
            };
        }
        Ok(log)
    }

    /// Leg 1's log: issuance of A, an entry for P1, a revocation of A, an
    /// entry for P2 and issuance of B.
    fn history(&self) -> Result<History, Box<dyn Error>> {
        let p1 = attest(&self.a_key, P1);
        let p2 = attest(&self.a_key, P2);
        let log = self.log(
            "history",
            &[
                Some(issuance(&self.a)),
                Some(entry(&self.a, &p1)),
                None,
                Some(entry(&self.a, &p2)),
                Some(issuance(&self.b)),
            ],
        )?;
        Ok(History { log, p1, p2 })
    }

    fn check(
        &self,
        log: &Log<FileLeafStore>,
        cert: &IssuedCertificate,
        cose: &[u8],
        payload: &[u8],
        n: u64,
    ) -> Result<AttestationStanding, RevocationError> {
        verify_attestation_against_log(
            log.store(),
            &self.issuer.public_key(),
            &cert.der_bytes,
            cose,
            payload,
            evidence(n),
        )
    }
}

#[test]
fn d013_r6_ac1_only_an_entry_before_the_revocation_verifies() -> TestResult {
    let fixture = fixture()?;
    let History { log, p1, p2 } = fixture.history()?;
    let (mut verified, mut refused) = (0, 0);
    match fixture.check(&log, &fixture.a, &p1, P1, 5) {
        Ok(AttestationStanding::Verified {
            entry_index: 1,
            revocation_index: 2,
            folded_size: 5,
        }) => verified += 1,
        other => panic!("P1 answered {other:?}"),
    }
    match fixture.check(&log, &fixture.a, &p2, P2, 5) {
        Err(RevocationError::AttestationAfterRevocation {
            revocation_index: 2,
            folded_size: 5,
        }) => refused += 1,
        other => panic!("P2 answered {other:?}"),
    }
    let p3 = attest(&fixture.a_key, P3);
    match fixture.check(&log, &fixture.a, &p3, P3, 5) {
        Err(RevocationError::AttestationAfterRevocation {
            revocation_index: 2,
            folded_size: 5,
        }) => refused += 1,
        other => panic!("P3 answered {other:?}"),
    }
    assert_eq!((verified, refused), (1, 2));
    Ok(())
}

#[test]
fn d013_r6_ac2_inclusion_and_consistency_proofs_verify_after_the_revocation() -> TestResult {
    let fixture = fixture()?;
    let History { log, .. } = fixture.history()?;
    let tree = log.tree();
    assert_eq!(tree.len(), 5);
    let root = tree.root();
    let mut proofs = 0;
    for index in [0u64, 2] {
        let leaf = log
            .leaf_bytes(index)
            .ok_or_else(|| format!("no leaf at index {index}"))?;
        verify_inclusion_raw(&root, leaf, index, &tree.prove_inclusion(index)?)?;
        proofs += 1;
    }
    let old_root = log.prefix_tree(2)?.root();
    verify_consistency(&old_root, &root, &tree.prove_consistency(2, 5)?)?;
    proofs += 1;
    assert_eq!(proofs, 3);
    Ok(())
}

#[test]
fn d013_r6_ac3_the_issuance_record_still_verifies() -> TestResult {
    let fixture = fixture()?;
    let History { log, .. } = fixture.history()?;
    let leaf = log.leaf_bytes(0).ok_or("no leaf at index 0")?;
    let CertificateLeaf::Issuance { der } = CertificateLeaf::decode(leaf)? else {
        panic!("leaf 0 is not an issuance leaf");
    };
    assert_eq!(CertificateHash::of_der(&der), hash(&fixture.a));
    verify_certificate_chain_at(
        &der,
        &fixture.issuer.public_key(),
        fixture.a.expires_at - Duration::minutes(30),
    )?;
    Ok(())
}

#[test]
fn d013_r6_ac4_another_key_is_refused_and_an_unrevoked_key_needs_no_entry() -> TestResult {
    let fixture = fixture()?;
    let History { log, .. } = fixture.history()?;
    let by_b = attest(&fixture.b_key, P1);
    assert_eq!(
        fixture.check(&log, &fixture.a, &by_b, P1, 5),
        Err(RevocationError::AttestationSignatureInvalid)
    );
    assert_eq!(
        fixture.check(&log, &fixture.b, &by_b, P1, 5),
        Ok(AttestationStanding::NotRevoked { folded_size: 5 })
    );
    Ok(())
}

#[test]
fn d013_r6_ac5_an_unreadable_leaf_below_the_revocation_blocks_the_verified_answer() -> TestResult {
    let fixture = fixture()?;
    let p1 = attest(&fixture.a_key, P1);
    let log = fixture.log(
        "unreadable",
        &[
            Some(issuance(&fixture.a)),
            Some(entry(&fixture.a, &p1)),
            Some(UNREADABLE.to_vec()),
            None,
        ],
    )?;
    let (mut unreadable, mut verified) = (0, 0);
    match fixture.check(&log, &fixture.a, &p1, P1, 4) {
        Err(RevocationError::FoldUnreadableLeaf {
            index: 2,
            folded_size: 4,
        }) => unreadable += 1,
        Ok(AttestationStanding::Verified { .. }) => verified += 1,
        other => panic!("the unreadable log answered {other:?}"),
    }
    assert_eq!((unreadable, verified), (1, 0));
    Ok(())
}
