//! A signed-event log that starts from its owner's signed snapshot and reads
//! only the leaves after it.
//!
//! The directory and the grants each fold one log of signed events into
//! state: records, the operations that made them, the refusals kept by name,
//! and a receipt per event. A snapshot seals that folded state with the tree
//! size and root it was taken at (`lys/log-snapshot/v1`), so a start reads
//! one snapshot, then only the leaves after it, and folds only those.
//!
//! # What is believed, and when it is refused
//!
//! The snapshot must verify under the service key, name this owner's kind of
//! state and this log, and its frontier must fold to the root it signs. It
//! may claim no more leaves than the log has pinned, and the log's leaves
//! after it must reach the pinned root from its frontier. The owner then
//! requires the state to read back whole and to hold exactly one receipt per
//! leaf, the last completing the snapshot's root. A snapshot failing any
//! check is refused by its [`SnapshotRefusal`] name, logged, and never used:
//! the state is rebuilt from every leaf and a new snapshot is written.
//!
//! # When a snapshot is written
//!
//! When the owner has folded every leaf the log holds and the log has crossed
//! a multiple of the cadence, a count of entries, since the last snapshot;
//! and at once after a rebuild. A snapshot that cannot be written does not
//! undo the append it follows, which is already durable: the failure is
//! logged by name and kept until a later snapshot succeeds.

use std::marker::PhantomData;
use std::num::NonZeroU64;

use lys_core::Ed25519Identity;
use lys_core::merkle::InclusionProof;
use lys_log_store::{Frontier, FrontierLog, LeafStore, SnapshotRefusal, Start, StoreError, unseal};

use crate::log::{Coordinate, Reopen};

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
    /// The log could not be read or written.
    fn unavailable(reason: String) -> Self::Error;
    /// The leaf at `index` is not an event this service signed.
    fn not_an_event(index: u64, reason: String) -> Self::Error;
}

/// Every event of a start or an adoption, in log order, with its coordinate.
pub(crate) type Events<E> = Vec<(E, Coordinate)>;

/// What a start hands its owner to fold.
pub struct Opening<E> {
    /// The owner's state from the snapshot, `None` when the log was rebuilt.
    pub state: Option<Vec<u8>>,
    /// The tree size the state was folded at.
    pub size: u64,
    /// The root at that size.
    pub root: [u8; 32],
    /// The events after the state, in log order, with their coordinates.
    pub events: Vec<(E, Coordinate)>,
}

impl<E> std::fmt::Debug for Opening<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Opening")
            .field("state", &self.state.as_ref().map(Vec::len))
            .field("size", &self.size)
            .field("events", &self.events.len())
            .finish_non_exhaustive()
    }
}

/// A signed-event log and the snapshots written of its owner's state.
pub(crate) struct Ledger<S: LeafStore, K: Leaves> {
    log: FrontierLog<S>,
    frontier: Frontier,
    every: NonZeroU64,
    start: Start,
    snapshot_at: u64,
    snapshot_owed: bool,
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

/// Extends `frontier` by `leaves`, which start at its size, verifying each
/// and answering each event with the coordinate it completed.
fn fold<K: Leaves>(
    frontier: &mut Frontier,
    leaves: &[Vec<u8>],
    key: &[u8; 32],
) -> Result<Events<K::Event>, K::Error> {
    let mut events = Vec::with_capacity(leaves.len());
    for bytes in leaves {
        let index = frontier.size();
        let leaf_hash = frontier.push(bytes);
        let event =
            K::verify(bytes, key).map_err(|error| K::not_an_event(index, error.to_string()))?;
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

/// What a start from the snapshot found: the ledger and the opening, or the
/// refusal that sends the start to the whole log.
type Resumed<S, K> = Result<(Ledger<S, K>, Opening<<K as Leaves>::Event>), SnapshotRefusal>;

impl<S: LeafStore, K: Leaves> Ledger<S, K> {
    /// Opens the log `reopen` gives from its owner's snapshot, reading only
    /// the leaves after it, or from every leaf when the snapshot is refused.
    pub(crate) fn open(
        reopen: &Reopen<S>,
        key: &Ed25519Identity,
        every: NonZeroU64,
    ) -> Result<(Self, Opening<K::Event>), K::Error> {
        let public = key.public_key_bytes();
        let store = reopen().map_err(|error| store_down::<K>(&error))?;
        check_past_pin::<S, K>(&store, &public)?;
        match Self::resume(store, &public, every)? {
            Ok((ledger, opening)) => {
                tracing::info!(domain = K::DOMAIN, "{}", ledger.start);
                Ok((ledger, opening))
            }
            Err(refusal) => Self::rebuild(reopen, refusal, &public, every),
        }
    }

    fn resume(store: S, key: &[u8; 32], every: NonZeroU64) -> Result<Resumed<S, K>, K::Error> {
        let sealed = match store.snapshot() {
            Ok(Some(sealed)) => sealed,
            Ok(None) => return Ok(Err(SnapshotRefusal::Missing)),
            Err(error) => return Err(store_down::<K>(&error)),
        };
        let snapshot = match unseal(&sealed, K::DOMAIN, store.origin(), key) {
            Ok(snapshot) => snapshot,
            Err(refusal) => return Ok(Err(refusal)),
        };
        let (size, pinned) = (snapshot.frontier().size(), store.pinned().tree_size);
        if size > pinned {
            return Ok(Err(SnapshotRefusal::BeyondLog { size, pinned }));
        }
        let (frontier, state) = snapshot.into_parts();
        let root = frontier.root();
        let (log, tail) = match FrontierLog::resume(store, frontier.clone()) {
            Ok(resumed) => resumed,
            Err(StoreError::PinMismatch { .. }) => {
                return Ok(Err(SnapshotRefusal::WrongRoot { size }));
            }
            Err(error) => return Err(store_down::<K>(&error)),
        };
        let mut frontier = frontier;
        let events = fold::<K>(&mut frontier, &tail.leaves, key)?;
        let replayed = u64::try_from(tail.leaves.len()).unwrap_or(u64::MAX);
        let ledger = Self::new(
            log,
            frontier,
            every,
            Start::Resumed { size, replayed },
            size,
        );
        Ok(Ok((
            ledger,
            Opening {
                state: Some(state),
                size,
                root,
                events,
            },
        )))
    }

    fn new(
        log: FrontierLog<S>,
        frontier: Frontier,
        every: NonZeroU64,
        start: Start,
        snapshot_at: u64,
    ) -> Self {
        let snapshot_owed = matches!(start, Start::Rebuilt { .. });
        Self {
            log,
            frontier,
            every,
            start,
            snapshot_at,
            snapshot_owed,
            snapshot_failure: None,
            kind: PhantomData,
        }
    }

    /// Opens the log from every leaf because `refusal` refused its snapshot,
    /// and logs the refusal. The owner folds every event and a snapshot is
    /// owed at once.
    fn rebuild(
        reopen: &Reopen<S>,
        refusal: SnapshotRefusal,
        key: &[u8; 32],
        every: NonZeroU64,
    ) -> Result<(Self, Opening<K::Event>), K::Error> {
        let store = reopen().map_err(|error| store_down::<K>(&error))?;
        let (log, tail) = FrontierLog::open(store).map_err(|error| store_down::<K>(&error))?;
        let mut frontier = Frontier::new();
        let events = fold::<K>(&mut frontier, &tail.leaves, key)?;
        let replayed = u64::try_from(tail.leaves.len()).unwrap_or(u64::MAX);
        let start = Start::Rebuilt { refusal, replayed };
        tracing::warn!(domain = K::DOMAIN, "{start}");
        let empty = Frontier::new().root();
        let ledger = Self::new(log, frontier, every, start, 0);
        Ok((
            ledger,
            Opening {
                state: None,
                size: 0,
                root: empty,
                events,
            },
        ))
    }

    /// Refuses the state the owner could not read from the snapshot, by
    /// `reason`, and reopens the log from every leaf.
    pub(crate) fn refuse_state(
        &mut self,
        reopen: &Reopen<S>,
        reason: String,
        key: &Ed25519Identity,
    ) -> Result<Events<K::Event>, K::Error> {
        let refusal = SnapshotRefusal::StateUnreadable { reason };
        let (ledger, opening) =
            Self::rebuild(reopen, refusal, &key.public_key_bytes(), self.every)?;
        *self = ledger;
        Ok(opening.events)
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

    /// The recorded leaf at `index`, read from the store.
    pub(crate) fn leaf(&self, index: u64) -> Result<Option<Vec<u8>>, K::Error> {
        if index >= self.len() {
            return Ok(None);
        }
        self.log
            .leaf_bytes(index)
            .map_err(|error| store_down::<K>(&error))
    }

    /// The recorded event at `index`, read from the store and verified.
    pub(crate) fn event(&self, index: u64, key: &[u8; 32]) -> Result<Option<K::Event>, K::Error> {
        self.leaf(index)?
            .map(|bytes| {
                K::verify(&bytes, key).map_err(|error| K::not_an_event(index, error.to_string()))
            })
            .transpose()
    }

    /// An inclusion proof of the leaf at `index` in the recorded tree. The
    /// proof tree is built from the stored leaves the first time one is asked
    /// for, never at a start.
    pub(crate) fn inclusion_proof(&self, index: u64) -> Result<InclusionProof, K::Error> {
        self.log
            .proof_tree()
            .map_err(|error| store_down::<K>(&error))?
            .prove_inclusion(index)
            .map_err(|error| K::unavailable(error.to_string()))
    }

    /// Appends `bytes` as one leaf and records it. A failure may still have
    /// stored the leaf: nothing is recorded, and [`Ledger::adopt`] resolves it.
    pub(crate) fn append(&mut self, bytes: &[u8]) -> Result<Coordinate, StoreError> {
        let (index, leaf_hash) = self.log.append(bytes)?;
        self.frontier.push_hash(leaf_hash);
        Ok(Coordinate {
            index,
            tree_size: self.frontier.size(),
            root: self.frontier.root(),
            leaf_hash,
        })
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
        let events = fold::<K>(&mut frontier, &tail.leaves, &public)?;
        self.log = log;
        self.frontier = frontier;
        Ok(events)
    }

    /// Writes a snapshot of the owner's state when one is owed or the log has
    /// crossed a multiple of the cadence since the last. The owner calls this
    /// only when `encode` gives the fold of every leaf the log holds. A
    /// failure is logged and kept by name.
    pub(crate) fn snapshot_if_due(
        &mut self,
        key: &Ed25519Identity,
        encode: impl FnOnce() -> Result<Vec<u8>, String>,
    ) {
        let every = self.every.get();
        if !self.snapshot_owed && self.len() / every <= self.snapshot_at / every {
            return;
        }
        let written = encode().and_then(|state| {
            self.log
                .write_snapshot(K::DOMAIN, &state, key)
                .map_err(|error| error.to_string())
        });
        match written {
            Ok(size) => {
                self.snapshot_at = size;
                self.snapshot_owed = false;
                self.snapshot_failure = None;
            }
            Err(reason) => {
                tracing::error!(domain = K::DOMAIN, "SnapshotNotWritten: {reason}");
                self.snapshot_failure = Some(reason);
            }
        }
    }
}
