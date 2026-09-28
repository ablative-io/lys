//! Folding committed grant events into the book, projecting
//! them into the permission relationships, and committing new ones.
//!
//! The grants hold the book and the number of leaves folded; the events and
//! their receipts stay in the log and are built from it only where an answer
//! must carry one. A snapshot of the book is written when one is owed and
//! every leaf is folded.

use std::collections::{BTreeMap, BTreeSet};

use lys_log_store::LeafStore;

use super::admission::Route;
use super::authority::{Grants, Recorded};
use super::error::GrantError;
use super::events::{GrantChange, GrantEvent, SignedGrantEvent, sign_grant_event};
use super::permission::{Relationship, RelationshipStore, naming, relationships_of};
use super::projection::Changed;
use super::receipt::GrantReceipt;
use super::state;
use super::types::GrantId;
use crate::id::IdentityId;
use crate::log::Coordinate;
use crate::operation::OperationId;

/// The relationships to write and to delete for one event.
type Delta = (Vec<Relationship>, Vec<Relationship>);

fn not_held(index: u64) -> GrantError {
    GrantError::LogUnavailable {
        reason: format!("leaf {index} is not held"),
    }
}

impl<S: LeafStore, R: RelationshipStore> Grants<S, R> {
    /// Apply a committed event to the book, or keep the book's refusal of it
    /// by name, and count it folded.
    pub(super) fn record_committed(
        &mut self,
        signed: &SignedGrantEvent,
        coordinate: Coordinate,
    ) -> Result<(), GrantError> {
        let index = self.folded;
        if coordinate.index != index {
            return Err(GrantError::LogUnavailable {
                reason: format!(
                    "leaf {} arrived where leaf {index} was expected",
                    coordinate.index
                ),
            });
        }
        if let Err(refusal) = self.book.apply(signed.event(), index) {
            self.book.refuse(signed.event(), index, refusal);
        }
        self.folded += 1;
        Ok(())
    }

    /// Write a snapshot of the book when one is owed. Called only when every
    /// leaf of the log is folded.
    pub(super) fn snapshot(&mut self) {
        let (book, folded) = (&self.book, self.folded);
        self.ledger
            .snapshot_if_due(&self.key, || state::encode(book, folded));
    }

    /// Resolve an uncertain append, and apply every leaf it adopted.
    pub fn settle_log(&mut self) -> Result<(), GrantError> {
        if let Some(adopted) = self.ledger.reconcile(&self.key)? {
            for (signed, coordinate) in adopted {
                self.record_committed(&signed, coordinate)?;
            }
            self.snapshot();
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

    /// The relationships the event at `index` writes and deletes, judged from
    /// what the book records at that index and the book as it stands.
    fn delta(&self, index: u64, changes: &BTreeMap<u64, Changed>) -> Result<Delta, GrantError> {
        if self.book.refused().contains_key(&index) {
            return Ok((Vec::new(), Vec::new()));
        }
        match changes.get(&index) {
            Some(Changed::Issued(id)) => {
                let grant = self
                    .book
                    .grant(*id)
                    .ok_or_else(|| GrantError::GrantUnknown {
                        grant: id.to_string(),
                    })?;
                let live = match self.book.lineage(*id) {
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
            Some(Changed::Revoked(id)) => {
                let mut tree: BTreeSet<GrantId> = self.book.descendants(*id).into_iter().collect();
                tree.insert(*id);
                Ok((Vec::new(), naming(&self.relationships.read()?, &tree)))
            }
            None => Ok((Vec::new(), Vec::new())),
        }
    }

    /// Project every committed event the relationships do not yet reflect, in
    /// log order, answering the revision they stand at. What each event did
    /// is read from the book, never from the log.
    pub fn project(&mut self) -> Result<u64, GrantError> {
        let from = self.relationships.revision()?;
        if from >= self.folded {
            return Ok(from);
        }
        let changes = self.book.changes_from(from);
        loop {
            let revision = self.relationships.revision()?;
            if revision >= self.folded {
                return Ok(revision);
            }
            let (touch, delete) = self.delta(revision, &changes)?;
            self.relationships.write(revision + 1, &touch, &delete)?;
        }
    }

    /// The event an operation recorded and its receipt, if it recorded one,
    /// built from the log from the nearest checkpoint.
    pub(super) fn answered(
        &self,
        operation: OperationId,
    ) -> Result<Option<(GrantEvent, GrantReceipt)>, GrantError> {
        let Some(index) = self.book.operation(operation) else {
            return Ok(None);
        };
        let (signed, coordinate) = self.ledger.entry(index)?.ok_or_else(|| not_held(index))?;
        Ok(Some((
            signed.event().clone(),
            GrantReceipt::of(&signed, coordinate),
        )))
    }

    /// Answer the committed `event` with its `receipt` once it is projected,
    /// or name what is pending.
    pub(super) fn answer(
        &mut self,
        event: GrantEvent,
        receipt: GrantReceipt,
    ) -> Result<Recorded, GrantError> {
        let operation = event.operation();
        let index = self
            .book
            .operation(operation)
            .ok_or_else(|| GrantError::OperationReused {
                operation: operation.to_string(),
            })?;
        if let Some((_, refusal)) = self.book.refused().get(&index) {
            return Err(refusal.clone());
        }
        if receipt.coordinate.index != index {
            return Err(GrantError::LogUnavailable {
                reason: format!(
                    "operation {operation} names leaf {index}, and its receipt leaf {}",
                    receipt.coordinate.index
                ),
            });
        }
        let projected = match self.project() {
            Ok(projected) => projected,
            Err(_) => self.relationships.revision().unwrap_or(0),
        };
        if projected > index {
            Ok(Recorded {
                event,
                index,
                receipt,
            })
        } else {
            Err(GrantError::ProjectionPending {
                operation: operation.to_string(),
                grant: event.grant().to_string(),
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
                self.record_committed(&signed, coordinate)?;
                self.snapshot();
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
                self.record_committed(&signed, coordinate)?;
                self.snapshot();
                let receipt = GrantReceipt::of(&signed, coordinate);
                self.answer(signed.event().clone(), receipt)
            }
            Err(failure) => {
                if self.ledger.uncertain().is_none() {
                    return Err(GrantError::AppendRefused {
                        reason: failure.to_string(),
                    });
                }
                self.settle_for_change()?;
                match self.answered(operation)? {
                    Some((recorded, receipt)) => self.answer(recorded, receipt),
                    None => Err(GrantError::AppendRefused {
                        reason: failure.to_string(),
                    }),
                }
            }
        }
    }
}
