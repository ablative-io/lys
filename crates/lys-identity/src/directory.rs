//! The directory's typed API: every change is one signed event, committed to
//! the log before it is answered (P4, P5).
//!
//! A change is judged against the projection, signed by the service key,
//! appended as one leaf and only then applied and answered with its receipt.
//! An append whose outcome is uncertain holds the directory: every change and
//! every read of current state is refused `AppendUncertain` until the leaf is
//! read back and the change is known to be recorded or not. Every leaf the
//! read-back finds from that index on is applied, whoever wrote it, before
//! anything is answered. A leaf the projection refuses there breaks the
//! directory: every later call is refused by name, as a restart would be.
//!
//! The operation id rule: the same operation id with the same request answers
//! the first receipt again and records nothing; the same id with a different
//! request is refused `OperationReused`.

use std::num::NonZeroU64;

use lys_core::Ed25519Identity;
use lys_log_store::LeafStore;

use crate::directory_state;
use crate::error::IdentityError;
use crate::event::{Change, IdentityEvent};
use crate::id::{AgentId, IdentityId, PersonId};
use crate::lifecycle::Transition;
use crate::log::{Coordinate, EventLog, Reopen};
use crate::operation::OperationId;
use crate::profile::Profile;
use crate::projection::{Projection, Record};
use crate::provenance::Actor;
use crate::receipt::Receipt;
use crate::restart::SNAPSHOT_EVERY;
use crate::signer::{SignedEvent, sign_event};

#[path = "directory_reporting.rs"]
pub mod reporting;

#[path = "directory_batch.rs"]
mod batch;

#[path = "directory_install.rs"]
mod install;

#[path = "directory_draft.rs"]
mod draft;

#[path = "directory_product_draft.rs"]
mod product_draft;

#[path = "directory_changes.rs"]
mod changes;

pub use crate::organisation::Founded;

/// Why the directory stopped answering, if it has.
type Broken = Option<String>;

/// Whether `actor` is who `recorded` was: the same login binding, the same
/// authentication method, the same app acted through and the same draft
/// provenance. The instant the actor was authenticated at is not who they
/// are: a retry of the same act seconds later is the same act
/// (DIRECTORY-117 R1), and the recorded leaf keeps the first act's instant.
fn same_actor(recorded: &Actor, actor: &Actor) -> bool {
    recorded.binding() == actor.binding()
        && recorded.provenance().method() == actor.provenance().method()
        && recorded.through_app() == actor.through_app()
        && recorded.draft() == actor.draft()
}

/// The directory of people and agents over its log.
pub struct Directory<S: LeafStore> {
    log: EventLog<S>,
    pub(crate) projection: Projection,
    key: Ed25519Identity,
    folded: u64,
    broken: Broken,
}

impl<S: LeafStore> Directory<S> {
    /// Open the directory over the store `reopen` gives from its signed
    /// snapshot, reading and folding only the leaves after it. A snapshot is
    /// written every [`SNAPSHOT_EVERY`] entries.
    ///
    /// # Errors
    /// - `IdentityError::LogUnavailable` if opening, reading or folding the log fails.
    /// - `IdentityError::LeafNotAnEvent` if a stored leaf is missing or cannot be verified.
    /// - `IdentityError::DirectorySnapshotUnmigrated` if an old snapshot requires explicit migration.
    /// - `IdentityError::OperationReused` if a replayed event repeats an operation.
    /// - `IdentityError::AlreadyRegistered` if replay registers an identity twice.
    /// - `IdentityError::AlreadyBootstrapped` if replay repeats setup for a bound login.
    /// - `IdentityError::IdentityUnknown` if a replayed change names an absent identity.
    /// - `IdentityError::BindingTaken` if replay binds one login to different people.
    /// - `IdentityError::BindingMalformed` if an issuer move produces an invalid login binding.
    /// - `IdentityError::StateMismatch` if a replayed transition starts from another state.
    /// - `IdentityError::LinkSourceSeen` if replay repeats a link source operation.
    /// - `IdentityError::ChangeMismatch` if replayed identity or reporting fields disagree.
    /// - `IdentityError::AnswersToUnknown` if a reporting target is absent or of another kind.
    /// - `IdentityError::AnswersToInactive` if the immediate reporting target is inactive.
    /// - `IdentityError::AnswersToCycle` if reporting returns to an earlier identity.
    /// - `IdentityError::NoAccountablePerson` if the reporting chain cannot reach an active person.
    /// - `IdentityError::DraftChangeInvalid` if a replayed draft payload or decision is invalid.
    /// - `IdentityError::DraftHashMismatch` if a decision or correction names another creation.
    /// - `IdentityError::DraftNotFound` if a replayed decision names an absent draft.
    /// - `IdentityError::DraftNotPending` if a replayed decision is outside the draft's current state.
    pub fn open(reopen: Reopen<S>, key: Ed25519Identity) -> Result<Self, IdentityError> {
        Self::open_with(reopen, key, SNAPSHOT_EVERY)
    }

    /// As [`Directory::open`], writing a snapshot every `every` entries.
    ///
    /// # Errors
    /// - `IdentityError::LogUnavailable` if opening, reading or folding the log fails.
    /// - `IdentityError::LeafNotAnEvent` if a stored leaf is missing or cannot be verified.
    /// - `IdentityError::DirectorySnapshotUnmigrated` if an old snapshot requires explicit migration.
    /// - `IdentityError::OperationReused` if a replayed event repeats an operation.
    /// - `IdentityError::AlreadyRegistered` if replay registers an identity twice.
    /// - `IdentityError::AlreadyBootstrapped` if replay repeats setup for a bound login.
    /// - `IdentityError::IdentityUnknown` if a replayed change names an absent identity.
    /// - `IdentityError::BindingTaken` if replay binds one login to different people.
    /// - `IdentityError::BindingMalformed` if an issuer move produces an invalid login binding.
    /// - `IdentityError::StateMismatch` if a replayed transition starts from another state.
    /// - `IdentityError::LinkSourceSeen` if replay repeats a link source operation.
    /// - `IdentityError::ChangeMismatch` if replayed identity or reporting fields disagree.
    /// - `IdentityError::AnswersToUnknown` if a reporting target is absent or of another kind.
    /// - `IdentityError::AnswersToInactive` if the immediate reporting target is inactive.
    /// - `IdentityError::AnswersToCycle` if reporting returns to an earlier identity.
    /// - `IdentityError::NoAccountablePerson` if the reporting chain cannot reach an active person.
    /// - `IdentityError::DraftChangeInvalid` if a replayed draft payload or decision is invalid.
    /// - `IdentityError::DraftHashMismatch` if a decision or correction names another creation.
    /// - `IdentityError::DraftNotFound` if a replayed decision names an absent draft.
    /// - `IdentityError::DraftNotPending` if a replayed decision is outside the draft's current state.
    pub fn open_with(
        reopen: Reopen<S>,
        key: Ed25519Identity,
        every: NonZeroU64,
    ) -> Result<Self, IdentityError> {
        let (mut log, opening) = EventLog::open(reopen, &key, every)?;
        if opening
            .state
            .as_deref()
            .is_some_and(|state| directory_state::version(state) == Ok(2))
        {
            return Err(IdentityError::DirectorySnapshotUnmigrated {
                found: 2,
                expected: directory_state::STATE_VERSION,
            });
        }
        let read = opening
            .state
            .as_deref()
            .map(|state| directory_state::decode(state, opening.size));
        let (projection, folded, events) = match read {
            None => (Projection::new(), 0, opening.events),
            Some(Ok(projection)) => (projection, opening.size, opening.events),
            Some(Err(reason)) => (Projection::new(), 0, log.refuse_state(reason, &key)?),
        };
        let mut directory = Self {
            log,
            projection,
            key,
            folded,
            broken: None,
        };
        for (signed, coordinate) in events {
            directory.record_committed(&signed, coordinate)?;
        }
        directory.snapshot();
        Ok(directory)
    }

    /// Write a snapshot of the folded state when one is owed and every leaf
    /// is folded. A broken directory's state is not the fold of its log, so
    /// none is written.
    fn snapshot(&mut self) {
        if self.broken.is_some() {
            return;
        }
        let (projection, folded) = (&self.projection, self.folded);
        self.log
            .snapshot_if_due(&self.key, || directory_state::encode(projection, folded));
    }

    /// The service's public key, against which every event and receipt verifies.
    pub fn service_key(&self) -> [u8; 32] {
        self.key.public_key_bytes()
    }

    /// The log, for receipts and inclusion proofs, once any uncertain append
    /// is resolved.
    ///
    /// # Errors
    /// - `IdentityError::LogUnavailable` if the directory is broken, the store cannot be reopened, or an adopted event is refused by the projection.
    /// - `IdentityError::LeafNotAnEvent` if reconciliation finds a missing or invalid signed leaf.
    pub fn log(&mut self) -> Result<&EventLog<S>, IdentityError> {
        self.settle()?;
        Ok(&self.log)
    }

    /// Resolve an uncertain append, and apply every leaf it adopted, before
    /// anything is answered as current.
    ///
    /// # Errors
    /// - `IdentityError::LogUnavailable` if the directory is broken, the store cannot be reopened, or an adopted event is refused by the projection.
    /// - `IdentityError::LeafNotAnEvent` if reconciliation finds a missing or invalid signed leaf.
    pub fn settle(&mut self) -> Result<(), IdentityError> {
        if let Some(reason) = &self.broken {
            return Err(IdentityError::LogUnavailable {
                reason: reason.clone(),
            });
        }
        let Some(reconciled) = self.log.reconcile(&self.key)? else {
            return Ok(());
        };
        for (signed, coordinate) in reconciled.adopted {
            if let Err(refusal) = self.record_committed(&signed, coordinate) {
                let reason = format!(
                    "leaf {} in the log is refused by the projection: {refusal}",
                    coordinate.index
                );
                self.broken = Some(reason.clone());
                return Err(IdentityError::LogUnavailable { reason });
            }
        }
        self.snapshot();
        Ok(())
    }

    /// The identity `id` as it stands, once any uncertain append is resolved.
    ///
    /// # Errors
    /// - `IdentityError::LogUnavailable` if the directory is broken, the store cannot be reopened, or an adopted event is refused by the projection.
    /// - `IdentityError::LeafNotAnEvent` if reconciliation finds a missing or invalid signed leaf.
    pub fn record(&mut self, id: IdentityId) -> Result<Option<Record>, IdentityError> {
        self.settle()?;
        Ok(self.projection.record(id).cloned())
    }

    /// The whole projection as it stands, once any uncertain append is resolved.
    ///
    /// # Errors
    /// - `IdentityError::LogUnavailable` if the directory is broken, the store cannot be reopened, or an adopted event is refused by the projection.
    /// - `IdentityError::LeafNotAnEvent` if reconciliation finds a missing or invalid signed leaf.
    pub fn projection(&mut self) -> Result<&Projection, IdentityError> {
        self.settle()?;
        Ok(&self.projection)
    }

    fn record_committed(
        &mut self,
        signed: &SignedEvent,
        coordinate: Coordinate,
    ) -> Result<(), IdentityError> {
        let (index, expected) = (coordinate.index, self.folded);
        if index != expected {
            return Err(IdentityError::LogUnavailable {
                reason: format!("leaf {index} arrived where leaf {expected} was expected"),
            });
        }
        self.projection.apply_entry(signed.entry(), index)?;
        self.folded += 1;
        Ok(())
    }

    /// The event and receipt an operation recorded, if it recorded one, both
    /// built from the log.
    pub(crate) fn answered(
        &self,
        operation: OperationId,
    ) -> Result<Option<(IdentityEvent, Receipt)>, IdentityError> {
        let Some(index) = self.projection.operation(operation) else {
            return Ok(None);
        };
        let (signed, coordinate) =
            self.log
                .entry(index)?
                .ok_or_else(|| IdentityError::LogUnavailable {
                    reason: format!("operation {operation} names leaf {index}, which is not held"),
                })?;
        Ok(Some((
            signed.event()?.clone(),
            Receipt::of(&signed, coordinate)?,
        )))
    }

    /// The first answer to `operation`, if it was answered for the same request.
    fn retry(
        &self,
        operation: OperationId,
        actor: &Actor,
        identity: Option<IdentityId>,
        change: &Change,
    ) -> Result<Option<(IdentityId, Receipt)>, IdentityError> {
        let Some((event, receipt)) = self.answered(operation)? else {
            return Ok(None);
        };
        let same_identity = identity.is_none_or(|identity| identity == event.identity());
        if same_actor(event.actor(), actor) && event.change() == change && same_identity {
            Ok(Some((event.identity(), receipt)))
        } else {
            Err(IdentityError::OperationReused {
                operation: operation.to_string(),
            })
        }
    }

    /// Judge, sign, append and apply one change.
    fn commit(&mut self, event: IdentityEvent) -> Result<Receipt, IdentityError> {
        self.projection.check(&event)?;
        let signed = sign_event(event, &self.key)?;
        let failure = match self.log.append(&signed) {
            Ok(coordinate) => {
                self.record_committed(&signed, coordinate)?;
                self.snapshot();
                return Receipt::of(&signed, coordinate);
            }
            Err(failure) => failure,
        };
        if !self.log.is_uncertain() {
            return Err(failure);
        }
        // A settle that cannot read the log back keeps the hold, and the
        // caller's retry of the same operation finds the answer once it can.
        self.settle()?;
        match self.answered(signed.event()?.operation())? {
            Some((event, receipt)) if &event == signed.event()? => Ok(receipt),
            _ => Err(IdentityError::AppendRefused {
                reason: failure.to_string(),
            }),
        }
    }

    /// Answer `change` to an identity the caller names, under the retry rule.
    fn change(
        &mut self,
        actor: Actor,
        operation: OperationId,
        identity: IdentityId,
        change: Change,
        recorded_at: u64,
    ) -> Result<Receipt, IdentityError> {
        self.settle()?;
        if let Some((_, receipt)) = self.retry(operation, &actor, Some(identity), &change)? {
            return Ok(receipt);
        }
        self.commit(IdentityEvent::new(
            operation,
            actor,
            identity,
            recorded_at,
            change,
        )?)
    }

    /// Commit a change the link-audit receiver built, under the retry rule.
    pub(crate) fn commit_change(
        &mut self,
        actor: Actor,
        operation: OperationId,
        identity: IdentityId,
        change: Change,
        recorded_at: u64,
    ) -> Result<Receipt, IdentityError> {
        self.change(actor, operation, identity, change, recorded_at)
    }

    /// The receipt of the event at log index `index`, built from the log:
    /// its event and root are read from the nearest checkpoint on. `None` for
    /// a leaf the directory has not folded.
    ///
    /// # Errors
    /// - `IdentityError::LogUnavailable` if a checkpoint or stored leaf cannot be read.
    /// - `IdentityError::LeafNotAnEvent` if the stored event cannot be verified.
    /// - `IdentityError::InstallEntry` if the requested leaf is an install event.
    /// - `IdentityError::DraftEntry` if the requested leaf is a draft.
    pub fn receipt_at(&self, index: u64) -> Result<Option<Receipt>, IdentityError> {
        if index >= self.folded {
            return Ok(None);
        }
        self.log
            .entry(index)?
            .map(|(signed, coordinate)| Receipt::of(&signed, coordinate))
            .transpose()
    }

    /// The committed event at `index` with its log position, or `None` past
    /// what the directory has folded.
    pub(crate) fn committed_at(
        &self,
        index: u64,
    ) -> Result<Option<(SignedEvent, Coordinate)>, IdentityError> {
        if index >= self.folded {
            return Ok(None);
        }
        self.log.entry(index)
    }
}

#[cfg(test)]
mod same_actor_tests {
    use super::same_actor;
    use crate::{Actor, AuthMethod, LoginBinding, Provenance};

    fn actor(subject: &str, method: AuthMethod, at: u64) -> Actor {
        Actor::new(
            LoginBinding::new("https://issuer.example.test", subject).unwrap(),
            Provenance::new(method, at),
        )
    }

    #[test]
    fn a_retry_at_another_second_is_the_same_actor_and_another_who_is_not() {
        let first = actor("ada", AuthMethod::Oidc, 4);
        assert!(same_actor(&first, &actor("ada", AuthMethod::Oidc, 90)));
        assert!(!same_actor(&first, &actor("bea", AuthMethod::Oidc, 4)));
        assert!(!same_actor(&first, &actor("ada", AuthMethod::Operator, 4)));
        assert!(!same_actor(
            &first,
            &actor("ada", AuthMethod::Oidc, 4).with_through_app("fixture_product")
        ));
        let through = actor("ada", AuthMethod::Oidc, 4).with_through_app("fixture_product");
        assert!(same_actor(
            &through,
            &actor("ada", AuthMethod::Oidc, 61).with_through_app("fixture_product")
        ));
    }
}
