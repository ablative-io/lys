//! Commit of one signed event through lys-log-store as one leaf, and
//! reconciliation of an uncertain append (P4, P5).
//!
//! An event is recorded when its leaf is durable in the log and the log's pin
//! covers it, and not before. An append that fails may still have stored its
//! leaf, so a failure is not a refusal: the log holds the append as uncertain,
//! refuses every further append and every read of the log, and resolves it by
//! opening the store again and reading the leaf at that index back. The leaf
//! is there, so the change was recorded, or it is not, so the change was not.
//! Every leaf the reopened log holds from that index on is handed back to be
//! applied, whoever wrote it. Nothing is answered as current while the answer
//! could be either.

use lys_core::merkle::{AppendOnlyTree, InclusionProof, RawLeaf, raw_leaf_hash};
use lys_log_store::{LeafStore, Log, StoreError};

use crate::error::IdentityError;
use crate::signer::{SignedEvent, verify_event};

/// Where an event's leaf stands in the log, as its receipt returns it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Coordinate {
    /// The leaf's index.
    pub index: u64,
    /// The size of the tree the leaf completed.
    pub tree_size: u64,
    /// The root of that tree.
    pub root: [u8; 32],
    /// The RFC 6962 SHA-256 leaf hash of the signed message.
    pub leaf_hash: [u8; 32],
}

/// How an uncertain append was resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolved {
    /// The leaf is in the log: the change was recorded, here.
    Committed(Coordinate),
    /// The leaf is not in the log: the change was not recorded.
    NotCommitted,
}

/// What resolving an uncertain append found, and every leaf it adopted.
///
/// The reopened log may hold leaves this handle never wrote, from another
/// writer on the same store. They are adopted in log order, beside this
/// handle's own leaf when it landed, and the caller applies every one before
/// it answers anything as current.
#[derive(Debug, Clone)]
pub struct Reconciled {
    /// Whether this handle's own leaf was recorded.
    pub resolved: Resolved,
    /// Every leaf from the uncertain index to the end of the reopened log.
    pub adopted: Vec<(SignedEvent, Coordinate)>,
}

/// Opens the log's store afresh, to read back what a failed append left.
pub type Reopen<S> = Box<dyn Fn() -> Result<S, StoreError> + Send + Sync>;

struct Pending {
    index: u64,
    bytes: Vec<u8>,
}

/// The directory's log of signed events.
pub struct EventLog<S: LeafStore> {
    log: Log<S>,
    reopen: Reopen<S>,
    service_key: [u8; 32],
    pending: Option<Pending>,
}

fn unavailable(error: &StoreError) -> IdentityError {
    IdentityError::LogUnavailable {
        reason: error.to_string(),
    }
}

/// Open the store `reopen` gives as a log, refusing before anything is pinned
/// when a leaf past the pin is not a whole event this directory signed.
///
/// `Log::open` pins one leaf past the pin as an interrupted append. A torn
/// or foreign leaf pinned there could never be removed without equivocating,
/// so it is checked here first and refused by name while it is still unpinned.
fn open_checked<S: LeafStore>(
    reopen: &Reopen<S>,
    service_key: &[u8; 32],
) -> Result<Log<S>, IdentityError> {
    let store = reopen().map_err(|error| unavailable(&error))?;
    for index in store.pinned().tree_size..store.extent() {
        let bytes = store
            .leaf(index)
            .map_err(|error| unavailable(&error))?
            .ok_or_else(|| IdentityError::LeafNotAnEvent {
                index,
                reason: "the leaf is missing inside the store's extent".to_owned(),
            })?;
        verify_event(&bytes, service_key).map_err(|error| IdentityError::LeafNotAnEvent {
            index,
            reason: format!("the leaf past the pin was left unpinned: {error}"),
        })?;
    }
    Log::open(store).map_err(|error| unavailable(&error))
}

/// Every leaf of `log` from `from` on, as a verified event with the
/// coordinate it completed.
fn replay<S: LeafStore>(
    log: &Log<S>,
    service_key: &[u8; 32],
    from: u64,
) -> Result<Vec<(SignedEvent, Coordinate)>, IdentityError> {
    let mut tree = AppendOnlyTree::<RawLeaf>::new();
    let mut events = Vec::new();
    for index in 0..log.tree().len() {
        let Some(bytes) = log.leaf_bytes(index) else {
            return Err(IdentityError::LeafNotAnEvent {
                index,
                reason: "the leaf is missing inside the log's extent".to_owned(),
            });
        };
        tree.append_raw(bytes);
        if index < from {
            continue;
        }
        let event =
            verify_event(bytes, service_key).map_err(|error| IdentityError::LeafNotAnEvent {
                index,
                reason: error.to_string(),
            })?;
        let (root, tree_size) = tree.root().to_parts();
        events.push((
            event,
            Coordinate {
                index,
                tree_size,
                root,
                leaf_hash: raw_leaf_hash(bytes),
            },
        ));
    }
    Ok(events)
}

impl<S: LeafStore> EventLog<S> {
    /// Open the log over the store `reopen` gives, verify every leaf against
    /// `service_key`, and return the events in log order with their coordinates.
    pub fn open(
        reopen: Reopen<S>,
        service_key: [u8; 32],
    ) -> Result<(Self, Vec<(SignedEvent, Coordinate)>), IdentityError> {
        let log = open_checked(&reopen, &service_key)?;
        let events = replay(&log, &service_key, 0)?;
        Ok((
            Self {
                log,
                reopen,
                service_key,
                pending: None,
            },
            events,
        ))
    }

    /// Whether an append is held uncertain and every current read must wait for it.
    pub fn is_uncertain(&self) -> bool {
        self.pending.is_some()
    }

    fn certain(&self) -> Result<&Log<S>, IdentityError> {
        match &self.pending {
            Some(pending) => Err(IdentityError::AppendUncertain {
                index: pending.index,
            }),
            None => Ok(&self.log),
        }
    }

    /// The number of leaves in the log, refused while an append is uncertain.
    pub fn len(&self) -> Result<u64, IdentityError> {
        Ok(self.certain()?.tree().len())
    }

    /// Whether the log holds no leaf, refused while an append is uncertain.
    pub fn is_empty(&self) -> Result<bool, IdentityError> {
        Ok(self.certain()?.tree().is_empty())
    }

    /// The log's current size and root, refused while an append is uncertain.
    pub fn head(&self) -> Result<(u64, [u8; 32]), IdentityError> {
        let (root, size) = self.certain()?.tree().root().to_parts();
        Ok((size, root))
    }

    /// The leaf bytes at `index`, if the log holds one there, refused while
    /// an append is uncertain.
    pub fn leaf(&self, index: u64) -> Result<Option<&[u8]>, IdentityError> {
        Ok(self.certain()?.leaf_bytes(index))
    }

    /// An inclusion proof of the leaf at `index` in the log's current tree,
    /// refused while an append is uncertain.
    pub fn inclusion_proof(&self, index: u64) -> Result<InclusionProof, IdentityError> {
        self.certain()?
            .tree()
            .prove_inclusion(index)
            .map_err(|error| IdentityError::LogUnavailable {
                reason: error.to_string(),
            })
    }

    /// Append `event` as one leaf.
    ///
    /// A failed append may still have stored its leaf, so it is held
    /// uncertain and answered `LogUnavailable` with the store's reason; the
    /// caller resolves it with [`EventLog::reconcile`].
    pub fn append(&mut self, event: &SignedEvent) -> Result<Coordinate, IdentityError> {
        let index = self.certain()?.tree().len();
        match self.log.append(event.bytes()) {
            Ok((index, leaf_hash)) => {
                let (root, tree_size) = self.log.tree().root().to_parts();
                Ok(Coordinate {
                    index,
                    tree_size,
                    root,
                    leaf_hash,
                })
            }
            Err(failure) => {
                self.pending = Some(Pending {
                    index,
                    bytes: event.bytes().to_vec(),
                });
                Err(unavailable(&failure))
            }
        }
    }

    /// Resolve a held uncertain append from a fresh open of the store, and
    /// adopt every leaf from its index on. Answers `None` when nothing was
    /// held. The hold stays until every step has succeeded.
    pub fn reconcile(&mut self) -> Result<Option<Reconciled>, IdentityError> {
        let Some(pending) = &self.pending else {
            return Ok(None);
        };
        let log = open_checked(&self.reopen, &self.service_key)?;
        let size = log.tree().len();
        if size < pending.index {
            return Err(IdentityError::LogUnavailable {
                reason: format!(
                    "the reopened log holds {size} leaves, fewer than the {} it held before the append",
                    pending.index
                ),
            });
        }
        let adopted = replay(&log, &self.service_key, pending.index)?;
        let resolved = match adopted.first() {
            Some((signed, coordinate)) if signed.bytes() == pending.bytes.as_slice() => {
                Resolved::Committed(*coordinate)
            }
            _ => Resolved::NotCommitted,
        };
        self.log = log;
        self.pending = None;
        Ok(Some(Reconciled { resolved, adopted }))
    }

    /// The service key every leaf is verified against.
    pub fn service_key(&self) -> &[u8; 32] {
        &self.service_key
    }
}
