//! Folding committed grant events into the book and receipts, projecting
//! them into the permission relationships, and committing new ones.
//!
//! The grants hold the book and one receipt per leaf; the events themselves
//! stay in the log and are read back only where an answer must carry one. A
//! snapshot of the book and receipts is written when one is owed and every
//! leaf is folded.

use std::collections::BTreeSet;

use lys_log_store::LeafStore;

use super::admission::Route;
use super::authority::{Grants, Recorded};
use super::error::GrantError;
use super::events::{
    GrantChange, GrantEvent, ISSUE, REVOKE, SignedGrantEvent, USE, sign_grant_event,
};
use super::permission::{Relationship, RelationshipStore, naming, relationships_of};
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
    /// by name, and keep its receipt.
    pub(super) fn record_committed(
        &mut self,
        signed: &SignedGrantEvent,
        coordinate: Coordinate,
    ) -> Result<(), GrantError> {
        let index = self.receipts.len() as u64;
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
        self.receipts.push(GrantReceipt::of(signed, coordinate));
        Ok(())
    }

    /// Write a snapshot of the book and receipts when one is owed. Called
    /// only when every leaf of the log is folded.
    pub(super) fn snapshot(&mut self) {
        let (book, receipts) = (&self.book, &self.receipts);
        self.ledger
            .snapshot_if_due(&self.key, || state::encode(book, receipts));
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
    /// its receipt and the book as it stands.
    fn delta(&self, index: u64) -> Result<Delta, GrantError> {
        if self.book.refused().contains_key(&index) {
            return Ok((Vec::new(), Vec::new()));
        }
        let receipt = usize::try_from(index)
            .ok()
            .and_then(|slot| self.receipts.get(slot))
            .ok_or_else(|| not_held(index))?;
        match receipt.change_kind {
            ISSUE => {
                let grant =
                    self.book
                        .grant(receipt.grant)
                        .ok_or_else(|| GrantError::GrantUnknown {
                            grant: receipt.grant.to_string(),
                        })?;
                let live = match self.book.lineage(receipt.grant) {
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
            REVOKE => {
                let mut tree: BTreeSet<GrantId> =
                    self.book.descendants(receipt.grant).into_iter().collect();
                tree.insert(receipt.grant);
                Ok((Vec::new(), naming(&self.relationships.read()?, &tree)))
            }
            USE => Ok((Vec::new(), Vec::new())),
            other => Err(GrantError::LogUnavailable {
                reason: format!("leaf {index} names change kind {other}"),
            }),
        }
    }

    /// Project every committed event the relationships do not yet reflect, in
    /// log order, answering the revision they stand at.
    pub fn project(&mut self) -> Result<u64, GrantError> {
        loop {
            let revision = self.relationships.revision()?;
            if revision >= self.revision() {
                return Ok(revision);
            }
            let (touch, delete) = self.delta(revision)?;
            self.relationships.write(revision + 1, &touch, &delete)?;
        }
    }

    /// The event an operation recorded, if it recorded one, read back from
    /// the log.
    pub(super) fn answered_event(
        &self,
        operation: OperationId,
    ) -> Result<Option<GrantEvent>, GrantError> {
        let Some(index) = self.book.operation(operation) else {
            return Ok(None);
        };
        let signed = self.ledger.event(index)?.ok_or_else(|| not_held(index))?;
        Ok(Some(signed.event().clone()))
    }

    /// Answer the committed `event` once it is projected, or name what is pending.
    pub(super) fn answer(&mut self, event: GrantEvent) -> Result<Recorded, GrantError> {
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
        let receipt = usize::try_from(index)
            .ok()
            .and_then(|slot| self.receipts.get(slot))
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
                self.answer(signed.event().clone())
            }
            Err(failure) => {
                if self.ledger.uncertain().is_none() {
                    return Err(GrantError::AppendRefused {
                        reason: failure.to_string(),
                    });
                }
                self.settle_for_change()?;
                match self.answered_event(operation)? {
                    Some(recorded) => self.answer(recorded),
                    None => Err(GrantError::AppendRefused {
                        reason: failure.to_string(),
                    }),
                }
            }
        }
    }
}
