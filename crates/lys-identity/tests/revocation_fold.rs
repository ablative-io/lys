//! R3: the fold of the live set from a certificate log, refusing unauthorised
//! revocations, revocations before issuance and reinstatements by name.

mod revocation_support;

use std::collections::BTreeMap;
use std::error::Error;
use std::path::{Path, PathBuf};

use lys_core::ca::{LYS_OID_ARC, encode_extension};
use lys_identity::revocation::{
    CertificateHash, CertificateLeaf, LeafRefusal, LiveSet, RefusedLeaf, Revocation, fold,
};
use lys_log_store::{FileLeafStore, LeafStore, Log};
use revocation_support::{Issuer, TestResult, issue, new_issuer, open_log, seed};
use sha2::{Digest, Sha256};
use tempfile::TempDir;

/// Seven bytes no leaf decodes from.
const UNREADABLE: [u8; 7] = [9; 7];

/// The lowercase hex of `bytes`.
fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        out.push(char::from(DIGITS[usize::from(byte >> 4)]));
        out.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    out
}

/// An issuer, a second authority, and two certificates the issuer issued.
struct Fixture {
    dir: TempDir,
    issuer: Issuer,
    other: Issuer,
    a_der: Vec<u8>,
    b_der: Vec<u8>,
}

fn fixture() -> Result<Fixture, Box<dyn Error>> {
    let dir = TempDir::new()?;
    let issuer = new_issuer(dir.path(), "issuer", seed())?;
    let other = new_issuer(dir.path(), "other", seed())?;
    let a_der = issue(&issuer, "agent-a", vec![])?.der_bytes;
    let b_der = issue(&issuer, "agent-b", vec![])?.der_bytes;
    Ok(Fixture {
        dir,
        issuer,
        other,
        a_der,
        b_der,
    })
}

impl Fixture {
    fn a(&self) -> CertificateHash {
        CertificateHash::of_der(&self.a_der)
    }

    fn b(&self) -> CertificateHash {
        CertificateHash::of_der(&self.b_der)
    }

    fn log_path(&self, name: &str) -> PathBuf {
        self.dir.path().join(name)
    }

    /// A log at `name` holding `leaves`, appended in order.
    fn log(&self, name: &str, leaves: &[Vec<u8>]) -> Result<Log<FileLeafStore>, Box<dyn Error>> {
        let mut log = open_log(&self.log_path(name))?;
        for leaf in leaves {
            log.append(leaf)?;
        }
        Ok(log)
    }

    fn fold(&self, log: &Log<FileLeafStore>) -> Result<LiveSet, Box<dyn Error>> {
        Ok(fold(log.store(), &self.issuer.public_key())?)
    }

    /// Leg 1's log: issuance of A, issuance of B, a revocation of A and a
    /// verbatim copy of A's issuance leaf.
    fn last_wins_log(&self) -> Result<Log<FileLeafStore>, Box<dyn Error>> {
        let mut log = open_log(&self.log_path("last-wins"))?;
        let origin = log.origin().to_owned();
        let issuance_a = issuance(&self.a_der);
        let leaves = [
            issuance_a.clone(),
            issuance(&self.b_der),
            revocation(&origin, self.a(), &self.issuer),
            issuance_a,
        ];
        for leaf in &leaves {
            log.append(leaf)?;
        }
        Ok(log)
    }
}

/// A revocation leaf of `certificate` signed by `key` for `origin`.
fn revocation(origin: &str, certificate: CertificateHash, key: &Issuer) -> Vec<u8> {
    CertificateLeaf::Revocation(Revocation::sign(origin, certificate, &key.key)).encode()
}

fn issuance(der: &[u8]) -> Vec<u8> {
    CertificateLeaf::Issuance { der: der.to_vec() }.encode()
}

fn refused(index: u64, refusal: LeafRefusal) -> RefusedLeaf {
    RefusedLeaf { index, refusal }
}

/// The SHA-256 of every file under `dir`, by path.
fn file_hashes(dir: &Path) -> Result<BTreeMap<PathBuf, [u8; 32]>, Box<dyn Error>> {
    let mut hashes = BTreeMap::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(&directory)? {
            let path = entry?.path();
            if path.is_dir() {
                pending.push(path);
            } else {
                let digest: [u8; 32] = Sha256::digest(std::fs::read(&path)?).into();
                hashes.insert(path, digest);
            }
        }
    }
    Ok(hashes)
}

#[test]
fn d013_r3_ac1_a_replayed_issuance_leaf_never_wins_over_a_revocation() -> TestResult {
    let fixture = fixture()?;
    let log = fixture.last_wins_log()?;
    let set = fixture.fold(&log)?;
    assert_eq!(set.folded_size, 4);
    assert_eq!(
        set.issued,
        BTreeMap::from([(fixture.a(), 0), (fixture.b(), 1)])
    );
    assert_eq!(set.revoked, BTreeMap::from([(fixture.a(), 2)]));
    assert_eq!(
        set.refused,
        vec![refused(3, LeafRefusal::CertificateReinstatementRefused)]
    );
    assert!(!set.is_live(&fixture.a()));
    assert!(set.is_live(&fixture.b()));
    Ok(())
}

#[test]
fn d013_r3_ac2_revocations_by_another_key_or_for_another_origin_revoke_nothing() -> TestResult {
    let fixture = fixture()?;
    let origin = open_log(&fixture.log_path("origin-only"))?
        .origin()
        .to_owned();
    let log = fixture.log(
        "other-signer",
        &[
            issuance(&fixture.a_der),
            revocation(&origin, fixture.a(), &fixture.other),
            revocation("example.test/lys/another-log", fixture.a(), &fixture.issuer),
        ],
    )?;
    let set = fixture.fold(&log)?;
    assert_eq!(set.folded_size, 3);
    assert!(set.revoked.is_empty());
    assert!(set.is_live(&fixture.a()));
    assert_eq!(
        set.refused,
        vec![
            refused(1, LeafRefusal::RevocationNotSignedByIssuer),
            refused(2, LeafRefusal::RevocationNotSignedByIssuer),
        ]
    );
    Ok(())
}

#[test]
fn d013_r3_ac3_repeats_change_nothing_and_an_unreadable_leaf_is_recorded() -> TestResult {
    let fixture = fixture()?;
    let origin = open_log(&fixture.log_path("origin-only"))?
        .origin()
        .to_owned();
    let revocation = revocation(&origin, fixture.a(), &fixture.issuer);
    let log = fixture.log(
        "repeats",
        &[
            issuance(&fixture.a_der),
            issuance(&fixture.a_der),
            revocation.clone(),
            revocation,
            UNREADABLE.to_vec(),
        ],
    )?;
    let set = fixture.fold(&log)?;
    assert_eq!(set.revoked, BTreeMap::from([(fixture.a(), 2)]));
    assert_eq!(set.issued, BTreeMap::from([(fixture.a(), 0)]));
    assert!(
        set.refused_as(LeafRefusal::CertificateReinstatementRefused)
            .is_empty()
    );
    assert_eq!(
        set.refused,
        vec![refused(4, LeafRefusal::FoldUnreadableLeaf)]
    );
    assert_eq!(set.unreadable_leaf(), Some(4));
    assert_eq!(set.folded_size, 5);
    Ok(())
}

#[test]
fn d013_r3_ac4_a_certificate_with_two_claims_is_revoked_whole() -> TestResult {
    let fixture = fixture()?;
    let first: Vec<u64> = LYS_OID_ARC.iter().copied().chain([101]).collect();
    let second: Vec<u64> = LYS_OID_ARC.iter().copied().chain([102]).collect();
    let der = issue(
        &fixture.issuer,
        "agent-two-claims",
        vec![
            encode_extension(&first, b"the first claim".to_vec()),
            encode_extension(&second, b"the second claim".to_vec()),
        ],
    )?
    .der_bytes;
    let origin = open_log(&fixture.log_path("origin-only"))?
        .origin()
        .to_owned();
    let log = fixture.log(
        "two-claims",
        &[
            issuance(&der),
            revocation(&origin, CertificateHash::of_der(&der), &fixture.issuer),
        ],
    )?;
    let set = fixture.fold(&log)?;
    assert_eq!(set.revoked.len(), 1);
    assert_eq!(set.revoked.get(&CertificateHash::of_der(&der)), Some(&1));
    assert!(set.refused.is_empty());
    assert!(!set.is_live(&CertificateHash::of_der(&der)));
    Ok(())
}

#[test]
fn d013_r3_ac5_three_folds_write_nothing() -> TestResult {
    let fixture = fixture()?;
    let log = fixture.last_wins_log()?;
    let path = fixture.log_path("last-wins");
    let (extent, pinned, files) = (
        log.store().extent(),
        log.store().pinned(),
        file_hashes(&path)?,
    );
    assert!(!files.is_empty());
    let mut folds = 0;
    for _ in 0..3 {
        let set = fixture.fold(&log)?;
        assert_eq!(set.folded_size, 4);
        folds += 1;
    }
    assert_eq!(folds, 3);
    let reopened = FileLeafStore::open(&path)?;
    assert_eq!(reopened.extent(), extent);
    assert_eq!(reopened.pinned(), pinned);
    assert_eq!(file_hashes(&path)?, files);
    Ok(())
}

#[test]
fn d013_r3_ac6_a_revocation_before_issuance_revokes_nothing() -> TestResult {
    let fixture = fixture()?;
    let origin = open_log(&fixture.log_path("origin-only"))?
        .origin()
        .to_owned();
    let log = fixture.log(
        "before-issuance",
        &[
            revocation(&origin, fixture.a(), &fixture.issuer),
            issuance(&fixture.a_der),
        ],
    )?;
    let set = fixture.fold(&log)?;
    assert_eq!(set.folded_size, 2);
    assert_eq!(set.issued, BTreeMap::from([(fixture.a(), 1)]));
    assert!(set.revoked.is_empty());
    assert!(
        set.refused_as(LeafRefusal::CertificateReinstatementRefused)
            .is_empty()
    );
    assert_eq!(
        set.refused,
        vec![refused(0, LeafRefusal::RevocationBeforeIssuance)]
    );
    assert!(set.is_live(&fixture.a()));
    Ok(())
}

#[test]
fn d013_r3_ac7_a_second_authoritys_issuance_is_chain_invalid() -> TestResult {
    let fixture = fixture()?;
    let c_der = issue(&fixture.other, "agent-c", vec![])?.der_bytes;
    let c = CertificateHash::of_der(&c_der);
    let origin = open_log(&fixture.log_path("origin-only"))?
        .origin()
        .to_owned();
    let log = fixture.log(
        "second-authority",
        &[
            issuance(&c_der),
            revocation(&origin, c, &fixture.issuer),
            issuance(&fixture.a_der),
        ],
    )?;
    let set = fixture.fold(&log)?;
    assert_eq!(set.folded_size, 3);
    assert_eq!(set.issued, BTreeMap::from([(fixture.a(), 2)]));
    assert!(!set.issued.contains_key(&c));
    assert!(set.revoked.is_empty());
    assert_eq!(
        set.refused,
        vec![
            refused(0, LeafRefusal::CertificateChainInvalid),
            refused(1, LeafRefusal::RevocationBeforeIssuance),
        ]
    );
    Ok(())
}

#[test]
fn d013_r3_ac8_a_subject_equal_to_the_issuers_hex_key_is_chain_invalid() -> TestResult {
    let fixture = fixture()?;
    let e_der = issue(&fixture.issuer, &hex(&fixture.issuer.public_key()), vec![])?.der_bytes;
    let e = CertificateHash::of_der(&e_der);
    let log = fixture.log(
        "self-signed-screen",
        &[issuance(&e_der), issuance(&fixture.a_der)],
    )?;
    let set = fixture.fold(&log)?;
    assert_eq!(set.folded_size, 2);
    assert_eq!(set.issued, BTreeMap::from([(fixture.a(), 1)]));
    assert!(!set.issued.contains_key(&e));
    assert!(set.revoked.is_empty());
    assert_eq!(
        set.refused,
        vec![refused(0, LeafRefusal::CertificateChainInvalid)]
    );
    Ok(())
}
