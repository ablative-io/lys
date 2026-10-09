#![cfg(test)]

//! DIRECTORY-089 R1, `d089_r1_old_install_and_bounded_replay` (cost half):
//! two logs with the same requested delta and different historical lengths
//! emit the same delta and read the same number of leaves through the
//! range read, bounded by the checkpoint distance and never the history;
//! the whole-history read and a position-by-position read are the positive
//! controls the counter must tell apart.

use std::error::Error;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use crate::support::World;
use lys_identity::IdentityId;
use lys_identity::checkpoints::CHECKPOINT_EVERY;
use lys_identity::grants::{MemoryRelationships, PassOn, Route};
use lys_identity::log::Reopen;
use lys_log_store::{Frontier, LeafStore, PinnedRoot, StoreError, StoreResult};

type TestResult = Result<(), Box<dyn Error>>;

/// One in-memory log's durable parts, shared by every handle opened on it.
#[derive(Default)]
struct Disk {
    leaves: Vec<Vec<u8>>,
    pinned: Option<PinnedRoot>,
    snapshot: Option<Vec<u8>>,
}

/// A handle on the in-memory log that counts every leaf read through it.
struct Counted {
    disk: Arc<Mutex<Disk>>,
    reads: Arc<AtomicU64>,
}

fn poisoned(reason: &str) -> StoreError {
    StoreError::Io {
        context: "the fixture log lock is poisoned".to_owned(),
        source: std::io::Error::other(reason.to_owned()),
    }
}

impl Counted {
    fn disk(&self) -> StoreResult<std::sync::MutexGuard<'_, Disk>> {
        self.disk
            .lock()
            .map_err(|error| poisoned(&error.to_string()))
    }
}

impl LeafStore for Counted {
    fn origin(&self) -> &'static str {
        "example.test/lys/grants"
    }

    fn extent(&self) -> u64 {
        self.disk().map_or(0, |disk| {
            u64::try_from(disk.leaves.len()).unwrap_or(u64::MAX)
        })
    }

    fn leaf(&self, index: u64) -> StoreResult<Option<Vec<u8>>> {
        self.reads.fetch_add(1, Ordering::SeqCst);
        let disk = self.disk()?;
        Ok(usize::try_from(index)
            .ok()
            .and_then(|at| disk.leaves.get(at))
            .cloned())
    }

    fn append(&mut self, index: u64, leaves: &[&[u8]], pin: PinnedRoot) -> StoreResult<()> {
        let mut disk = self.disk()?;
        if u64::try_from(disk.leaves.len()).ok() != Some(index) {
            return Err(StoreError::Io {
                context: "the fixture append is not at the end".to_owned(),
                source: std::io::Error::other("misplaced"),
            });
        }
        disk.leaves.extend(leaves.iter().map(|leaf| leaf.to_vec()));
        disk.pinned = Some(pin);
        Ok(())
    }

    fn pinned(&self) -> PinnedRoot {
        self.disk()
            .ok()
            .and_then(|disk| disk.pinned)
            .unwrap_or_else(|| {
                let empty = Frontier::new();
                PinnedRoot {
                    tree_size: empty.size(),
                    root: empty.root(),
                }
            })
    }

    fn pin(&mut self, pin: PinnedRoot) -> StoreResult<()> {
        self.disk()?.pinned = Some(pin);
        Ok(())
    }

    fn snapshot(&self) -> StoreResult<Option<Vec<u8>>> {
        Ok(self.disk()?.snapshot.clone())
    }

    fn put_snapshot(&mut self, bytes: &[u8]) -> StoreResult<()> {
        self.disk()?.snapshot = Some(bytes.to_vec());
        Ok(())
    }
}

/// A world whose grant log is in memory, its leaf reads counted, grown to
/// `length` events.
fn grown(length: u64) -> Result<(World<Counted>, Arc<AtomicU64>), Box<dyn Error>> {
    let disk = Arc::new(Mutex::new(Disk::default()));
    let reads = Arc::new(AtomicU64::new(0));
    let counted = Arc::clone(&reads);
    let mut world = World::with(
        Box::new(move |_: &Path| -> Reopen<Counted> {
            let disk = Arc::clone(&disk);
            let reads = Arc::clone(&counted);
            Box::new(move || {
                Ok(Counted {
                    disk: Arc::clone(&disk),
                    reads: Arc::clone(&reads),
                })
            })
        }),
        MemoryRelationships::default(),
    )?;
    world.root(world.tom, "tern", PassOn::UseOnly, None)?;
    let tom = IdentityId::Person(world.tom);
    while world.events() < length {
        world.exercise(tom, "read", Route::Api)?;
    }
    Ok((world, reads))
}

/// The leaves `read` takes, and what it answers.
fn counted<T>(reads: &AtomicU64, read: impl FnOnce() -> T) -> (u64, T) {
    let before = reads.load(Ordering::SeqCst);
    let answer = read();
    (reads.load(Ordering::SeqCst) - before, answer)
}

#[test]
fn a_replay_reads_the_same_leaves_at_two_history_lengths() -> TestResult {
    const DELTA: u64 = 3;
    const PAST_CHECKPOINT: u64 = 20;
    let short = PAST_CHECKPOINT;
    let long = CHECKPOINT_EVERY + PAST_CHECKPOINT;
    let mut costs = Vec::new();
    for length in [short, long] {
        let (world, reads) = grown(length)?;
        let head = world.events();
        assert_eq!(head, length);
        let (range_reads, replayed) = counted(&reads, || {
            world.grants.ledger().entries_between(head - DELTA, head)
        });
        let replayed = replayed?;
        assert_eq!(
            u64::try_from(replayed.len())?,
            DELTA,
            "the same delta is emitted"
        );
        assert_eq!(
            replayed.first().map(|(_, coordinate)| coordinate.index),
            Some(head - DELTA)
        );
        assert!(range_reads <= CHECKPOINT_EVERY + DELTA);

        // Controls: one read per position re-reads from the checkpoint each
        // time, and the whole history grows with the log.
        let (one_by_one, _) = counted(&reads, || {
            (head - DELTA..head)
                .map(|index| world.grants.ledger().entry(index))
                .collect::<Result<Vec<_>, _>>()
        });
        assert!(one_by_one > range_reads, "{one_by_one} > {range_reads}");
        let (whole, _) = counted(&reads, || world.grants.ledger().entries());
        assert!(whole >= head, "the whole history reads every leaf: {whole}");
        costs.push(range_reads);
    }
    assert_eq!(
        costs[0], costs[1],
        "the range read's cost does not depend on the history before its checkpoint"
    );
    Ok(())
}
