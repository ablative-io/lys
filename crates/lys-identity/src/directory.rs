//! The directory's typed API: every change is one signed event, committed to
//! the log before it is answered (P4, P5).
//!
//! A change is judged against the projection, signed by the service key,
//! appended as one leaf and only then applied and answered with its receipt.
//! An append whose outcome is uncertain holds the directory: every change and
//! every read of current state is refused `AppendUncertain` until the leaf is
//! read back and the change is known to be recorded or not.
//!
//! The operation id rule: the same operation id with the same request answers
//! the first receipt again and records nothing; the same id with a different
//! request is refused `OperationReused`.

use std::collections::HashMap;

use lys_core::Ed25519Identity;
use lys_log_store::LeafStore;

use crate::error::IdentityError;
use crate::event::{Change, IdentityEvent};
use crate::id::{AgentId, IdentityId, PersonId};
use crate::lifecycle::Transition;
use crate::log::{EventLog, Reopen, Resolved};
use crate::operation::OperationId;
use crate::profile::Profile;
use crate::projection::{Projection, Record};
use crate::provenance::Actor;
use crate::receipt::Receipt;
use crate::signer::{SignedEvent, sign_event};

/// The directory of people and agents over its log.
pub struct Directory<S: LeafStore> {
    log: EventLog<S>,
    projection: Projection,
    key: Ed25519Identity,
    answered: HashMap<OperationId, (IdentityEvent, Receipt)>,
    pending: Option<SignedEvent>,
}

impl<S: LeafStore> Directory<S> {
    /// Open the directory over the store `reopen` gives, replaying every event
    /// into the projection.
    pub fn open(reopen: Reopen<S>, key: Ed25519Identity) -> Result<Self, IdentityError> {
        let (log, events) = EventLog::open(reopen, key.public_key_bytes())?;
        let mut directory = Self {
            log,
            projection: Projection::new(),
            key,
            answered: HashMap::new(),
            pending: None,
        };
        for (signed, coordinate) in events {
            directory.record_committed(&signed, Receipt::of(&signed, coordinate))?;
        }
        Ok(directory)
    }

    /// The service's public key, against which every event and receipt verifies.
    pub fn service_key(&self) -> [u8; 32] {
        self.key.public_key_bytes()
    }

    /// The log, for receipts and inclusion proofs.
    pub fn log(&self) -> &EventLog<S> {
        &self.log
    }

    /// Resolve an uncertain append before anything is answered as current.
    pub fn settle(&mut self) -> Result<(), IdentityError> {
        if !self.log.is_uncertain() {
            return Ok(());
        }
        let resolved = self.log.reconcile()?;
        let pending = self.pending.take();
        if let (Some(Resolved::Committed(coordinate)), Some(signed)) = (resolved, pending) {
            let receipt = Receipt::of(&signed, coordinate);
            self.record_committed(&signed, receipt)?;
        }
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
        receipt: Receipt,
    ) -> Result<(), IdentityError> {
        let event = signed.event();
        self.projection.apply(event, receipt.coordinate().index)?;
        self.answered
            .insert(event.operation(), (event.clone(), receipt));
        Ok(())
    }

    /// The first answer to `operation`, if it was answered for the same request.
    fn retry(
        &self,
        operation: OperationId,
        actor: &Actor,
        identity: Option<IdentityId>,
        change: &Change,
    ) -> Result<Option<(IdentityId, Receipt)>, IdentityError> {
        let Some((event, receipt)) = self.answered.get(&operation) else {
            return Ok(None);
        };
        let same_identity = identity.is_none_or(|identity| identity == event.identity());
        if event.actor() == actor && event.change() == change && same_identity {
            Ok(Some((event.identity(), receipt.clone())))
        } else {
            Err(IdentityError::OperationReused {
                operation: operation.to_string(),
            })
        }
    }

    /// Judge, sign, append and apply one change.
    fn commit(&mut self, event: IdentityEvent) -> Result<Receipt, IdentityError> {
        self.projection.check(&event)?;
        let signed = sign_event(event, &self.key);
        match self.log.append(&signed) {
            Ok(coordinate) => {
                let receipt = Receipt::of(&signed, coordinate);
                self.record_committed(&signed, receipt.clone())?;
                Ok(receipt)
            }
            Err(error) => {
                if self.log.is_uncertain() {
                    self.pending = Some(signed);
                }
                Err(error)
            }
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
        let from = match self.answered.get(&operation) {
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

    /// The receipt of the event at log index `index`.
    pub fn receipt_at(&self, index: u64) -> Option<&Receipt> {
        self.answered
            .values()
            .map(|(_, receipt)| receipt)
            .find(|receipt| receipt.coordinate().index == index)
    }
}
