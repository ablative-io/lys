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

/// Why the directory stopped answering, if it has.
type Broken = Option<String>;

/// The directory of people and agents over its log.
pub struct Directory<S: LeafStore> {
    log: EventLog<S>,
    projection: Projection,
    key: Ed25519Identity,
    folded: u64,
    broken: Broken,
}

impl<S: LeafStore> Directory<S> {
    /// Open the directory over the store `reopen` gives from its signed
    /// snapshot, reading and folding only the leaves after it. A snapshot is
    /// written every [`SNAPSHOT_EVERY`] entries.
    pub fn open(reopen: Reopen<S>, key: Ed25519Identity) -> Result<Self, IdentityError> {
        Self::open_with(reopen, key, SNAPSHOT_EVERY)
    }

    /// As [`Directory::open`], writing a snapshot every `every` entries.
    pub fn open_with(
        reopen: Reopen<S>,
        key: Ed25519Identity,
        every: NonZeroU64,
    ) -> Result<Self, IdentityError> {
        let (mut log, opening) = EventLog::open(reopen, &key, every)?;
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
    pub fn log(&mut self) -> Result<&EventLog<S>, IdentityError> {
        self.settle()?;
        Ok(&self.log)
    }

    /// Resolve an uncertain append, and apply every leaf it adopted, before
    /// anything is answered as current.
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
    pub fn record(&mut self, id: IdentityId) -> Result<Option<Record>, IdentityError> {
        self.settle()?;
        Ok(self.projection.record(id).cloned())
    }

    /// The whole projection as it stands, once any uncertain append is resolved.
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
        self.projection.apply(signed.event(), index)?;
        self.folded += 1;
        Ok(())
    }

    /// The event and receipt an operation recorded, if it recorded one, both
    /// built from the log.
    fn answered(
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
            signed.event().clone(),
            Receipt::of(&signed, coordinate),
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
        if event.actor() == actor && event.change() == change && same_identity {
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
                return Ok(Receipt::of(&signed, coordinate));
            }
            Err(failure) => failure,
        };
        if !self.log.is_uncertain() {
            return Err(failure);
        }
        // A settle that cannot read the log back keeps the hold, and the
        // caller's retry of the same operation finds the answer once it can.
        self.settle()?;
        match self.answered(signed.event().operation())? {
            Some((event, receipt)) if &event == signed.event() => Ok(receipt),
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

    /// Create one active person bound to the authenticated actor in one signed leaf.
    ///
    /// The service must admit the configured administrator before calling this.
    /// Retrying the same operation, login and profile returns its original receipt,
    /// including after a new authenticated session or a reopen. It never reactivates
    /// a person whose state was subsequently changed.
    pub fn setup_person(
        &mut self,
        actor: Actor,
        operation: OperationId,
        profile: Profile,
        recorded_at: u64,
    ) -> Result<(PersonId, Receipt), IdentityError> {
        self.settle()?;
        let change = Change::SetupPerson { profile };
        if let Some((event, receipt)) = self.answered(operation)? {
            if event.actor().binding() == actor.binding()
                && event.change() == &change
                && let IdentityId::Person(person) = event.identity()
            {
                return Ok((person, receipt));
            }
            return Err(IdentityError::OperationReused {
                operation: operation.to_string(),
            });
        }
        if let Some(person) = self.projection.person_for(actor.binding()) {
            return Err(IdentityError::AlreadyBootstrapped {
                person: person.to_string(),
            });
        }
        let person = PersonId::generate()?;
        let event = IdentityEvent::new(
            operation,
            actor,
            IdentityId::Person(person),
            recorded_at,
            change,
        )?;
        Ok((person, self.commit(event)?))
    }

    /// Register a person, answered with their new enduring id.
    pub fn register_person(
        &mut self,
        actor: Actor,
        operation: OperationId,
        profile: Profile,
        recorded_at: u64,
    ) -> Result<(PersonId, Receipt), IdentityError> {
        self.settle()?;
        let change = Change::RegisterPerson { profile };
        if let Some((IdentityId::Person(id), receipt)) =
            self.retry(operation, &actor, None, &change)?
        {
            return Ok((id, receipt));
        }
        let id = PersonId::generate()?;
        let event = IdentityEvent::new(
            operation,
            actor,
            IdentityId::Person(id),
            recorded_at,
            change,
        )?;
        Ok((id, self.commit(event)?))
    }

    /// Register an agent under `responsible`, the person who registers it, for life.
    ///
    /// Nothing is started, and no login, credential, handle or certificate is issued.
    pub fn register_agent(
        &mut self,
        actor: Actor,
        operation: OperationId,
        responsible: PersonId,
        profile: Profile,
        recorded_at: u64,
    ) -> Result<(AgentId, Receipt), IdentityError> {
        self.settle()?;
        let change = Change::RegisterAgent {
            responsible,
            profile,
        };
        if let Some((IdentityId::Agent(id), receipt)) =
            self.retry(operation, &actor, None, &change)?
        {
            return Ok((id, receipt));
        }
        let id = AgentId::generate()?;
        let event =
            IdentityEvent::new(operation, actor, IdentityId::Agent(id), recorded_at, change)?;
        Ok((id, self.commit(event)?))
    }

    /// Replace an identity's display profile.
    pub fn change_profile(
        &mut self,
        actor: Actor,
        operation: OperationId,
        identity: IdentityId,
        profile: Profile,
        recorded_at: u64,
    ) -> Result<Receipt, IdentityError> {
        self.change(
            actor,
            operation,
            identity,
            Change::ChangeProfile { profile },
            recorded_at,
        )
    }

    /// Bind a login to a person. One login names at most one person.
    pub fn bind_login(
        &mut self,
        actor: Actor,
        operation: OperationId,
        person: PersonId,
        binding: crate::binding::LoginBinding,
        recorded_at: u64,
    ) -> Result<Receipt, IdentityError> {
        self.change(
            actor,
            operation,
            IdentityId::Person(person),
            Change::BindLogin { binding },
            recorded_at,
        )
    }

    /// Move an identity through one lifecycle transition, from the state it is in.
    pub fn transition(
        &mut self,
        actor: Actor,
        operation: OperationId,
        identity: IdentityId,
        transition: Transition,
        reason: &str,
        recorded_at: u64,
    ) -> Result<Receipt, IdentityError> {
        self.settle()?;
        let from = match self.answered(operation)? {
            Some((event, _)) => match event.change() {
                Change::Transition { from, .. } => *from,
                _ => {
                    return Err(IdentityError::OperationReused {
                        operation: operation.to_string(),
                    });
                }
            },
            None => self
                .projection
                .record(identity)
                .ok_or_else(|| IdentityError::IdentityUnknown {
                    identity: identity.to_string(),
                })?
                .state(),
        };
        let to = transition.target(from)?;
        self.change(
            actor,
            operation,
            identity,
            Change::Transition {
                transition,
                from,
                to,
                reason: reason.to_owned(),
            },
            recorded_at,
        )
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
    pub fn receipt_at(&self, index: u64) -> Result<Option<Receipt>, IdentityError> {
        if index >= self.folded {
            return Ok(None);
        }
        Ok(self
            .log
            .entry(index)?
            .map(|(signed, coordinate)| Receipt::of(&signed, coordinate)))
    }
}
