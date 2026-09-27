//! The grants' one authority owner: every issue, pass-on, revocation and
//! exercise decision goes through [`Grants`], whichever route it came by.
//!
//! A request is judged by admission, signed and committed to the grant log as
//! one grant event, applied to the grant book, projected into the permission
//! relationships, and only then answered. A refused request records nothing.
//! A retry of an answered operation with the same request is answered again
//! and records nothing; the same operation with a different request is refused
//! `OperationReused`. A decision is made only from relationships at least as
//! fresh as every change on the authority it rests on.

use std::collections::BTreeSet;

use lys_core::Ed25519Identity;
use lys_log_store::LeafStore;

use super::admission::{
    DelegateRequest, RootRequest, Route, effective, judge_delegation, judge_root,
};
use super::error::GrantError;
use super::events::SignedGrantEvent;
use super::events::{GrantChange, GrantEvent};
use super::model::Model;
use super::permission::{RelationshipStore, confirm};
use super::projection::GrantBook;
use super::receipt::GrantReceipt;
use super::recovery::GrantLedger;
use super::revocation::judge_revoke;
use super::types::{Action, Grant, GrantId, Resource, Source};
use crate::id::{IdentityId, PersonId};
use crate::log::Reopen;
use crate::operation::OperationId;
use crate::projection::Projection;

/// A request to exercise an action on a resource.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExerciseRequest {
    /// The authenticated caller.
    pub caller: IdentityId,
    /// How the request arrived.
    pub route: Route,
    /// The object acted on.
    pub resource: Resource,
    /// The action.
    pub action: Action,
}

/// A request to revoke a grant, and with it everything derived from it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevokeRequest {
    /// The caller's operation id, kept across retries.
    pub operation: OperationId,
    /// The authenticated caller.
    pub caller: IdentityId,
    /// How the request arrived.
    pub route: Route,
    /// The grant revoked.
    pub grant: GrantId,
    /// Why.
    pub reason: String,
}

/// An exercise the grants permit, and the authority it rests on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Permit {
    /// The grant exercised.
    pub grant: GrantId,
    /// The grant and every ancestor, from the grant to its root.
    pub path: Vec<GrantId>,
    /// The person the root grant is held by.
    pub root_person: PersonId,
    /// The actions the grant carries.
    pub actions: BTreeSet<Action>,
    /// The model version the grant was judged under.
    pub model_version: u64,
    /// The revision of the permission relationships the decision was made at.
    pub revision: u64,
}

/// A recorded grant change and where it stands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recorded {
    /// The event.
    pub event: GrantEvent,
    /// Its index in the grant log.
    pub index: u64,
    /// Its receipt.
    pub receipt: GrantReceipt,
}

/// The grants: their log, book and permission relationships, and the model
/// they are judged against.
pub struct Grants<S: LeafStore, R: RelationshipStore> {
    pub(super) model: Model,
    pub(super) root_authority: PersonId,
    pub(super) key: Ed25519Identity,
    pub(super) ledger: GrantLedger<S>,
    pub(super) book: GrantBook,
    pub(super) events: Vec<(SignedGrantEvent, GrantReceipt)>,
    pub(super) relationships: R,
}

fn root_matches(request: &RootRequest, grant: &Grant) -> bool {
    let parts = grant.parts();
    parts.issuer == request.caller
        && parts.holder == IdentityId::Person(request.holder)
        && parts.source == Source::Root
        && parts.resource == request.resource
        && parts.relation == request.relation
        && parts.pass_on == request.pass_on
        && parts.window == request.window
}

fn delegation_matches(request: &DelegateRequest, grant: &Grant) -> bool {
    let parts = grant.parts();
    parts.issuer == request.caller
        && parts.holder == request.recipient
        && parts.responsible == request.responsible
        && parts.source == Source::Grant(request.source)
        && parts.resource == request.resource
        && parts.relation == request.relation
        && parts.pass_on == request.pass_on
        && parts.window == request.window
}

impl<S: LeafStore, R: RelationshipStore> Grants<S, R> {
    /// Open the grants over the grant log `reopen` gives, replaying every
    /// event into the book and projecting what the relationships lack.
    pub fn open(
        reopen: Reopen<S>,
        key: Ed25519Identity,
        relationships: R,
        model: Model,
        root_authority: PersonId,
    ) -> Result<Self, GrantError> {
        let (ledger, events) = GrantLedger::open(reopen, key.public_key_bytes())?;
        let mut grants = Self {
            model,
            root_authority,
            key,
            ledger,
            book: GrantBook::new(),
            events: Vec::new(),
            relationships,
        };
        for (signed, coordinate) in events {
            grants.record_committed(signed, coordinate)?;
        }
        grants.project().ok();
        Ok(grants)
    }

    /// The model new requests are judged against.
    pub fn model(&self) -> &Model {
        &self.model
    }

    /// The grant book, as the committed events make it.
    pub fn book(&self) -> &GrantBook {
        &self.book
    }

    /// Every committed grant event with its receipt, in log order.
    pub fn events(&self) -> &[(SignedGrantEvent, GrantReceipt)] {
        &self.events
    }

    /// The number of committed grant events: the revision a fully fresh decision stands at.
    pub fn revision(&self) -> u64 {
        self.events.len() as u64
    }

    /// The grant log, for checkpoints and inclusion proofs.
    pub fn ledger(&self) -> &GrantLedger<S> {
        &self.ledger
    }

    /// The permission engine.
    pub fn relationships(&self) -> &R {
        &self.relationships
    }

    fn reused(operation: OperationId) -> GrantError {
        GrantError::OperationReused {
            operation: operation.to_string(),
        }
    }

    /// Issue a root grant to a person, as the root authority.
    pub fn issue_root(
        &mut self,
        directory: &Projection,
        request: &RootRequest,
        at: u64,
    ) -> Result<Recorded, GrantError> {
        self.settle_for_change()?;
        if let Some(event) = self.answered_event(request.operation) {
            return match event.change() {
                GrantChange::Issue(grant) if root_matches(request, grant) => {
                    self.answer(request.operation)
                }
                _ => Err(Self::reused(request.operation)),
            };
        }
        let grant = judge_root(
            directory,
            &self.model,
            self.root_authority,
            request,
            GrantId::generate()?,
        )?;
        self.commit(GrantEvent::new(
            request.operation,
            request.caller,
            at,
            GrantChange::Issue(Box::new(grant)),
        )?)
    }

    /// Pass on part of a grant the caller holds.
    pub fn delegate(
        &mut self,
        directory: &Projection,
        request: &DelegateRequest,
        at: u64,
    ) -> Result<Recorded, GrantError> {
        self.settle_for_change()?;
        if let Some(event) = self.answered_event(request.operation) {
            return match event.change() {
                GrantChange::Issue(grant) if delegation_matches(request, grant) => {
                    self.answer(request.operation)
                }
                _ => Err(Self::reused(request.operation)),
            };
        }
        let grant = judge_delegation(
            &self.book,
            directory,
            &self.model,
            request,
            at,
            GrantId::generate()?,
        )?;
        self.commit(GrantEvent::new(
            request.operation,
            request.caller,
            at,
            GrantChange::Issue(Box::new(grant)),
        )?)
    }

    /// Revoke a grant, and with it everything derived from it.
    pub fn revoke(&mut self, request: &RevokeRequest, at: u64) -> Result<Recorded, GrantError> {
        self.settle_for_change()?;
        let change = GrantChange::Revoke {
            grant: request.grant,
            reason: request.reason.clone(),
        };
        if let Some(event) = self.answered_event(request.operation) {
            return if event.caller() == request.caller && event.change() == &change {
                self.answer(request.operation)
            } else {
                Err(Self::reused(request.operation))
            };
        }
        judge_revoke(
            &self.book,
            self.root_authority,
            request.caller,
            request.grant,
        )?;
        self.commit(GrantEvent::new(
            request.operation,
            request.caller,
            at,
            change,
        )?)
    }

    /// Refuse unless the relationships reflect every change to a grant on `path`.
    fn fresh(&self, path: &[GrantId], projected: u64) -> Result<(), GrantError> {
        for grant in path {
            let Some(record) = self.book.record(*grant) else {
                continue;
            };
            let latest = record
                .revoked()
                .map_or(record.index(), |revocation| revocation.index);
            if latest >= projected {
                let (signed, _) = usize::try_from(latest)
                    .ok()
                    .and_then(|index| self.events.get(index))
                    .ok_or_else(|| GrantError::GrantUnknown {
                        grant: grant.to_string(),
                    })?;
                return Err(GrantError::ProjectionPending {
                    operation: signed.event().operation().to_string(),
                    grant: grant.to_string(),
                    index: latest,
                });
            }
        }
        Ok(())
    }

    /// Whether the caller may exercise the action on the resource at `at`,
    /// answered with the authority it rests on or refused by the boundary that
    /// blocks it. `at_least` is a revision the caller requires the decision to
    /// reflect, such as the one a receipt names.
    pub fn check(
        &mut self,
        directory: &Projection,
        request: &ExerciseRequest,
        at: u64,
        at_least: Option<u64>,
    ) -> Result<Permit, GrantError> {
        self.settle_log().ok();
        let projected = match self.project() {
            Ok(projected) => projected,
            Err(_) => self.relationships.revision()?,
        };
        if let Some(required) = at_least
            && projected < required
        {
            return Err(GrantError::StaleDecision {
                required,
                projected,
            });
        }
        if let Some(held) = self.ledger.uncertain()
            && let GrantChange::Issue(grant) = held.event.change()
            && grant.holder() == request.caller
            && grant.resource() == &request.resource
        {
            return Err(GrantError::OperationUnresolved {
                operation: held.operation.to_string(),
                grant: held.grant.to_string(),
            });
        }
        let unresolved = self
            .ledger
            .uncertain()
            .and_then(|held| match held.event.change() {
                GrantChange::Revoke { grant, .. } => Some((held.operation, *grant)),
                GrantChange::Issue(_) => None,
            });
        let held = self.relationships.read()?;
        let mut refusal = None;
        let candidates = self.book.held_by(request.caller).filter(|record| {
            record.grant().resource() == &request.resource
                && record.grant().actions().contains(&request.action)
        });
        for record in candidates {
            let grant = record.grant();
            let decided = effective(&self.book, directory, grant.id(), at).and_then(|lineage| {
                if let Some((operation, revoked)) = unresolved
                    && lineage.path.contains(&revoked)
                {
                    return Err(GrantError::OperationUnresolved {
                        operation: operation.to_string(),
                        grant: revoked.to_string(),
                    });
                }
                self.fresh(&lineage.path, projected)?;
                confirm(&held, &lineage.path, at)?;
                Ok(lineage)
            });
            match decided {
                Ok(lineage) => {
                    return Ok(Permit {
                        grant: grant.id(),
                        path: lineage.path,
                        root_person: lineage.root_person,
                        actions: grant.actions().clone(),
                        model_version: grant.parts().model_version,
                        revision: projected,
                    });
                }
                Err(error) => {
                    refusal.get_or_insert(error);
                }
            }
        }
        Err(refusal.unwrap_or_else(|| GrantError::NotHeld {
            identity: request.caller.to_string(),
            resource: request.resource.to_string(),
            action: request.action.to_string(),
        }))
    }
}
