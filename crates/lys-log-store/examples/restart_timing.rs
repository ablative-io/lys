//! Times a restart of a file-backed log two ways: the full open through
//! [`Log::open`], which reads and hashes every leaf, and [`open_with_snapshot`],
//! which resumes a [`FrontierLog`](lys_log_store::FrontierLog) from a signed
//! snapshot and reads only the leaves after it.
//!
//! ```text
//! cargo run --release -p lys-log-store --example restart_timing -- [leaves]
//! ```
//!
//! Builds a log of `leaves` leaves (default 1,000,000) in a temporary folder,
//! seals a snapshot 1,000 leaves before its end, opens it both ways, and
//! prints one JSON line: the leaf count, the snapshot's size, each start's
//! duration in milliseconds, and the leaves each start read from the store.
//!
//! Both starts run after the build, so the leaf files are equally warm in the
//! page cache for each; the build itself is timed and printed separately.

use std::cell::Cell;
use std::error::Error;
use std::path::Path;
use std::time::Instant;

use lys_core::Ed25519Identity;
use lys_log_store::{
    FileLeafStore, Frontier, LeafStore, Log, PinnedRoot, Start, StoreResult, open_with_snapshot,
    seal,
};

/// The leaf count when no argument is given.
const DEFAULT_LEAVES: u64 = 1_000_000;

/// How many leaves the snapshot sits before the end of the log.
const TAIL: u64 = 1_000;

/// The origin of the log this example builds.
const ORIGIN: &str = "example.com/lys/restart-timing";

/// The kind of state the snapshot names.
const DOMAIN: &str = "lys/restart-timing/v1";

/// A [`LeafStore`] that counts the leaves read through it and passes every
/// call on to the store it wraps.
struct Counting<S> {
    inner: S,
    reads: Cell<u64>,
}

impl<S: LeafStore> Counting<S> {
    fn new(inner: S) -> Self {
        Self {
            inner,
            reads: Cell::new(0),
        }
    }

    /// How many leaves were read through this handle.
    fn reads(&self) -> u64 {
        self.reads.get()
    }
}

impl<S: LeafStore> LeafStore for Counting<S> {
    fn origin(&self) -> &str {
        self.inner.origin()
    }

    fn extent(&self) -> u64 {
        self.inner.extent()
    }

    fn leaf(&self, index: u64) -> StoreResult<Option<Vec<u8>>> {
        self.reads.set(self.reads.get().saturating_add(1));
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

/// Reads the leaf count from the first argument, or answers the default.
/// Underscores are accepted as digit separators.
fn leaf_count() -> Result<u64, Box<dyn Error>> {
    let Some(argument) = std::env::args().nth(1) else {
        return Ok(DEFAULT_LEAVES);
    };
    let digits: String = argument.chars().filter(|c| *c != '_').collect();
    let count = digits.parse::<u64>().map_err(|error| {
        format!("the leaf count {argument:?} is not a whole number of leaves: {error}")
    })?;
    if count == 0 {
        return Err("the leaf count must be at least 1".into());
    }
    Ok(count)
}

/// The bytes of the leaf at `index`: distinct for every index.
fn leaf_bytes(index: u64) -> Vec<u8> {
    format!("restart-timing leaf {index:020}").into_bytes()
}

/// Appends the leaves `from..to` to `store`, extending `frontier` with each.
fn append_range(
    store: &mut FileLeafStore,
    frontier: &mut Frontier,
    from: u64,
    to: u64,
) -> StoreResult<()> {
    for index in from..to {
        let bytes = leaf_bytes(index);
        frontier.push(&bytes);
        let pin = PinnedRoot {
            tree_size: frontier.size(),
            root: frontier.root(),
        };
        store.append(index, &[bytes.as_slice()], pin)?;
    }
    Ok(())
}

/// Builds a log of `leaves` leaves at `dir`, with a snapshot sealed by `key`
/// at `snapshot_at`, and pins the whole tree once at the end.
///
/// Each leaf is stored durably as it is written; the pin is written once
/// because an intermediate pin changes nothing either start reads.
fn build(
    dir: &Path,
    leaves: u64,
    snapshot_at: u64,
    key: &Ed25519Identity,
) -> Result<(), Box<dyn Error>> {
    let mut store = FileLeafStore::create(dir, ORIGIN)?;
    let mut frontier = Frontier::new();
    append_range(&mut store, &mut frontier, 0, snapshot_at)?;
    let state = snapshot_at.to_be_bytes();
    store.put_snapshot(&seal(DOMAIN, ORIGIN, &frontier, &state, key))?;
    append_range(&mut store, &mut frontier, snapshot_at, leaves)?;
    store.pin(PinnedRoot {
        tree_size: frontier.size(),
        root: frontier.root(),
    })?;
    Ok(())
}

/// What one start measured.
struct Measured {
    millis: f64,
    reads: u64,
}

/// Opens the log at `dir` through [`Log::open`], reading every leaf.
fn time_full_open(dir: &Path, leaves: u64) -> Result<Measured, Box<dyn Error>> {
    let began = Instant::now();
    let log = Log::open(Counting::new(FileLeafStore::open(dir)?))?;
    let millis = began.elapsed().as_secs_f64() * 1e3;
    let size = log.tree().len();
    if size != leaves {
        return Err(format!("the full open holds {size} leaves, not {leaves}").into());
    }
    Ok(Measured {
        millis,
        reads: log.store().reads(),
    })
}

/// Opens the log at `dir` through [`open_with_snapshot`] from its snapshot,
/// and refuses a start that did not resume from it.
fn time_snapshot_start(
    dir: &Path,
    leaves: u64,
    snapshot_at: u64,
    public_key: &[u8; 32],
) -> Result<Measured, Box<dyn Error>> {
    let began = Instant::now();
    let started = open_with_snapshot(Counting::new(FileLeafStore::open(dir)?), DOMAIN, public_key)?;
    let millis = began.elapsed().as_secs_f64() * 1e3;
    match started.start {
        Start::Resumed { size, .. } if size == snapshot_at => {}
        Start::Resumed { size, .. } => {
            return Err(format!("the start resumed at tree size {size}, not {snapshot_at}").into());
        }
        Start::Rebuilt { refusal, .. } => {
            return Err(format!("the start did not resume from the snapshot: {refusal}").into());
        }
    }
    let size = started.log.len();
    if size != leaves {
        return Err(format!("the resumed log holds {size} leaves, not {leaves}").into());
    }
    let state = snapshot_at.to_be_bytes();
    if started.state.as_deref() != Some(state.as_slice()) {
        return Err("the resumed state is not the state the snapshot sealed".into());
    }
    Ok(Measured {
        millis,
        reads: started.log.store().reads(),
    })
}

fn main() -> Result<(), Box<dyn Error>> {
    let leaves = leaf_count()?;
    let snapshot_at = leaves.saturating_sub(TAIL);
    let scratch = tempfile::tempdir()?;
    let dir = scratch.path().join("log");
    let key = Ed25519Identity::load_or_generate(&scratch.path().join("key"))?;

    let began = Instant::now();
    build(&dir, leaves, snapshot_at, &key)?;
    let build_millis = began.elapsed().as_secs_f64() * 1e3;

    let full = time_full_open(&dir, leaves)?;
    let resumed = time_snapshot_start(&dir, leaves, snapshot_at, &key.public_key_bytes())?;

    let line = serde_json::json!({
        "leaves": leaves,
        "snapshot_at": snapshot_at,
        "build_ms": build_millis,
        "log_open_ms": full.millis,
        "log_open_leaf_reads": full.reads,
        "snapshot_start_ms": resumed.millis,
        "snapshot_start_leaf_reads": resumed.reads,
    });
    println!("{line}");
    scratch.close()?;
    Ok(())
}
