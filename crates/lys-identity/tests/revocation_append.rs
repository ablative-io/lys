//! R4: issuance, revocation and attestation-entry leaves are appended only as
//! new leaves at the log's extent, and a refused write is returned with its
//! operation and index.

mod revocation_support;

use std::error::Error;
use std::path::PathBuf;

use lys_core::attestation::sign_attestation;
use lys_core::merkle::{AppendOnlyTree, RawLeaf};
use lys_identity::revocation::{
    AppendOperation, CertificateHash, RevocationError, append_attestation_entry, append_issuance,
    append_revocation, fold,
};
use lys_log_store::{FileLeafStore, LeafStore, Log, PinnedRoot, StoreError, StoreResult};
use revocation_support::fixtures::identity;
use revocation_support::{Issuer, TestResult, issue, new_issuer, open_log, seed};
use tempfile::TempDir;

/// An issuer, two certificates it issued and an attestation by B's key.
struct Fixture {
    dir: TempDir,
    issuer: Issuer,
    a_der: Vec<u8>,
    b_der: Vec<u8>,
    b_cose: Vec<u8>,
}

fn fixture() -> Result<Fixture, Box<dyn Error>> {
    let dir = TempDir::new()?;
    let issuer = new_issuer(dir.path(), "issuer", seed())?;
    let a = issue(&issuer, "agent-a", vec![])?;
    let b = issue(&issuer, "agent-b", vec![])?;
    let b_key = identity(dir.path(), "agent-b", b.subject_signing_key.to_bytes())?;
    let b_cose = sign_attestation(b"a payload B attests to", &b_key).to_cose_bytes();
    Ok(Fixture {
        dir,
        issuer,
        a_der: a.der_bytes,
        b_der: b.der_bytes,
        b_cose,
    })
}

/// The leaves of leg 1's log after they were appended, and where it lives.
struct Appended {
    path: PathBuf,
    indexes: [u64; 4],
    before_revocation: [Vec<u8>; 2],
    leaves: Vec<Vec<u8>>,
}

impl Fixture {
    fn a(&self) -> CertificateHash {
        CertificateHash::of_der(&self.a_der)
    }

    fn b(&self) -> CertificateHash {
        CertificateHash::of_der(&self.b_der)
    }

    /// Leg 1: issuance of A, issuance of B, a revocation of A and an
    /// attestation entry for B on a fresh file-backed log.
    fn append_all(&self) -> Result<Appended, Box<dyn Error>> {
        let path = self.dir.path().join("certificates");
        let mut log = open_log(&path)?;
        let first = append_issuance(&mut log, &self.a_der)?;
        let second = append_issuance(&mut log, &self.b_der)?;
        let before_revocation = [read(log.store(), 0)?, read(log.store(), 1)?];
        let third = append_revocation(&mut log, self.a(), &self.issuer.key)?;
        let fourth = append_attestation_entry(&mut log, self.b(), &self.b_cose)?;
        let leaves = (0..log.store().extent())
            .map(|index| read(log.store(), index))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Appended {
            path,
            indexes: [first, second, third, fourth],
            before_revocation,
            leaves,
        })
    }
}

fn read(store: &FileLeafStore, index: u64) -> Result<Vec<u8>, Box<dyn Error>> {
    Ok(store
        .leaf(index)?
        .ok_or_else(|| format!("no leaf at index {index}"))?)
}

#[test]
fn d013_r4_ac1_four_appends_land_at_the_extent_and_rewrite_nothing() -> TestResult {
    let fixture = fixture()?;
    let appended = fixture.append_all()?;
    assert_eq!(appended.indexes, [0, 1, 2, 3]);
    let store = FileLeafStore::open(&appended.path)?;
    assert_eq!(store.extent(), 4);
    assert_eq!(read(&store, 0)?, appended.before_revocation[0]);
    assert_eq!(read(&store, 1)?, appended.before_revocation[1]);
    assert_eq!(appended.leaves.len(), 4);
    Ok(())
}

#[test]
fn d013_r4_ac2_the_reopened_log_folds_and_reads_byte_identical_leaves() -> TestResult {
    let fixture = fixture()?;
    let appended = fixture.append_all()?;
    let reopened = Log::open(FileLeafStore::open(&appended.path)?)?;
    let set = fold(reopened.store(), &fixture.issuer.public_key())?;
    assert_eq!(set.folded_size, 4);
    assert_eq!(set.revoked.get(&fixture.a()), Some(&2));
    assert!(!set.is_live(&fixture.a()));
    assert!(set.is_live(&fixture.b()));
    assert!(set.refused.is_empty());
    let mut compared = 0;
    for (index, before) in appended.leaves.iter().enumerate() {
        let index = u64::try_from(index)?;
        assert_eq!(&read(reopened.store(), index)?, before, "leaf {index}");
        assert_eq!(reopened.leaf_bytes(index), Some(before.as_slice()));
        compared += 1;
    }
    assert_eq!(compared, 4);
    Ok(())
}

/// A store whose write at index 1 fails with an I/O error; every other part of
/// the contract is honoured.
struct FailingStore {
    origin: String,
    leaves: Vec<Vec<u8>>,
    pinned: PinnedRoot,
    snapshot: Option<Vec<u8>>,
}

impl FailingStore {
    fn new() -> Self {
        let (root, tree_size) = AppendOnlyTree::<RawLeaf>::new().root().to_parts();
        Self {
            origin: "example.test/lys/failing".to_owned(),
            leaves: Vec::new(),
            pinned: PinnedRoot { tree_size, root },
            snapshot: None,
        }
    }
}

impl LeafStore for FailingStore {
    fn origin(&self) -> &str {
        &self.origin
    }

    fn extent(&self) -> u64 {
        u64::try_from(self.leaves.len()).unwrap_or(u64::MAX)
    }

    fn leaf(&self, index: u64) -> StoreResult<Option<Vec<u8>>> {
        Ok(usize::try_from(index)
            .ok()
            .and_then(|index| self.leaves.get(index).cloned()))
    }

    fn append(&mut self, index: u64, leaves: &[&[u8]], pin: PinnedRoot) -> StoreResult<()> {
        if index == 1 {
            return Err(StoreError::Io {
                context: "writing leaf 1 to the failing store".to_owned(),
                source: std::io::Error::other("the device is full"),
            });
        }
        assert_eq!(index, self.extent(), "a write lands only at the extent");
        self.leaves.extend(leaves.iter().map(|bytes| bytes.to_vec()));
        self.pinned = pin;
        Ok(())
    }

    fn pinned(&self) -> PinnedRoot {
        self.pinned
    }

    fn pin(&mut self, pin: PinnedRoot) -> StoreResult<()> {
        self.pinned = pin;
        Ok(())
    }

    fn snapshot(&self) -> StoreResult<Option<Vec<u8>>> {
        Ok(self.snapshot.clone())
    }

    fn put_snapshot(&mut self, bytes: &[u8]) -> StoreResult<()> {
        self.snapshot = Some(bytes.to_vec());
        Ok(())
    }
}

#[test]
fn d013_r4_ac3_a_refused_write_names_the_operation_and_index_and_is_not_written() -> TestResult {
    let fixture = fixture()?;
    let mut log = Log::open(FailingStore::new())?;
    assert_eq!(append_issuance(&mut log, &fixture.a_der)?, 0);
    let refusal = append_revocation(&mut log, fixture.a(), &fixture.issuer.key)
        .err()
        .ok_or("the failing write was reported as written")?;
    match &refusal {
        RevocationError::StoreWriteFailed {
            operation,
            index,
            reason,
        } => {
            assert_eq!(*operation, AppendOperation::Revocation);
            assert_eq!(*index, 1);
            assert!(reason.contains("the device is full"), "{reason}");
        }
        other => panic!("refused with another name: {other}"),
    }
    assert!(refusal.to_string().starts_with("store_write_failed: "));
    assert_eq!(log.store().extent(), 1);
    assert_eq!(log.tree().len(), 1);
    Ok(())
}
