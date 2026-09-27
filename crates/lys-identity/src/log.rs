//! Commit of one signed event through lys-log-store as one leaf, and
//! reconciliation of an uncertain append (P4, P5).
//!
//! An event is recorded when its leaf is durable in the log and the log's pin
//! covers it, and not before. An append that fails may still have stored its
//! leaf, so a failure is not a refusal: the log holds the append as uncertain,
//! refuses every further append and every read that asks for the current
//! state, and resolves it by opening the store again and reading the leaf at
//! that index back. The leaf is there, so the change was recorded, or it is not,
//! so the change was not. Nothing is answered as current while the answer
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

/// Every leaf of `log` as a verified event with the coordinate it completed.
fn replay<S: LeafStore>(
    log: &Log<S>,
    service_key: &[u8; 32],
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
        let event =
            verify_event(bytes, service_key).map_err(|error| IdentityError::LeafNotAnEvent {
                index,
                reason: error.to_string(),
            })?;
        tree.append_raw(bytes);
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
        let log = Log::open(reopen().map_err(|error| unavailable(&error))?)
            .map_err(|error| unavailable(&error))?;
        let events = replay(&log, &service_key)?;
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

    /// The number of leaves in the log.
    pub fn len(&self) -> u64 {
        self.log.tree().len()
    }

    /// Whether the log holds no leaf.
    pub fn is_empty(&self) -> bool {
        self.log.tree().is_empty()
    }

    /// The log's current size and root.
    pub fn head(&self) -> (u64, [u8; 32]) {
        let (root, size) = self.log.tree().root().to_parts();
        (size, root)
    }

    /// The leaf bytes at `index`, if the log holds one there.
    pub fn leaf(&self, index: u64) -> Option<&[u8]> {
        self.log.leaf_bytes(index)
    }

    /// An inclusion proof of the leaf at `index` in the log's current tree.
    pub fn inclusion_proof(&self, index: u64) -> Result<InclusionProof, IdentityError> {
        self.log
            .tree()
            .prove_inclusion(index)
            .map_err(|error| IdentityError::LogUnavailable {
                reason: error.to_string(),
            })
    }

    /// Append `event` as one leaf. A failed append is resolved before this
    /// returns if the store can be read, and held uncertain if it cannot.
    pub fn append(&mut self, event: &SignedEvent) -> Result<Coordinate, IdentityError> {
        if let Some(pending) = &self.pending {
            return Err(IdentityError::AppendUncertain {
                index: pending.index,
            });
        }
        let index = self.log.tree().len();
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
                match self.reconcile()? {
                    Some(Resolved::Committed(coordinate)) => Ok(coordinate),
                    Some(Resolved::NotCommitted) | None => Err(IdentityError::AppendRefused {
                        reason: failure.to_string(),
                    }),
                }
            }
        }
    }

    /// Resolve a held uncertain append by reading its leaf back from a fresh
    /// open of the store. Answers `None` when nothing was held.
    pub fn reconcile(&mut self) -> Result<Option<Resolved>, IdentityError> {
        let Some(pending) = self.pending.take() else {
            return Ok(None);
        };
        let reopened = (self.reopen)()
            .map_err(|error| unavailable(&error))
            .and_then(|store| Log::open(store).map_err(|error| unavailable(&error)));
        let log = match reopened {
            Ok(log) => log,
            Err(error) => {
                self.pending = Some(pending);
                return Err(error);
            }
        };
        self.log = log;
        if self.log.leaf_bytes(pending.index) != Some(pending.bytes.as_slice()) {
            return Ok(Some(Resolved::NotCommitted));
        }
        let prefix = self
            .log
            .prefix_tree(pending.index + 1)
            .map_err(|error| unavailable(&error))?;
        let (root, tree_size) = prefix.root().to_parts();
        Ok(Some(Resolved::Committed(Coordinate {
            index: pending.index,
            tree_size,
            root,
            leaf_hash: raw_leaf_hash(&pending.bytes),
        })))
    }

    /// The service key every leaf is verified against.
    pub fn service_key(&self) -> &[u8; 32] {
        &self.service_key
    }
}
