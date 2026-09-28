//! A signed-event log that starts from its signed snapshot and reads only the
//! leaves after it.
//!
//! The directory and the grants each fold one log of signed events. What they
//! hold of that log is its leaves, in order, each with the coordinate it
//! completed; their projections are indexes over those events. A snapshot
//! carries every leaf the log held at one tree size, sealed by the service key
//! and bound to the tree's root at that size (`lys/log-snapshot/v1`), so a
//! start reads one snapshot and then only the leaves after it.
//!
//! # What is believed, and when it is refused
//!
//! [`lys_log_store::start`] checks the snapshot's signature, kind, origin and
//! root, and that the log reaches its pinned root from the snapshot's frontier.
//! This module then requires the snapshot's leaves to fold, through the leaves
//! after them, to the root the log was opened at, and every one of them to be
//! an event this service signed. A snapshot failing any check is refused by its
//! [`SnapshotRefusal`] name, logged, and never used: the state is rebuilt from
//! every leaf of the log and a new snapshot is written at once.
//!
//! # When a snapshot is written
//!
//! After every append or adoption that carries the log across a multiple of
//! the cadence, a count of entries. A snapshot that cannot be written does not
//! undo the append it follows, which is already durable: the failure is logged
//! by name and kept on the log until a later snapshot succeeds.

use std::marker::PhantomData;
use std::num::NonZeroU64;
use std::sync::OnceLock;

use lys_core::Ed25519Identity;
use lys_core::merkle::{AppendOnlyTree, InclusionProof, RawLeaf};
use lys_log_store::{
    Frontier, FrontierLog, LeafStore, SnapshotRefusal, Start, StoreError, Tail, start,
};

use crate::log::{Coordinate, Reopen};
use crate::snapshot_state::{decode_leaves, encode_leaves};

/// How many entries a log grows by between snapshots, unless its owner is
/// opened with another count.
pub const SNAPSHOT_EVERY: NonZeroU64 = NonZeroU64::MIN.saturating_add(1023);

/// One kind of signed-event leaf: how it is read, and the owner's errors.
pub(crate) trait Leaves {
    /// A verified event.
    type Event;
    /// The owner's error.
    type Error: std::fmt::Display;
    /// The snapshot domain the owner's state is sealed under.
    const DOMAIN: &'static str;
    /// Reads a leaf, checking its signature.
    fn verify(bytes: &[u8], key: &[u8; 32]) -> Result<Self::Event, Self::Error>;
    /// Reads a leaf a checked snapshot carries, without checking its
    /// signature again.
    fn attested(bytes: &[u8], key: &[u8; 32]) -> Result<Self::Event, Self::Error>;
    /// The log could not be read or written.
    fn unavailable(reason: String) -> Self::Error;
    /// The leaf at `index` is not an event this service signed.
    fn not_an_event(index: u64, reason: String) -> Self::Error;
}

/// Every event of a start or an adoption, in log order, with its coordinate.
pub(crate) type Events<E> = Vec<(E, Coordinate)>;

/// How a leaf is read into an event.
type Read<K> = fn(&[u8], &[u8; 32]) -> Result<<K as Leaves>::Event, <K as Leaves>::Error>;

/// A snapshot's leaves with the tail after them, the tree they make, and
/// their events.
struct Accepted<E> {
    leaves: Vec<Vec<u8>>,
    frontier: Frontier,
    events: Events<E>,
}

/// A signed-event log, its leaves, and the snapshots written of them.
pub(crate) struct Ledger<S: LeafStore, K: Leaves> {
    log: FrontierLog<S>,
    leaves: Vec<Vec<u8>>,
    frontier: Frontier,
    proofs: OnceLock<AppendOnlyTree<RawLeaf>>,
    every: NonZeroU64,
    start: Start,
    snapshot_failure: Option<String>,
    kind: PhantomData<fn() -> K>,
}

fn store_down<K: Leaves>(error: &StoreError) -> K::Error {
    K::unavailable(error.to_string())
}

/// Refuses, before anything is pinned, a leaf past the pin that is not a
/// whole event this service signed: one pinned there could never be removed.
fn check_past_pin<S: LeafStore, K: Leaves>(store: &S, key: &[u8; 32]) -> Result<(), K::Error> {
    for index in store.pinned().tree_size..store.extent() {
        let bytes = store
            .leaf(index)
            .map_err(|error| store_down::<K>(&error))?
            .ok_or_else(|| {
                K::not_an_event(
                    index,
                    "the leaf is missing inside the store's extent".to_owned(),
                )
            })?;
        K::verify(&bytes, key).map_err(|error| {
            K::not_an_event(
                index,
                format!("the leaf past the pin was left unpinned: {error}"),
            )
        })?;
    }
    Ok(())
}

/// Extends `frontier` by `leaves`, which start at its size, reading each with
/// `read` and answering each event with the coordinate it completed.
fn fold<K: Leaves>(
    frontier: &mut Frontier,
    leaves: &[Vec<u8>],
    key: &[u8; 32],
    read: Read<K>,
) -> Result<Events<K::Event>, K::Error> {
    let mut events = Vec::with_capacity(leaves.len());
    for bytes in leaves {
        let index = frontier.size();
        let leaf_hash = frontier.push(bytes);
        let event = read(bytes, key).map_err(|error| K::not_an_event(index, error.to_string()))?;
        events.push((
            event,
            Coordinate {
                index,
                tree_size: frontier.size(),
                root: frontier.root(),
                leaf_hash,
            },
        ));
    }
    Ok(events)
}

/// The snapshot's leaves and the tail folded into events, refused by name
/// unless they fold to the root the log was opened at.
fn accept<S: LeafStore, K: Leaves>(
    state: &[u8],
    tail: &Tail,
    log: &FrontierLog<S>,
    key: &[u8; 32],
) -> Result<Accepted<K::Event>, SnapshotRefusal> {
    let size = tail.from;
    let leaves = decode_leaves(state).map_err(|reason| SnapshotRefusal::StateUnreadable {
        reason: reason.to_owned(),
    })?;
    if u64::try_from(leaves.len()).ok() != Some(size) {
        return Err(SnapshotRefusal::StateUnreadable {
            reason: format!(
                "the snapshot is at tree size {size} and carries {} leaves",
                leaves.len()
            ),
        });
    }
    let mut frontier = Frontier::new();
    let mut events = fold::<K>(&mut frontier, &leaves, key, K::attested).map_err(|error| {
        SnapshotRefusal::StateUnreadable {
            reason: error.to_string(),
        }
    })?;
    let tail_events = fold::<K>(&mut frontier, &tail.leaves, key, K::verify).map_err(|error| {
        SnapshotRefusal::StateUnreadable {
            reason: error.to_string(),
        }
    })?;
    if frontier.root() != log.frontier().root() || frontier.size() != log.len() {
        return Err(SnapshotRefusal::WrongRoot { size });
    }
    events.extend(tail_events);
    let mut leaves = leaves;
    leaves.extend(tail.leaves.iter().cloned());
    Ok(Accepted {
        leaves,
        frontier,
        events,
    })
}

impl<S: LeafStore, K: Leaves> Ledger<S, K> {
    /// Opens the log `reopen` gives from its snapshot, reading only the leaves
    /// after it, or from every leaf when the snapshot is refused, and then
    /// writes a new snapshot. Answers the log and every event it holds.
    pub(crate) fn open(
        reopen: &Reopen<S>,
        key: &Ed25519Identity,
        every: NonZeroU64,
    ) -> Result<(Self, Events<K::Event>), K::Error> {
        let public = key.public_key_bytes();
        let store = reopen().map_err(|error| store_down::<K>(&error))?;
        check_past_pin::<S, K>(&store, &public)?;
        let started = start(store, K::DOMAIN, &public).map_err(|error| store_down::<K>(&error))?;
        let refusal = match (started.state, started.start) {
            (Some(state), how) => {
                match accept::<S, K>(&state, &started.tail, &started.log, &public) {
                    Ok(Accepted {
                        leaves,
                        frontier,
                        events,
                    }) => {
                        let from = started.tail.from;
                        let mut ledger = Self::new(started.log, leaves, frontier, every, how);
                        tracing::info!(domain = K::DOMAIN, "{}", ledger.start);
                        ledger.snapshot_if_due(from, key);
                        return Ok((ledger, events));
                    }
                    Err(refusal) => refusal,
                }
            }
            (None, Start::Rebuilt { refusal, .. }) => {
                return Self::rebuilt(started.log, &started.tail, refusal, key, every);
            }
            (None, Start::Resumed { size, .. }) => SnapshotRefusal::StateUnreadable {
                reason: format!("the start resumed at tree size {size} without a state"),
            },
        };
        let store = reopen().map_err(|error| store_down::<K>(&error))?;
        let (log, tail) = FrontierLog::open(store).map_err(|error| store_down::<K>(&error))?;
        Self::rebuilt(log, &tail, refusal, key, every)
    }

    fn new(
        log: FrontierLog<S>,
        leaves: Vec<Vec<u8>>,
        frontier: Frontier,
        every: NonZeroU64,
        start: Start,
    ) -> Self {
        Self {
            log,
            leaves,
            frontier,
            proofs: OnceLock::new(),
            every,
            start,
            snapshot_failure: None,
            kind: PhantomData,
        }
    }

    /// Folds every leaf of a log opened from nothing because `refusal`
    /// refused its snapshot, logs the refusal, and writes a new snapshot.
    fn rebuilt(
        log: FrontierLog<S>,
        tail: &Tail,
        refusal: SnapshotRefusal,
        key: &Ed25519Identity,
        every: NonZeroU64,
    ) -> Result<(Self, Events<K::Event>), K::Error> {
        let mut frontier = Frontier::new();
        let events = fold::<K>(
            &mut frontier,
            &tail.leaves,
            &key.public_key_bytes(),
            K::verify,
        )?;
        let replayed = u64::try_from(tail.leaves.len()).unwrap_or(u64::MAX);
        let how = Start::Rebuilt { refusal, replayed };
        tracing::warn!(domain = K::DOMAIN, "{how}");
        let mut ledger = Self::new(log, tail.leaves.clone(), frontier, every, how);
        ledger.write_snapshot(key).map_err(K::unavailable)?;
        Ok((ledger, events))
    }

    /// How the log was started, and why, when its snapshot was refused.
    pub(crate) fn start(&self) -> &Start {
        &self.start
    }

    /// Why the last snapshot could not be written, while no later one was.
    pub(crate) fn snapshot_failure(&self) -> Option<&str> {
        self.snapshot_failure.as_deref()
    }

    /// The number of leaves recorded.
    pub(crate) fn len(&self) -> u64 {
        self.frontier.size()
    }

    /// The recorded size and root.
    pub(crate) fn head(&self) -> (u64, [u8; 32]) {
        (self.frontier.size(), self.frontier.root())
    }

    /// The recorded leaf at `index`.
    pub(crate) fn leaf(&self, index: u64) -> Option<&[u8]> {
        usize::try_from(index)
            .ok()
            .and_then(|slot| self.leaves.get(slot))
            .map(Vec::as_slice)
    }

    /// An inclusion proof of the leaf at `index` in the recorded tree, built
    /// from the held leaves the first time one is asked for.
    pub(crate) fn inclusion_proof(&self, index: u64) -> Result<InclusionProof, K::Error> {
        let prove = |tree: &AppendOnlyTree<RawLeaf>| {
            tree.prove_inclusion(index)
                .map_err(|error| K::unavailable(error.to_string()))
        };
        if let Some(tree) = self.proofs.get() {
            return prove(tree);
        }
        let tree = AppendOnlyTree::<RawLeaf>::reconstruct_from_raw_leaves(&self.leaves);
        let (root, size) = tree.root().to_parts();
        if (size, root) != self.head() {
            return Err(K::unavailable(format!(
                "the held leaves build a tree of size {size} whose root is not the log's"
            )));
        }
        prove(self.proofs.get_or_init(|| tree))
    }

    /// Appends `bytes` as one leaf and records it, then writes a snapshot if
    /// the log crossed a multiple of the cadence. A failure may still have
    /// stored the leaf: nothing is recorded, and [`Ledger::adopt`] resolves it.
    pub(crate) fn append(
        &mut self,
        bytes: &[u8],
        key: &Ed25519Identity,
    ) -> Result<Coordinate, StoreError> {
        let before = self.len();
        let (index, leaf_hash) = self.log.append(bytes)?;
        self.frontier.push_hash(leaf_hash);
        self.leaves.push(bytes.to_vec());
        if let Some(tree) = self.proofs.get_mut() {
            tree.append_raw(bytes);
        }
        let coordinate = Coordinate {
            index,
            tree_size: self.frontier.size(),
            root: self.frontier.root(),
            leaf_hash,
        };
        self.snapshot_if_due(before, key);
        Ok(coordinate)
    }

    /// Reopens the store, reads every leaf after the recorded ones, and
    /// records them, answering each as an event. Nothing is recorded unless
    /// every step succeeds.
    pub(crate) fn adopt(
        &mut self,
        reopen: &Reopen<S>,
        key: &Ed25519Identity,
    ) -> Result<Events<K::Event>, K::Error> {
        let public = key.public_key_bytes();
        let before = self.len();
        let store = reopen().map_err(|error| store_down::<K>(&error))?;
        check_past_pin::<S, K>(&store, &public)?;
        let extent = store.extent();
        if extent < before {
            return Err(K::unavailable(format!(
                "the reopened log holds {extent} leaves, fewer than the {before} it held before the append"
            )));
        }
        let (log, tail) = FrontierLog::resume(store, self.frontier.clone())
            .map_err(|error| store_down::<K>(&error))?;
        let mut frontier = self.frontier.clone();
        let events = fold::<K>(&mut frontier, &tail.leaves, &public, K::verify)?;
        if let Some(tree) = self.proofs.get_mut() {
            for bytes in &tail.leaves {
                tree.append_raw(bytes);
            }
        }
        self.log = log;
        self.frontier = frontier;
        self.leaves.extend(tail.leaves);
        self.snapshot_if_due(before, key);
        Ok(events)
    }

    /// Writes a snapshot when the log has crossed a multiple of the cadence
    /// since it held `before` leaves. A failure is logged and kept by name.
    fn snapshot_if_due(&mut self, before: u64, key: &Ed25519Identity) {
        let every = self.every.get();
        if self.len() / every <= before / every {
            return;
        }
        if let Err(reason) = self.write_snapshot(key) {
            tracing::error!(domain = K::DOMAIN, "SnapshotNotWritten: {reason}");
            self.snapshot_failure = Some(reason);
        }
    }

    /// Seals every recorded leaf as the state at the log's size.
    fn write_snapshot(&mut self, key: &Ed25519Identity) -> Result<(), String> {
        let state = encode_leaves(&self.leaves);
        self.log
            .write_snapshot(K::DOMAIN, &state, key)
            .map_err(|error| error.to_string())?;
        self.snapshot_failure = None;
        Ok(())
    }
}
