//! The grant log: each signed grant event committed through lys-log-store as
//! one leaf, and an uncertain append resolved by reading it back.
//!
//! A grant change is recorded when its leaf is durable in the log and the
//! log's pin covers it, and not before. A failed append may still have stored
//! its leaf, so the log holds it as uncertain, with its operation and grant,
//! refuses every further append, and resolves it from a fresh open of the
//! store: the leaf is there, so the change was recorded, or it is not. Every
//! leaf the reopened log holds from that index on is handed back to be
//! applied, whoever wrote it.

use std::collections::BTreeSet;

use lys_core::merkle::{AppendOnlyTree, InclusionProof, RawLeaf, raw_leaf_hash};
use lys_log_store::{LeafStore, Log, StoreError};

use super::admission::Route;
use super::authority::{Grants, Recorded};
use super::error::GrantError;
use super::events::{
    GrantChange, GrantEvent, SignedGrantEvent, sign_grant_event, verify_grant_event,
};
use super::permission::{Relationship, RelationshipStore, naming, relationships_of};
use super::receipt::GrantReceipt;
use super::types::GrantId;
use crate::id::IdentityId;
use crate::log::{Coordinate, Reopen};
use crate::operation::OperationId;

/// An append whose outcome is not yet known.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Uncertain {
    /// The index the leaf was written at.
    pub index: u64,
    /// The operation it records.
    pub operation: OperationId,
    /// The grant it changes.
    pub grant: GrantId,
    /// The event it would record.
    pub event: GrantEvent,
}

/// The grant log.
pub struct GrantLedger<S: LeafStore> {
    log: Log<S>,
    reopen: Reopen<S>,
    service_key: [u8; 32],
    uncertain: Option<Uncertain>,
}

fn unavailable(error: &StoreError) -> GrantError {
    GrantError::LogUnavailable {
        reason: error.to_string(),
    }
}

/// Open the store `reopen` gives, refusing before anything is pinned when a
/// leaf past the pin is not a whole grant event this service signed.
fn open_checked<S: LeafStore>(
    reopen: &Reopen<S>,
    service_key: &[u8; 32],
) -> Result<Log<S>, GrantError> {
    let store = reopen().map_err(|error| unavailable(&error))?;
    for index in store.pinned().tree_size..store.extent() {
        let bytes = store
            .leaf(index)
            .map_err(|error| unavailable(&error))?
            .ok_or_else(|| GrantError::LeafNotAnEvent {
                index,
                reason: "the leaf is missing inside the store's extent".to_owned(),
            })?;
        verify_grant_event(&bytes, service_key).map_err(|error| GrantError::LeafNotAnEvent {
            index,
            reason: format!("the leaf past the pin was left unpinned: {error}"),
        })?;
    }
    Log::open(store).map_err(|error| unavailable(&error))
}

/// Every leaf of `log` from `from` on, verified, with the coordinate it completed.
fn replay<S: LeafStore>(
    log: &Log<S>,
    service_key: &[u8; 32],
    from: u64,
) -> Result<Vec<(SignedGrantEvent, Coordinate)>, GrantError> {
    let mut tree = AppendOnlyTree::<RawLeaf>::new();
    let mut events = Vec::new();
    for index in 0..log.tree().len() {
        let bytes = log
            .leaf_bytes(index)
            .ok_or_else(|| GrantError::LeafNotAnEvent {
                index,
                reason: "the leaf is missing inside the log's extent".to_owned(),
            })?;
        tree.append_raw(bytes);
        if index < from {
            continue;
        }
        let event =
            verify_grant_event(bytes, service_key).map_err(|error| GrantError::LeafNotAnEvent {
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

impl<S: LeafStore> GrantLedger<S> {
    /// Open the log over the store `reopen` gives, verify every leaf, and
    /// return the events in log order with their coordinates.
    pub fn open(
        reopen: Reopen<S>,
        service_key: [u8; 32],
    ) -> Result<(Self, Vec<(SignedGrantEvent, Coordinate)>), GrantError> {
        let log = open_checked(&reopen, &service_key)?;
        let events = replay(&log, &service_key, 0)?;
        Ok((
            Self {
                log,
                reopen,
                service_key,
                uncertain: None,
            },
            events,
        ))
    }

    /// The append held uncertain, if one is.
    pub fn uncertain(&self) -> Option<&Uncertain> {
        self.uncertain.as_ref()
    }

    fn certain(&self) -> Result<&Log<S>, GrantError> {
        match &self.uncertain {
            Some(held) => Err(GrantError::OperationUnresolved {
                operation: held.operation.to_string(),
                grant: held.grant.to_string(),
            }),
            None => Ok(&self.log),
        }
    }

    /// The log's current size and root, refused while an append is uncertain.
    pub fn head(&self) -> Result<(u64, [u8; 32]), GrantError> {
        let (root, size) = self.certain()?.tree().root().to_parts();
        Ok((size, root))
    }

    /// An inclusion proof of the leaf at `index` in the log's current tree.
    pub fn inclusion_proof(&self, index: u64) -> Result<InclusionProof, GrantError> {
        self.certain()?
            .tree()
            .prove_inclusion(index)
            .map_err(|error| GrantError::LogUnavailable {
                reason: error.to_string(),
            })
    }

    /// Append `event` as one leaf. A failed append is held uncertain and
    /// answered `LogUnavailable`; [`GrantLedger::reconcile`] resolves it.
    pub fn append(&mut self, event: &SignedGrantEvent) -> Result<Coordinate, GrantError> {
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
                self.uncertain = Some(Uncertain {
                    index,
                    operation: event.event().operation(),
                    grant: event.event().grant(),
                    event: event.event().clone(),
                });
                Err(unavailable(&failure))
            }
        }
    }

    /// Resolve a held uncertain append from a fresh open of the store, and
    /// answer every leaf from its index on. Answers `None` when nothing was
    /// held. The hold stays until every step has succeeded.
    pub fn reconcile(&mut self) -> Result<Option<Vec<(SignedGrantEvent, Coordinate)>>, GrantError> {
        let Some(held) = &self.uncertain else {
            return Ok(None);
        };
        let log = open_checked(&self.reopen, &self.service_key)?;
        let size = log.tree().len();
        if size < held.index {
            return Err(GrantError::LogUnavailable {
                reason: format!(
                    "the reopened log holds {size} leaves, fewer than the {} it held before the append",
                    held.index
                ),
            });
        }
        let adopted = replay(&log, &self.service_key, held.index)?;
        self.log = log;
        self.uncertain = None;
        Ok(Some(adopted))
    }
}

/// The relationships to write and to delete for one event.
type Delta = (Vec<Relationship>, Vec<Relationship>);

impl<S: LeafStore, R: RelationshipStore> Grants<S, R> {
    /// Apply a committed event to the book, or keep the book's refusal of it
    /// by name, and keep the event with its receipt.
    pub(super) fn record_committed(
        &mut self,
        signed: SignedGrantEvent,
        coordinate: Coordinate,
    ) -> Result<(), GrantError> {
        let index = self.events.len() as u64;
        if coordinate.index != index {
            return Err(GrantError::LogUnavailable {
                reason: format!(
                    "leaf {} arrived where leaf {index} was expected",
                    coordinate.index
                ),
            });
        }
        let receipt = GrantReceipt::of(&signed, coordinate);
        if let Err(refusal) = self.book.apply(signed.event(), index) {
            self.book.refuse(signed.event(), index, refusal);
        }
        self.events.push((signed, receipt));
        Ok(())
    }

    /// Resolve an uncertain append, and apply every leaf it adopted.
    pub fn settle_log(&mut self) -> Result<(), GrantError> {
        if let Some(adopted) = self.ledger.reconcile()? {
            for (signed, coordinate) in adopted {
                self.record_committed(signed, coordinate)?;
            }
        }
        Ok(())
    }

    /// Settle the log before a change, refusing by the held operation's name
    /// while it cannot be resolved.
    pub(super) fn settle_for_change(&mut self) -> Result<(), GrantError> {
        match self.settle_log() {
            Ok(()) => Ok(()),
            Err(error) => match self.ledger.uncertain() {
                Some(held) => Err(GrantError::OperationUnresolved {
                    operation: held.operation.to_string(),
                    grant: held.grant.to_string(),
                }),
                None => Err(error),
            },
        }
    }

    fn delta(&self, event: &GrantEvent, index: u64) -> Result<Delta, GrantError> {
        if self.book.refused().contains_key(&index) {
            return Ok((Vec::new(), Vec::new()));
        }
        match event.change() {
            GrantChange::Issue(grant) => {
                let live = match self.book.lineage(grant.id()) {
                    Ok(lineage) => lineage.path.iter().all(|id| {
                        self.book
                            .record(*id)
                            .is_some_and(|record| record.revoked().is_none())
                    }),
                    Err(_) => false,
                };
                if live {
                    Ok((relationships_of(grant), Vec::new()))
                } else {
                    Ok((Vec::new(), Vec::new()))
                }
            }
            GrantChange::Revoke { grant, .. } => {
                let mut tree: BTreeSet<GrantId> =
                    self.book.descendants(*grant).into_iter().collect();
                tree.insert(*grant);
                Ok((Vec::new(), naming(&self.relationships.read()?, &tree)))
            }
            GrantChange::Use { .. } => Ok((Vec::new(), Vec::new())),
        }
    }

    /// Project every committed event the relationships do not yet reflect, in
    /// log order, answering the revision they stand at.
    pub fn project(&mut self) -> Result<u64, GrantError> {
        loop {
            let revision = self.relationships.revision()?;
            let Some((signed, _)) = usize::try_from(revision)
                .ok()
                .and_then(|index| self.events.get(index))
            else {
                return Ok(revision);
            };
            let (touch, delete) = self.delta(signed.event(), revision)?;
            self.relationships.write(revision + 1, &touch, &delete)?;
        }
    }

    /// The event an operation recorded, if it recorded one.
    pub(super) fn answered_event(&self, operation: OperationId) -> Option<GrantEvent> {
        let index = usize::try_from(self.book.operation(operation)?).ok()?;
        self.events
            .get(index)
            .map(|(signed, _)| signed.event().clone())
    }

    /// Answer a committed operation once its event is projected, or name what is pending.
    pub(super) fn answer(&mut self, operation: OperationId) -> Result<Recorded, GrantError> {
        let index = self
            .book
            .operation(operation)
            .ok_or_else(|| GrantError::OperationReused {
                operation: operation.to_string(),
            })?;
        if let Some((_, refusal)) = self.book.refused().get(&index) {
            return Err(refusal.clone());
        }
        let (signed, receipt) = usize::try_from(index)
            .ok()
            .and_then(|slot| self.events.get(slot))
            .cloned()
            .ok_or_else(|| GrantError::LogUnavailable {
                reason: format!("operation {operation} names leaf {index}, which is not held"),
            })?;
        let projected = match self.project() {
            Ok(projected) => projected,
            Err(_) => self.relationships.revision().unwrap_or(0),
        };
        if projected > index {
            Ok(Recorded {
                event: signed.event().clone(),
                index,
                receipt,
            })
        } else {
            Err(GrantError::ProjectionPending {
                operation: operation.to_string(),
                grant: signed.event().grant().to_string(),
                index,
            })
        }
    }

    /// Record in the grant log that `holder` exercised `grant` through
    /// `route` at `at`, answering the use event's index. The event needs no
    /// projection: it changes no relationship.
    pub(super) fn record_use(
        &mut self,
        holder: IdentityId,
        grant: GrantId,
        route: Route,
        at: u64,
    ) -> Result<u64, GrantError> {
        let event = GrantEvent::new(
            OperationId::generate()?,
            holder,
            at,
            GrantChange::Use { grant, route },
        )?;
        self.book.check(&event)?;
        let operation = event.operation();
        let signed = sign_grant_event(event, &self.key)?;
        match self.ledger.append(&signed) {
            Ok(coordinate) => {
                let index = coordinate.index;
                self.record_committed(signed, coordinate)?;
                Ok(index)
            }
            Err(failure) => {
                if self.ledger.uncertain().is_some() {
                    self.settle_for_change()?;
                }
                self.book
                    .operation(operation)
                    .ok_or_else(|| GrantError::AppendRefused {
                        reason: failure.to_string(),
                    })
            }
        }
    }

    /// Sign, append and apply one judged change, holding an uncertain append
    /// until it is resolved and never answering before the event is recorded.
    pub(super) fn commit(&mut self, event: GrantEvent) -> Result<Recorded, GrantError> {
        self.book.check(&event)?;
        let operation = event.operation();
        let signed = sign_grant_event(event, &self.key)?;
        match self.ledger.append(&signed) {
            Ok(coordinate) => {
                self.record_committed(signed, coordinate)?;
                self.answer(operation)
            }
            Err(failure) => {
                if self.ledger.uncertain().is_none() {
                    return Err(GrantError::AppendRefused {
                        reason: failure.to_string(),
                    });
                }
                self.settle_for_change()?;
                if self.book.operation(operation).is_some() {
                    self.answer(operation)
                } else {
                    Err(GrantError::AppendRefused {
                        reason: failure.to_string(),
                    })
                }
            }
        }
    }
}
