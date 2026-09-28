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
use std::num::NonZeroU64;

use lys_core::Ed25519Identity;
use lys_core::merkle::InclusionProof;
use lys_log_store::{LeafStore, Start};

use super::admission::Route;
use super::authority::{Grants, Recorded};
use super::error::GrantError;
use super::events::{
    GrantChange, GrantEvent, SignedGrantEvent, read_attested_grant_event, sign_grant_event,
    verify_grant_event,
};
use super::permission::{Relationship, RelationshipStore, naming, relationships_of};
use super::receipt::GrantReceipt;
use super::types::GrantId;
use crate::id::IdentityId;
use crate::log::{Coordinate, Reopen};
use crate::operation::OperationId;
use crate::restart::{Leaves, Ledger};

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

/// The grant log's leaves: grant events signed by the service key.
pub(crate) struct GrantLeaves;

impl Leaves for GrantLeaves {
    type Event = SignedGrantEvent;
    type Error = GrantError;
    const DOMAIN: &'static str = "lys/identity-grants/v1";

    fn verify(bytes: &[u8], key: &[u8; 32]) -> Result<SignedGrantEvent, GrantError> {
        verify_grant_event(bytes, key)
    }

    fn attested(bytes: &[u8], key: &[u8; 32]) -> Result<SignedGrantEvent, GrantError> {
        read_attested_grant_event(bytes, key)
    }

    fn unavailable(reason: String) -> GrantError {
        GrantError::LogUnavailable { reason }
    }

    fn not_an_event(index: u64, reason: String) -> GrantError {
        GrantError::LeafNotAnEvent { index, reason }
    }
}

/// The grant log.
pub struct GrantLedger<S: LeafStore> {
    ledger: Ledger<S, GrantLeaves>,
    reopen: Reopen<S>,
    uncertain: Option<Uncertain>,
}

impl<S: LeafStore> GrantLedger<S> {
    /// Open the log over the store `reopen` gives from its snapshot, reading
    /// only the leaves after it, and return every event in log order with its
    /// coordinate. A snapshot is written every `every` entries.
    pub fn open(
        reopen: Reopen<S>,
        key: &Ed25519Identity,
        every: NonZeroU64,
    ) -> Result<(Self, Vec<(SignedGrantEvent, Coordinate)>), GrantError> {
        let (ledger, events) = Ledger::open(&reopen, key, every)?;
        Ok((
            Self {
                ledger,
                reopen,
                uncertain: None,
            },
            events,
        ))
    }

    /// How the log was started: from its snapshot, or from every leaf and
    /// the refusal that sent it there.
    pub fn start(&self) -> &Start {
        self.ledger.start()
    }

    /// Why the last snapshot could not be written, while no later one was.
    pub fn snapshot_failure(&self) -> Option<&str> {
        self.ledger.snapshot_failure()
    }

    /// The append held uncertain, if one is.
    pub fn uncertain(&self) -> Option<&Uncertain> {
        self.uncertain.as_ref()
    }

    fn certain(&self) -> Result<&Ledger<S, GrantLeaves>, GrantError> {
        match &self.uncertain {
            Some(held) => Err(GrantError::OperationUnresolved {
                operation: held.operation.to_string(),
                grant: held.grant.to_string(),
            }),
            None => Ok(&self.ledger),
        }
    }

    /// The log's current size and root, refused while an append is uncertain.
    pub fn head(&self) -> Result<(u64, [u8; 32]), GrantError> {
        Ok(self.certain()?.head())
    }

    /// An inclusion proof of the leaf at `index` in the log's current tree.
    pub fn inclusion_proof(&self, index: u64) -> Result<InclusionProof, GrantError> {
        self.certain()?.inclusion_proof(index)
    }

    /// Append `event` as one leaf, writing a snapshot with `key` when the log
    /// crosses a multiple of its cadence. A failed append is held uncertain
    /// and answered `LogUnavailable`; [`GrantLedger::reconcile`] resolves it.
    pub fn append(
        &mut self,
        event: &SignedGrantEvent,
        key: &Ed25519Identity,
    ) -> Result<Coordinate, GrantError> {
        let index = self.certain()?.len();
        self.ledger.append(event.bytes(), key).map_err(|failure| {
            self.uncertain = Some(Uncertain {
                index,
                operation: event.event().operation(),
                grant: event.event().grant(),
                event: event.event().clone(),
            });
            GrantError::LogUnavailable {
                reason: failure.to_string(),
            }
        })
    }

    /// Resolve a held uncertain append from a fresh open of the store, reading
    /// only the leaves from its index on, and answer every one of them.
    /// Answers `None` when nothing was held. The hold stays until every step
    /// has succeeded.
    pub fn reconcile(
        &mut self,
        key: &Ed25519Identity,
    ) -> Result<Option<Vec<(SignedGrantEvent, Coordinate)>>, GrantError> {
        if self.uncertain.is_none() {
            return Ok(None);
        }
        let adopted = self.ledger.adopt(&self.reopen, key)?;
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
        if let Some(adopted) = self.ledger.reconcile(&self.key)? {
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
        match self.ledger.append(&signed, &self.key) {
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
        match self.ledger.append(&signed, &self.key) {
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
