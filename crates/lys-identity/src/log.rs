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

use std::num::NonZeroU64;

use lys_core::Ed25519Identity;
use lys_core::merkle::InclusionProof;
use lys_log_store::{LeafStore, Start, StoreError};

use crate::error::IdentityError;
use crate::restart::{Leaves, Ledger, Opening};
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

/// The directory's leaves: identity events signed by its service key.
pub(crate) struct IdentityLeaves;

impl Leaves for IdentityLeaves {
    type Event = SignedEvent;
    type Error = IdentityError;
    const DOMAIN: &'static str = "lys/identity-directory/v1";

    fn verify(bytes: &[u8], key: &[u8; 32]) -> Result<SignedEvent, IdentityError> {
        verify_event(bytes, key)
    }

    fn unavailable(reason: String) -> IdentityError {
        IdentityError::LogUnavailable { reason }
    }

    fn not_an_event(index: u64, reason: String) -> IdentityError {
        IdentityError::LeafNotAnEvent { index, reason }
    }
}

/// The directory's log of signed events.
pub struct EventLog<S: LeafStore> {
    ledger: Ledger<S, IdentityLeaves>,
    reopen: Reopen<S>,
    service_key: [u8; 32],
    pending: Option<Pending>,
}

fn unavailable(error: &StoreError) -> IdentityError {
    IdentityError::LogUnavailable {
        reason: error.to_string(),
    }
}

impl<S: LeafStore> EventLog<S> {
    /// Open the log over the store `reopen` gives from its owner's snapshot,
    /// reading only the leaves after it, and hand back the snapshot's state
    /// with every event after it. A snapshot is owed every `every` entries.
    pub fn open(
        reopen: Reopen<S>,
        key: &Ed25519Identity,
        every: NonZeroU64,
    ) -> Result<(Self, Opening<SignedEvent>), IdentityError> {
        let (ledger, opening) = Ledger::open(&reopen, key, every)?;
        Ok((
            Self {
                ledger,
                reopen,
                service_key: key.public_key_bytes(),
                pending: None,
            },
            opening,
        ))
    }

    /// Refuse the snapshot's state, which the owner could not read, by
    /// `reason`, and reopen the log from every leaf, answering every event.
    pub fn refuse_state(
        &mut self,
        reason: String,
        key: &Ed25519Identity,
    ) -> Result<Vec<(SignedEvent, Coordinate)>, IdentityError> {
        self.ledger.refuse_state(&self.reopen, reason, key)
    }

    /// Write a snapshot of the state `encode` gives, when one is owed. The
    /// caller calls this only when that state is the fold of every leaf.
    pub fn snapshot_if_due(
        &mut self,
        key: &Ed25519Identity,
        encode: impl FnOnce() -> Result<Vec<u8>, String>,
    ) {
        if self.pending.is_none() {
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

    /// Whether an append is held uncertain and every current read must wait for it.
    pub fn is_uncertain(&self) -> bool {
        self.pending.is_some()
    }

    fn certain(&self) -> Result<&Ledger<S, IdentityLeaves>, IdentityError> {
        match &self.pending {
            Some(pending) => Err(IdentityError::AppendUncertain {
                index: pending.index,
            }),
            None => Ok(&self.ledger),
        }
    }

    /// The number of leaves in the log, refused while an append is uncertain.
    pub fn len(&self) -> Result<u64, IdentityError> {
        Ok(self.certain()?.len())
    }

    /// Whether the log holds no leaf, refused while an append is uncertain.
    pub fn is_empty(&self) -> Result<bool, IdentityError> {
        Ok(self.certain()?.len() == 0)
    }

    /// The log's current size and root, refused while an append is uncertain.
    pub fn head(&self) -> Result<(u64, [u8; 32]), IdentityError> {
        Ok(self.certain()?.head())
    }

    /// The leaf bytes at `index`, if the log holds one there, read from the
    /// store, refused while an append is uncertain.
    pub fn leaf(&self, index: u64) -> Result<Option<Vec<u8>>, IdentityError> {
        self.certain()?.leaf(index)
    }

    /// The event at `index`, if the log holds one there, read from the store
    /// and verified, refused while an append is uncertain.
    pub fn event(&self, index: u64) -> Result<Option<SignedEvent>, IdentityError> {
        self.certain()?.event(index, &self.service_key)
    }

    /// An inclusion proof of the leaf at `index` in the log's current tree,
    /// refused while an append is uncertain.
    pub fn inclusion_proof(&self, index: u64) -> Result<InclusionProof, IdentityError> {
        self.certain()?.inclusion_proof(index)
    }

    /// Append `event` as one leaf.
    ///
    /// A failed append may still have stored its leaf, so it is held
    /// uncertain and answered `LogUnavailable` with the store's reason; the
    /// caller resolves it with [`EventLog::reconcile`].
    pub fn append(&mut self, event: &SignedEvent) -> Result<Coordinate, IdentityError> {
        let index = self.certain()?.len();
        self.ledger.append(event.bytes()).map_err(|failure| {
            self.pending = Some(Pending {
                index,
                bytes: event.bytes().to_vec(),
            });
            unavailable(&failure)
        })
    }

    /// Resolve a held uncertain append from a fresh open of the store, reading
    /// only the leaves from its index on, and adopt every one of them. Answers
    /// `None` when nothing was held. The hold stays until every step has
    /// succeeded.
    pub fn reconcile(
        &mut self,
        key: &Ed25519Identity,
    ) -> Result<Option<Reconciled>, IdentityError> {
        let Some(pending) = &self.pending else {
            return Ok(None);
        };
        let adopted = self.ledger.adopt(&self.reopen, key)?;
        let resolved = match adopted.first() {
            Some((signed, coordinate)) if signed.bytes() == pending.bytes.as_slice() => {
                Resolved::Committed(*coordinate)
            }
            _ => Resolved::NotCommitted,
        };
        self.pending = None;
        Ok(Some(Reconciled { resolved, adopted }))
    }

    /// The service key every leaf is verified against.
    pub fn service_key(&self) -> &[u8; 32] {
        &self.service_key
    }
}
