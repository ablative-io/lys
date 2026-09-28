//! A restart loads the directory's signed snapshot and reads only the leaves
//! after it; a snapshot that is refused is named, the directory is rebuilt
//! from every leaf, and a new snapshot is written.

#![cfg(unix)]

use std::error::Error;
use std::num::NonZeroU64;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use lys_core::Ed25519Identity;
use lys_identity::log::Reopen;
use lys_identity::projection::Projection;
use lys_identity::receipt::Receipt;
use lys_identity::{Actor, AuthMethod, Directory, LoginBinding, OperationId, Profile, Provenance};
use lys_log_store::{
    FileLeafStore, Frontier, LeafStore, PinnedRoot, SnapshotRefusal, Start, StoreResult, seal,
};

type TestResult = Result<(), Box<dyn Error>>;

const ORIGIN: &str = "example.test/lys/directory";

/// The snapshot domain the directory seals its state under.
const DOMAIN: &str = "lys/identity-directory/v1";

/// A file store that counts every leaf read through it.
struct Counting {
    inner: FileLeafStore,
    reads: Arc<AtomicU64>,
}

impl LeafStore for Counting {
    fn origin(&self) -> &str {
        self.inner.origin()
    }

    fn extent(&self) -> u64 {
        self.inner.extent()
    }

    fn leaf(&self, index: u64) -> StoreResult<Option<Vec<u8>>> {
        self.reads.fetch_add(1, Ordering::SeqCst);
        self.inner.leaf(index)
    }

    fn put_leaf(&mut self, index: u64, bytes: &[u8]) -> StoreResult<()> {
        self.inner.put_leaf(index, bytes)
    }

    fn pinned(&self) -> PinnedRoot {
        self.inner.pinned()
    }

    fn pin(&mut self, pin: PinnedRoot) -> StoreResult<()> {
        self.inner.pin(pin)
    }

    fn snapshot(&self) -> StoreResult<Option<Vec<u8>>> {
        self.inner.snapshot()
    }

    fn put_snapshot(&mut self, bytes: &[u8]) -> StoreResult<()> {
        self.inner.put_snapshot(bytes)
    }
}

/// A log directory and its service key, with the leaf reads counted.
struct Place {
    dir: tempfile::TempDir,
    reads: Arc<AtomicU64>,
}

impl Place {
    fn new() -> Result<Self, Box<dyn Error>> {
        let dir = tempfile::TempDir::new()?;
        FileLeafStore::create(&dir.path().join("log"), ORIGIN)?;
        let key = dir.path().join("service.key");
        std::fs::write(&key, [23_u8; 32])?;
        std::fs::set_permissions(&key, std::fs::Permissions::from_mode(0o600))?;
        Ok(Self {
            dir,
            reads: Arc::new(AtomicU64::new(0)),
        })
    }

    fn log(&self) -> PathBuf {
        self.dir.path().join("log")
    }

    fn snapshot(&self) -> PathBuf {
        self.log().join("snapshot.bin")
    }

    /// Opens the directory, snapshotting every four entries, and answers it
    /// with the leaves the open read.
    fn open(&self) -> Result<(Directory<Counting>, u64), Box<dyn Error>> {
        let log: PathBuf = self.log();
        let reads = Arc::clone(&self.reads);
        let reopen: Reopen<Counting> = Box::new(move || {
            Ok(Counting {
                inner: FileLeafStore::open(&log)?,
                reads: Arc::clone(&reads),
            })
        });
        self.reads.store(0, Ordering::SeqCst);
        let key = Ed25519Identity::load(&self.dir.path().join("service.key"))?;
        let every = NonZeroU64::new(4).ok_or("four is not zero")?;
        let directory = Directory::open_with(reopen, key, every)?;
        Ok((directory, self.reads.load(Ordering::SeqCst)))
    }
}

fn administrator() -> Result<Actor, Box<dyn Error>> {
    Ok(Actor::new(
        LoginBinding::new("https://issuer.test", "administrator")?,
        Provenance::new(AuthMethod::Oidc, 1_790_000_000),
    ))
}

/// Registers `count` people, the `n`th with operation id `first + n`,
/// answering the receipt each registration was answered with.
fn register(
    directory: &mut Directory<Counting>,
    first: u8,
    count: u8,
) -> Result<Vec<Receipt>, Box<dyn Error>> {
    let mut receipts = Vec::new();
    for n in 0..count {
        let seed = first + n;
        let (_, receipt) = directory.register_person(
            administrator()?,
            OperationId::from_bytes([seed; 16]),
            Profile::new(&format!("Person {seed}"))?,
            u64::from(seed),
        )?;
        receipts.push(receipt);
    }
    Ok(receipts)
}

fn start_of(directory: &mut Directory<Counting>) -> Result<Start, Box<dyn Error>> {
    Ok(directory.log()?.start().clone())
}

fn projection_of(directory: &mut Directory<Counting>) -> Result<Projection, Box<dyn Error>> {
    Ok(directory.projection()?.clone())
}

fn flip_last_byte(path: &Path) -> TestResult {
    let mut bytes = std::fs::read(path)?;
    let last = bytes.last_mut().ok_or("the snapshot is empty")?;
    *last ^= 0xFF;
    std::fs::write(path, bytes)?;
    Ok(())
}

#[test]
fn a_restart_reads_only_the_leaves_after_the_snapshot() -> TestResult {
    let place = Place::new()?;
    let (mut first, _) = place.open()?;
    assert_eq!(
        first.log()?.start().refusal(),
        Some(&SnapshotRefusal::Missing),
        "a log with no snapshot is refused by name and rebuilt"
    );
    let answered = register(&mut first, 1, 10)?;
    let before = projection_of(&mut first)?;
    drop(first);

    let (mut restarted, reads) = place.open()?;
    assert_eq!(
        start_of(&mut restarted)?,
        Start::Resumed {
            size: 8,
            replayed: 2
        }
    );
    assert_eq!(
        reads, 2,
        "only the two leaves after the snapshot at 8 are read"
    );
    assert_eq!(projection_of(&mut restarted)?, before);
    assert_eq!(restarted.log()?.len()?, 10);
    for (index, receipt) in (0_u64..).zip(&answered) {
        assert_eq!(
            restarted.receipt_at(index)?.as_ref(),
            Some(receipt),
            "the receipt built from the log is the one answered at commit"
        );
    }
    assert_eq!(restarted.receipt_at(10)?, None);
    Ok(())
}

#[test]
fn a_snapshot_that_does_not_verify_is_refused_and_rewritten() -> TestResult {
    let place = Place::new()?;
    let (mut first, _) = place.open()?;
    register(&mut first, 1, 10)?;
    let before = projection_of(&mut first)?;
    drop(first);

    flip_last_byte(&place.snapshot())?;
    let (mut rebuilt, reads) = place.open()?;
    assert_eq!(
        start_of(&mut rebuilt)?,
        Start::Rebuilt {
            refusal: SnapshotRefusal::SignatureInvalid,
            replayed: 10
        }
    );
    assert_eq!(reads, 10, "a refused snapshot rebuilds from every leaf");
    assert_eq!(projection_of(&mut rebuilt)?, before);
    drop(rebuilt);

    let (mut again, reads) = place.open()?;
    assert_eq!(
        start_of(&mut again)?,
        Start::Resumed {
            size: 10,
            replayed: 0
        },
        "the rebuild wrote a new snapshot at the whole log"
    );
    assert_eq!(reads, 0);
    assert_eq!(projection_of(&mut again)?, before);
    Ok(())
}

#[test]
fn a_snapshot_of_another_log_is_refused_for_its_root() -> TestResult {
    let (one, other) = (Place::new()?, Place::new()?);
    let (mut first, _) = one.open()?;
    register(&mut first, 1, 8)?;
    drop(first);
    let (mut second, _) = other.open()?;
    register(&mut second, 101, 8)?;
    let before = projection_of(&mut second)?;
    drop(second);

    std::fs::copy(one.snapshot(), other.snapshot())?;
    let (mut rebuilt, reads) = other.open()?;
    assert_eq!(
        start_of(&mut rebuilt)?,
        Start::Rebuilt {
            refusal: SnapshotRefusal::WrongRoot { size: 8 },
            replayed: 8
        }
    );
    assert!(reads >= 8, "a refused snapshot rebuilds from every leaf");
    assert_eq!(projection_of(&mut rebuilt)?, before);
    Ok(())
}

#[test]
fn a_missing_snapshot_is_refused_by_name_and_rewritten() -> TestResult {
    let place = Place::new()?;
    let (mut first, _) = place.open()?;
    register(&mut first, 1, 6)?;
    drop(first);

    std::fs::remove_file(place.snapshot())?;
    let (mut rebuilt, reads) = place.open()?;
    assert_eq!(
        start_of(&mut rebuilt)?,
        Start::Rebuilt {
            refusal: SnapshotRefusal::Missing,
            replayed: 6
        }
    );
    assert_eq!(reads, 6);
    drop(rebuilt);

    let (again, reads) = place.open()?;
    assert_eq!(reads, 0, "the rebuild wrote a snapshot at the whole log");
    drop(again);
    Ok(())
}

#[test]
fn a_signed_snapshot_whose_state_does_not_read_is_refused_and_rewritten() -> TestResult {
    let place = Place::new()?;
    let (mut first, _) = place.open()?;
    register(&mut first, 1, 8)?;
    let before = projection_of(&mut first)?;
    drop(first);

    let store = FileLeafStore::open(&place.log())?;
    let leaves = (0..store.extent())
        .map(|index| store.leaf(index)?.ok_or("a leaf is missing".into()))
        .collect::<Result<Vec<_>, Box<dyn Error>>>()?;
    let key = Ed25519Identity::load(&place.dir.path().join("service.key"))?;
    let sealed = seal(
        DOMAIN,
        ORIGIN,
        &Frontier::from_leaves(&leaves),
        b"not a directory state",
        &key,
    );
    std::fs::write(place.snapshot(), sealed)?;

    let (mut rebuilt, reads) = place.open()?;
    let start = start_of(&mut rebuilt)?;
    assert!(
        matches!(
            start,
            Start::Rebuilt {
                refusal: SnapshotRefusal::StateUnreadable { .. },
                replayed: 8
            }
        ),
        "{start}"
    );
    assert_eq!(reads, 8, "a refused state rebuilds from every leaf");
    assert_eq!(projection_of(&mut rebuilt)?, before);
    drop(rebuilt);

    let (again, reads) = place.open()?;
    assert_eq!(reads, 0, "the rebuild wrote a snapshot at the whole log");
    drop(again);
    Ok(())
}
