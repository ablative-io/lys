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

use std::num::NonZeroU64;
use std::sync::Arc;

use lys_core::Ed25519Identity;
use lys_core::merkle::InclusionProof;
use lys_log_store::witness::{TailWitness, TailWitnessProvider};
use lys_log_store::{LeafStore, Start};

use super::error::GrantError;
use super::events::{GrantEvent, SignedGrantEvent, verify_grant_event};
use super::types::GrantId;
use crate::log::{Coordinate, Reopen};
use crate::operation::OperationId;
use crate::restart::{Leaves, Ledger, Opening};

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
    service_key: [u8; 32],
    uncertain: Option<Uncertain>,
    tail_provider: Option<Arc<dyn TailWitnessProvider + Send + Sync>>,
}

impl<S: LeafStore> GrantLedger<S> {
    /// Open the log over the store `reopen` gives from its owner's snapshot,
    /// reading only the leaves after it, and hand back the snapshot's state
    /// with every event after it. A snapshot is owed every `every` entries.
    pub fn open(
        reopen: Reopen<S>,
        key: &Ed25519Identity,
        every: NonZeroU64,
    ) -> Result<(Self, Opening<SignedGrantEvent>), GrantError> {
        Self::open_with_tail_provider(reopen, key, every, None)
    }

    /// Open with an explicitly supplied current-head tail capability.
    ///
    /// # Errors
    /// Retains the original opening failure; no capability is inferred from storage.
    pub fn open_with_tail_provider(
        reopen: Reopen<S>,
        key: &Ed25519Identity,
        every: NonZeroU64,
        tail_provider: Option<Arc<dyn TailWitnessProvider + Send + Sync>>,
    ) -> Result<(Self, Opening<SignedGrantEvent>), GrantError> {
        let (ledger, opening) = Ledger::open(&reopen, key, every)?;
        Ok((
            Self {
                ledger,
                reopen,
                service_key: key.public_key_bytes(),
                uncertain: None,
                tail_provider,
            },
            opening,
        ))
    }

    /// Acquire the complete tail after this owner's actual trusted frontier.
    /// An uncertain append stays held; acquiring evidence never reconciles it.
    ///
    /// # Errors
    /// Names an absent capability and preserves the provider's original refusal.
    pub fn acquire_tail(&self) -> Result<TailWitness, GrantError> {
        let provider = self
            .tail_provider
            .as_ref()
            .ok_or_else(|| super::tail_witness::unavailable("tail witness capability is absent"))?;
        provider
            .acquire(self.ledger.trusted_frontier())
            .map_err(|error| GrantError::LogUnavailable {
                reason: error.to_string(),
            })
    }

    /// Authenticate `witness` from owned in-memory inputs alone: this owner's
    /// trusted frontier, log origin and service key, every index, every
    /// event's signature and the certified root. It does not certify that the
    /// upper bound is still the current head; [`GrantLedger::with_verified_tail`]
    /// does that.
    ///
    /// # Errors
    /// Names the bound, index, signature or root that does not hold.
    pub fn authenticate_tail(
        &self,
        witness: &TailWitness,
    ) -> Result<Vec<SignedGrantEvent>, GrantError> {
        super::tail_witness::authenticate(
            witness,
            self.ledger.trusted_frontier(),
            self.ledger.origin(),
            &self.service_key,
        )
    }

    /// The size of the frontier this owner has verified and handed on.
    pub(super) fn trusted_size(&self) -> u64 {
        self.ledger.trusted_frontier().size()
    }

    /// Authenticate every tail event and read it within a fresh provider callback.
    /// This supplies the signed events; their authority effects are named by
    /// [`Grants::tail_authority`](super::Grants::tail_authority).
    /// The callback must not append through the same provider's head lock.
    ///
    /// # Errors
    /// Preserves provider, signature and reading failures; stale or incomplete
    /// evidence never reaches the reading. No permission failure is waived.
    pub fn with_verified_tail<T>(
        &self,
        witness: &TailWitness,
        reading: impl FnOnce(&[SignedGrantEvent]) -> Result<T, GrantError>,
    ) -> Result<T, GrantError> {
        let provider = self
            .tail_provider
            .as_ref()
            .ok_or_else(|| super::tail_witness::unavailable("tail witness capability is absent"))?;
        let settled = self.ledger.trusted_frontier();
        let mut reading = Some(reading);
        let mut result = None;
        provider
            .verify(witness, settled, &mut |certified| {
                result = Some(match reading.take() {
                    Some(reading) => {
                        if certified == witness {
                            super::tail_witness::authenticate(
                                certified,
                                settled,
                                self.ledger.origin(),
                                &self.service_key,
                            )
                            .and_then(|events| reading(&events))
                        } else {
                            Err(super::tail_witness::unavailable(
                                "tail provider substituted its certified reading",
                            ))
                        }
                    }
                    None => Err(super::tail_witness::unavailable(
                        "tail provider repeated its reading callback",
                    )),
                });
                Ok(())
            })
            .map_err(|error| GrantError::LogUnavailable {
                reason: error.to_string(),
            })?;
        match result {
            Some(result) => result,
            None => Err(super::tail_witness::unavailable(
                "tail provider omitted its reading callback",
            )),
        }
    }

    /// Refuse the snapshot's state, which the owner could not read, by
    /// `reason`, and reopen the log from every leaf, answering every event.
    pub fn refuse_state(
        &mut self,
        reason: String,
        key: &Ed25519Identity,
    ) -> Result<Vec<(SignedGrantEvent, Coordinate)>, GrantError> {
        self.ledger.refuse_state(&self.reopen, reason, key)
    }

    /// Write a snapshot of the state `encode` gives, when one is owed. The
    /// caller calls this only when that state is the fold of every leaf.
    pub fn snapshot_if_due(
        &mut self,
        key: &Ed25519Identity,
        encode: impl FnOnce() -> Result<Vec<u8>, String>,
    ) {
        if self.uncertain.is_none() {
            self.ledger.snapshot_if_due(key, encode);
        }
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

    /// The event at `index` with the coordinate it completed, read from the
    /// store from the nearest checkpoint and verified, refused while an
    /// append is uncertain.
    pub fn entry(&self, index: u64) -> Result<Option<(SignedGrantEvent, Coordinate)>, GrantError> {
        self.certain()?.entry(index, &self.service_key)
    }

    /// Every event in log order with its coordinate, read from the store in
    /// one pass and verified, refused while an append is uncertain.
    pub fn entries(&self) -> Result<Vec<(SignedGrantEvent, Coordinate)>, GrantError> {
        self.certain()?.entries(&self.service_key)
    }

    /// The events from `from` up to, not including, `through`, with their
    /// coordinates, read in one pass from the checkpoint at or below `from`
    /// and verified: a replay's cost is bounded by the checkpoint distance
    /// and the range, never the history (DIRECTORY-089 R1). Refused while
    /// an append is uncertain.
    pub fn entries_between(
        &self,
        from: u64,
        through: u64,
    ) -> Result<Vec<(SignedGrantEvent, Coordinate)>, GrantError> {
        self.certain()?
            .entries_between(from, through, &self.service_key)
    }

    /// An inclusion proof of the leaf at `index` in the log's current tree.
    pub fn inclusion_proof(&self, index: u64) -> Result<InclusionProof, GrantError> {
        self.certain()?.inclusion_proof(index)
    }

    /// Append `event` as one leaf. A failed append is held uncertain and
    /// answered `LogUnavailable`; [`GrantLedger::reconcile`] resolves it.
    pub fn append(&mut self, event: &SignedGrantEvent) -> Result<Coordinate, GrantError> {
        let index = self.certain()?.len();
        self.ledger.append(event.bytes()).map_err(|failure| {
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
