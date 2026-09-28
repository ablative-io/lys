#![cfg(test)]
//! A receipt is rebuilt from the root its leaf completed, kept beside the
//! log, and the one leaf read back: of a 5 000-leaf log, the receipt of leaf
//! 4 000 reads one leaf where the nearest checkpoint reads 929. The receipts
//! are the ones each append answered, byte for byte, and a roots file
//! removed from the disk is rebuilt from the log on open and answers the
//! same receipts. Every read is counted through the store, never timed.

use std::error::Error;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use lys_core::Ed25519Identity;
use lys_identity::log::Reopen;
use lys_identity::receipt::Receipt;
use lys_identity::{
    Actor, AuthMethod, Directory, LoginBinding, OperationId, Profile, Provenance, SNAPSHOT_EVERY,
};
use lys_log_store::{Frontier, LeafStore, PinnedRoot, StoreError, StoreResult};

type TestResult = Result<(), Box<dyn Error>>;

const ORIGIN: &str = "example.test/lys/directory";

/// How many leaves the log holds.
const LEAVES: u64 = 5_000;

/// The leaf whose receipt is read.
const READ: u64 = 4_000;

/// The durable parts of a store held in memory, shared by every handle
/// opened over them.
#[derive(Default)]
struct Disk {
    leaves: Vec<Vec<u8>>,
    pinned: Option<PinnedRoot>,
    snapshot: Option<Vec<u8>>,
}

/// A store in memory that counts every leaf read through it.
struct Memory {
    disk: Arc<Mutex<Disk>>,
    reads: Arc<AtomicU64>,
}

impl Memory {
    fn disk(&self) -> MutexGuard<'_, Disk> {
        self.disk.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl LeafStore for Memory {
    fn origin(&self) -> &str {
        ORIGIN
    }

    fn extent(&self) -> u64 {
        u64::try_from(self.disk().leaves.len()).unwrap_or(u64::MAX)
    }

    fn leaf(&self, index: u64) -> StoreResult<Option<Vec<u8>>> {
        self.reads.fetch_add(1, Ordering::SeqCst);
        let held = usize::try_from(index)
            .ok()
            .and_then(|at| self.disk().leaves.get(at).cloned());
        Ok(held)
    }

    fn put_leaf(&mut self, index: u64, bytes: &[u8]) -> StoreResult<()> {
        let next = self.extent();
        if index < next {
            return Err(StoreError::LeafAlreadyWritten { index });
        }
        if index > next {
            return Err(StoreError::LeafWouldLeaveGap { index, next });
        }
        self.disk().leaves.push(bytes.to_vec());
        Ok(())
    }

    fn pinned(&self) -> PinnedRoot {
        self.disk().pinned.unwrap_or(PinnedRoot {
            tree_size: 0,
            root: Frontier::new().root(),
        })
    }

    fn pin(&mut self, pin: PinnedRoot) -> StoreResult<()> {
        self.disk().pinned = Some(pin);
        Ok(())
    }

    fn snapshot(&self) -> StoreResult<Option<Vec<u8>>> {
        Ok(self.disk().snapshot.clone())
    }

    fn put_snapshot(&mut self, bytes: &[u8]) -> StoreResult<()> {
        self.disk().snapshot = Some(bytes.to_vec());
        Ok(())
    }
}

/// A log in memory, its roots file and service key on the disk, with the
/// leaves read counted.
struct Place {
    dir: tempfile::TempDir,
    disk: Arc<Mutex<Disk>>,
    reads: Arc<AtomicU64>,
}

impl Place {
    fn new() -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            dir: tempfile::TempDir::new()?,
            disk: Arc::default(),
            reads: Arc::default(),
        })
    }

    fn roots(&self) -> PathBuf {
        self.dir.path().join("leaf-roots.bin")
    }

    /// Opens the directory, keeping its roots in `roots` when one is named.
    fn open(&self, roots: Option<PathBuf>) -> Result<Directory<Memory>, Box<dyn Error>> {
        let (disk, reads) = (Arc::clone(&self.disk), Arc::clone(&self.reads));
        let reopen: Reopen<Memory> = Box::new(move || {
            Ok(Memory {
                disk: Arc::clone(&disk),
                reads: Arc::clone(&reads),
            })
        });
        let key = Ed25519Identity::load_or_generate(&self.dir.path().join("service.key"))?;
        Ok(Directory::open_beside(reopen, key, SNAPSHOT_EVERY, roots)?)
    }

    /// The receipt of the leaf at `index`, and how many leaves it read.
    fn receipt(
        &self,
        directory: &Directory<Memory>,
        index: u64,
    ) -> Result<(Receipt, u64), Box<dyn Error>> {
        self.reads.store(0, Ordering::SeqCst);
        let receipt = directory.receipt_at(index)?.ok_or("the leaf is folded")?;
        Ok((receipt, self.reads.load(Ordering::SeqCst)))
    }
}

fn administrator() -> Result<Actor, Box<dyn Error>> {
    Ok(Actor::new(
        LoginBinding::new("https://issuer.test", "administrator")?,
        Provenance::new(AuthMethod::Oidc, 1_790_000_000),
    ))
}

fn operation(n: u64) -> OperationId {
    let mut bytes = [1_u8; 16];
    bytes[8..].copy_from_slice(&n.to_be_bytes());
    OperationId::from_bytes(bytes)
}

fn answered(receipts: &[Receipt], index: u64) -> Result<&Receipt, Box<dyn Error>> {
    let at = usize::try_from(index)?;
    Ok(receipts.get(at).ok_or("the leaf was appended")?)
}

#[test]
fn a_receipt_reads_one_leaf_and_a_removed_roots_file_is_rebuilt() -> TestResult {
    let place = Place::new()?;
    let mut directory = place.open(Some(place.roots()))?;
    let mut receipts = Vec::new();
    for n in 0..LEAVES {
        let profile = Profile::new(&format!("Person {n}"))?;
        let (_, receipt) = directory.register_person(administrator()?, operation(n), profile, n)?;
        receipts.push(receipt);
    }
    let (receipt, reads) = place.receipt(&directory, READ)?;
    assert_eq!(reads, 1, "the receipt of leaf {READ} reads its one leaf");
    assert_eq!(&receipt, answered(&receipts, READ)?);
    drop(directory);

    let checkpointed = place.open(None)?;
    let (receipt, reads) = place.receipt(&checkpointed, READ)?;
    assert_eq!(reads, READ - 3 * 1024 + 1, "from the nearest checkpoint");
    assert_eq!(&receipt, answered(&receipts, READ)?);
    drop(checkpointed);

    let reopened = place.open(Some(place.roots()))?;
    let (receipt, reads) = place.receipt(&reopened, READ)?;
    assert_eq!(reads, 1, "the roots are read back after a restart");
    assert_eq!(&receipt, answered(&receipts, READ)?);
    drop(reopened);

    std::fs::remove_file(place.roots())?;
    let rebuilt = place.open(Some(place.roots()))?;
    assert_eq!(std::fs::metadata(place.roots())?.len(), LEAVES * 32);
    let mut checked = 0;
    for index in [
        0,
        1,
        1_023,
        1_024,
        3_071,
        3_072,
        READ,
        4_095,
        4_096,
        LEAVES - 1,
    ] {
        let (receipt, reads) = place.receipt(&rebuilt, index)?;
        assert_eq!(reads, 1, "leaf {index} reads its one leaf");
        assert_eq!(&receipt, answered(&receipts, index)?, "leaf {index}");
        checked += 1;
    }
    assert_eq!(checked, 10);
    Ok(())
}

/// How many leaves opening the directory over `place` reads, with the
/// directory it opened.
fn opened(place: &Place) -> Result<(Directory<Memory>, u64), Box<dyn Error>> {
    place.reads.store(0, Ordering::SeqCst);
    let directory = place.open(Some(place.roots()))?;
    Ok((directory, place.reads.load(Ordering::SeqCst)))
}

/// Overwrite the record of leaf `index` in the roots file.
fn tamper(place: &Place, index: u64) -> TestResult {
    use std::io::{Seek, SeekFrom, Write};
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .open(place.roots())?;
    file.seek(SeekFrom::Start(index * 32))?;
    file.write_all(&[0xA5; 32])?;
    file.sync_all()?;
    Ok(())
}

#[test]
fn a_refused_or_short_roots_file_is_rebuilt_from_the_log() -> TestResult {
    let place = Place::new()?;
    let mut directory = place.open(Some(place.roots()))?;
    let mut receipts = Vec::new();
    for n in 0..LEAVES {
        let profile = Profile::new(&format!("Person {n}"))?;
        let (_, receipt) = directory.register_person(administrator()?, operation(n), profile, n)?;
        receipts.push(receipt);
    }
    drop(directory);
    // The snapshot is at 4 096 leaves; a start reads the 904 after it.
    let tail = LEAVES - 4 * 1024;

    let (_, reads) = opened(&place)?;
    assert_eq!(reads, tail, "a kept roots file is read, not rebuilt");

    // Each damage is refused and the whole snapshot's roots are read again,
    // or, for a short file, those from the checkpoint below its end.
    let damages: [(&str, u64); 3] = [
        ("the record at the last checkpoint", 4 * 1024),
        ("the record at the snapshot", 4 * 1024),
        ("a file cut short", 3 * 1024),
    ];
    let mut fired = 0;
    for (damage, rebuilt) in damages {
        match damage {
            "the record at the last checkpoint" => tamper(&place, 3 * 1024 - 1)?,
            "the record at the snapshot" => tamper(&place, 4 * 1024 - 1)?,
            _ => std::fs::OpenOptions::new()
                .write(true)
                .open(place.roots())?
                .set_len(2_000 * 32)?,
        }
        let (directory, reads) = opened(&place)?;
        assert_eq!(reads, rebuilt + tail, "{damage}: rebuilt from the log");
        assert_eq!(std::fs::metadata(place.roots())?.len(), LEAVES * 32);
        for index in [0, 1_999, 3_071, READ, 4_095, LEAVES - 1] {
            let (receipt, reads) = place.receipt(&directory, index)?;
            assert_eq!(reads, 1, "{damage}: leaf {index} reads its one leaf");
            assert_eq!(&receipt, answered(&receipts, index)?, "{damage}: {index}");
        }
        fired += 1;
    }
    assert_eq!(fired, 3);
    Ok(())
}
