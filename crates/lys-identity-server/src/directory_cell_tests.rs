#![cfg(test)]
//! Gates on the shared directory: a read answers while a write is held
//! inside the disk, before the write is released, from the state before
//! it; once the write is answered, a read answers the state after it.
//! Counted through a leaf store that counts and holds its writes, never by
//! a clock.

use std::error::Error;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};

use lys_core::Ed25519Identity;
use lys_identity::log::Reopen;
use lys_identity::{Actor, AuthMethod, Directory, LoginBinding, OperationId, Profile, Provenance};
use lys_log_store::{FileLeafStore, LeafStore, PinnedRoot, StoreResult};

use super::DirectoryCell;
use crate::error::ServerError;

type TestResult = Result<(), Box<dyn Error>>;

const ORIGIN: &str = "example.test/lys/directory";

/// Where the held write stands.
#[derive(Default)]
struct Stand {
    armed: bool,
    entered: bool,
    released: bool,
}

/// Holds the next leaf write once armed, until released, and counts every
/// leaf written.
#[derive(Default)]
struct Gate {
    stand: Mutex<Stand>,
    moved: Condvar,
    puts: AtomicU64,
}

impl Gate {
    fn arm(&self) {
        self.stand.lock().unwrap().armed = true;
    }

    fn pass(&self) {
        self.puts.fetch_add(1, Ordering::SeqCst);
        let mut stand = self.stand.lock().unwrap();
        if !stand.armed {
            return;
        }
        stand.entered = true;
        self.moved.notify_all();
        while !stand.released {
            stand = self.moved.wait(stand).unwrap();
        }
    }

    fn wait_entered(&self) {
        let mut stand = self.stand.lock().unwrap();
        while !stand.entered {
            stand = self.moved.wait(stand).unwrap();
        }
    }

    fn release(&self) {
        self.stand.lock().unwrap().released = true;
        self.moved.notify_all();
    }

    fn released(&self) -> bool {
        self.stand.lock().unwrap().released
    }

    fn puts(&self) -> u64 {
        self.puts.load(Ordering::SeqCst)
    }
}

/// A file store whose leaf writes pass through the gate.
struct Gated {
    inner: FileLeafStore,
    gate: Arc<Gate>,
}

impl LeafStore for Gated {
    fn origin(&self) -> &str {
        self.inner.origin()
    }

    fn extent(&self) -> u64 {
        self.inner.extent()
    }

    fn leaf(&self, index: u64) -> StoreResult<Option<Vec<u8>>> {
        self.inner.leaf(index)
    }

    fn put_leaf(&mut self, index: u64, bytes: &[u8]) -> StoreResult<()> {
        self.gate.pass();
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

fn administrator() -> Result<Actor, Box<dyn Error>> {
    Ok(Actor::new(
        LoginBinding::new("https://issuer.test", "administrator")?,
        Provenance::new(AuthMethod::Oidc, 1_790_000_000),
    ))
}

/// How many identities a read of the directory answers.
fn people(cell: &DirectoryCell<Gated>) -> Result<usize, ServerError> {
    cell.read(|projection| Ok(projection.records().count()))
}

#[test]
fn a_read_answers_while_a_write_is_held_on_the_disk() -> TestResult {
    let dir = tempfile::tempdir()?;
    let log = dir.path().join("log");
    FileLeafStore::create(&log, ORIGIN)?;
    let gate = Arc::new(Gate::default());
    let opened = Arc::clone(&gate);
    let reopen: Reopen<Gated> = Box::new(move || {
        Ok(Gated {
            inner: FileLeafStore::open(&log)?,
            gate: Arc::clone(&opened),
        })
    });
    let key = Ed25519Identity::load_or_generate(&dir.path().join("service.key"))?;
    let cell = DirectoryCell::new(Directory::open(reopen, key)?);
    assert_eq!(people(&cell)?, 0);

    let (actor, profile) = (administrator()?, Profile::new("Ada")?);
    let operation = OperationId::from_bytes([1; 16]);
    gate.arm();
    std::thread::scope(|scope| -> TestResult {
        let shared = &cell;
        let writer = scope.spawn(move || {
            shared.write(move |directory| {
                Ok(directory.register_person(actor, operation, profile, 1)?)
            })
        });
        gate.wait_entered();
        let puts = gate.puts();
        let during = people(shared);
        let held = !gate.released();
        gate.release();
        let written = writer
            .join()
            .map_err(|panic| format!("the writer panicked: {panic:?}"))?;
        assert_eq!(puts, 1, "the write is inside the disk");
        assert_eq!(during?, 0, "the read answers the state before the write");
        assert!(held, "the read answered before the write was released");
        written?;
        Ok(())
    })?;
    assert_eq!(people(&cell)?, 1, "once answered, the write is read");
    assert_eq!(gate.puts(), 1);
    Ok(())
}
