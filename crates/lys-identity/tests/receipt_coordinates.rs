//! A receipt is rebuilt from its one leaf when the directory keeps each
//! leaf's coordinate beside its log: the same receipt, byte for byte, as the
//! checkpoints rebuild by reading up to a thousand leaves, and a coordinate
//! file removed from disk is rebuilt from the log when the directory opens.

use std::error::Error;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use lys_core::Ed25519Identity;
use lys_identity::log::Reopen;
use lys_identity::receipt::Receipt;
use lys_identity::{Actor, AuthMethod, Directory, LoginBinding, OperationId, Profile, Provenance};
use lys_log_store::{Frontier, LeafStore, PinnedRoot, StoreError, StoreResult};

type TestResult = Result<(), Box<dyn Error>>;

const ORIGIN: &str = "example.test/lys/directory";

/// How many leaves the ledger holds.
const LEAVES: u64 = 5_000;

/// The leaf whose receipt is rebuilt: 928 leaves past the checkpoint at 3072.
const LEAF: u64 = 4_000;

/// What a store held in memory keeps, shared by every handle opened on it.
#[derive(Default)]
struct Disk {
    leaves: Vec<Vec<u8>>,
    pinned: Option<PinnedRoot>,
    snapshot: Option<Vec<u8>>,
}

/// A store held in memory that counts every leaf read through it.
struct Counted {
    disk: Arc<Mutex<Disk>>,
    reads: Arc<AtomicU64>,
}

impl Counted {
    fn disk(&self) -> MutexGuard<'_, Disk> {
        self.disk.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl LeafStore for Counted {
    fn origin(&self) -> &str {
        ORIGIN
    }

    fn extent(&self) -> u64 {
        self.disk().leaves.len() as u64
    }

    fn leaf(&self, index: u64) -> StoreResult<Option<Vec<u8>>> {
        self.reads.fetch_add(1, Ordering::SeqCst);
        Ok(usize::try_from(index)
            .ok()
            .and_then(|slot| self.disk().leaves.get(slot).cloned()))
    }

    fn put_leaf(&mut self, index: u64, bytes: &[u8]) -> StoreResult<()> {
        let next = self.extent();
        if index != next {
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

/// One log held in memory, its leaf reads counted, and its service key.
struct Place {
    dir: tempfile::TempDir,
    disk: Arc<Mutex<Disk>>,
    reads: Arc<AtomicU64>,
}

impl Place {
    fn new() -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            dir: tempfile::TempDir::new()?,
            disk: Arc::new(Mutex::new(Disk::default())),
            reads: Arc::new(AtomicU64::new(0)),
        })
    }

    fn coordinates(&self) -> std::path::PathBuf {
        self.dir.path().join("coordinates.bin")
    }

    fn reopen(&self) -> Reopen<Counted> {
        let (disk, reads) = (Arc::clone(&self.disk), Arc::clone(&self.reads));
        Box::new(move || {
            Ok(Counted {
                disk: Arc::clone(&disk),
                reads: Arc::clone(&reads),
            })
        })
    }

    fn key(&self) -> Result<Ed25519Identity, Box<dyn Error>> {
        Ok(Ed25519Identity::load_or_generate(
            &self.dir.path().join("service.key"),
        )?)
    }

    /// The directory, keeping each leaf's coordinate in `coordinates`.
    fn keeping(&self, coordinates: &Path) -> Result<Directory<Counted>, Box<dyn Error>> {
        Ok(Directory::open_with_coordinates(
            self.reopen(),
            self.key()?,
            coordinates,
        )?)
    }

    /// The directory, rebuilding each receipt from the checkpoints.
    fn checkpointed(&self) -> Result<Directory<Counted>, Box<dyn Error>> {
        Ok(Directory::open(self.reopen(), self.key()?)?)
    }

    /// The receipt of the leaf at `index`, with the leaves read for it.
    fn receipt(
        &self,
        directory: &Directory<Counted>,
        index: u64,
    ) -> Result<(Option<Receipt>, u64), Box<dyn Error>> {
        self.reads.store(0, Ordering::SeqCst);
        let receipt = directory.receipt_at(index)?;
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
    let mut bytes = [0xa5; 16];
    bytes[8..].copy_from_slice(&(n + 1).to_be_bytes());
    OperationId::from_bytes(bytes)
}

/// The leaves the receipts are compared at: every checkpoint's edges, the
/// last leaf, and a spread between.
fn compared() -> Vec<u64> {
    let mut indexes: Vec<u64> = (0..LEAVES).step_by(97).collect();
    indexes.extend([1_023, 1_024, 1_025, 3_071, 3_072]);
    indexes.extend([LEAF, 4_095, 4_096, LEAVES - 1]);
    indexes
}

#[test]
fn a_receipt_is_rebuilt_from_its_one_leaf() -> TestResult {
    let place = Place::new()?;
    let mut directory = place.keeping(&place.coordinates())?;
    let mut answered = Vec::new();
    for n in 0..LEAVES {
        let (_, receipt) = directory.register_person(
            administrator()?,
            operation(n),
            Profile::new(&format!("Person {n}"))?,
            n,
        )?;
        answered.push(receipt);
    }
    let answer = |index: u64| usize::try_from(index).ok().and_then(|at| answered.get(at));

    let (kept, reads) = place.receipt(&directory, LEAF)?;
    assert_eq!(reads, 1, "the receipt of leaf {LEAF} reads its one leaf");
    assert_eq!(kept.as_ref(), answer(LEAF), "answered at commit");
    drop(directory);

    let checkpointed = place.checkpointed()?;
    let (rebuilt, reads) = place.receipt(&checkpointed, LEAF)?;
    assert_eq!(
        reads,
        LEAF - 3_072 + 1,
        "without the coordinates every leaf from the checkpoint at 3072 is read"
    );
    assert_eq!(rebuilt, kept, "the checkpoints rebuild the same receipt");

    let directory = place.keeping(&place.coordinates())?;
    let mut checked = 0;
    for index in compared() {
        let (kept, reads) = place.receipt(&directory, index)?;
        assert_eq!(reads, 1, "the receipt of leaf {index} reads its one leaf");
        assert_eq!(kept, checkpointed.receipt_at(index)?, "leaf {index}");
        assert_eq!(kept.as_ref(), answer(index), "leaf {index}");
        checked += 1;
    }
    assert_eq!(checked, compared().len());
    assert_eq!(directory.receipt_at(LEAVES)?, None);
    Ok(())
}

#[test]
fn a_coordinate_file_removed_is_rebuilt_from_the_log() -> TestResult {
    let place = Place::new()?;
    let mut directory = place.keeping(&place.coordinates())?;
    let mut answered = Vec::new();
    for n in 0..1_500 {
        let (_, receipt) = directory.register_person(
            administrator()?,
            operation(n),
            Profile::new(&format!("Person {n}"))?,
            n,
        )?;
        answered.push(receipt);
    }
    drop(directory);

    std::fs::remove_file(place.coordinates())?;
    let rebuilt = place.keeping(&place.coordinates())?;
    assert_eq!(
        std::fs::metadata(place.coordinates())?.len(),
        1_500 * 32,
        "one root is kept for every leaf of the log"
    );
    let mut checked = 0;
    for (index, receipt) in (0_u64..).zip(&answered) {
        let (kept, reads) = place.receipt(&rebuilt, index)?;
        assert_eq!(reads, 1, "the receipt of leaf {index} reads its one leaf");
        assert_eq!(kept.as_ref(), Some(receipt), "leaf {index}");
        checked += 1;
    }
    assert_eq!(checked, 1_500);
    drop(rebuilt);

    let file = std::fs::OpenOptions::new()
        .write(true)
        .open(place.coordinates())?;
    file.set_len(1_000 * 32)?;
    drop(file);
    let levelled = place.keeping(&place.coordinates())?;
    assert_eq!(
        std::fs::metadata(place.coordinates())?.len(),
        1_500 * 32,
        "a file behind the log is brought level with it"
    );
    let (kept, reads) = place.receipt(&levelled, 1_499)?;
    assert_eq!(reads, 1);
    assert_eq!(kept.as_ref(), answered.last());
    Ok(())
}
