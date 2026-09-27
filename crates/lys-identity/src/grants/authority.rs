//! The grants' one authority owner: every issue, pass-on, revocation and
//! exercise decision goes through [`Grants`], whichever route it came by.
//!
//! A request is judged by admission, recorded as one grant event, and only
//! then applied and answered. A refused request records nothing. A retry of an
//! answered operation with the same request is answered again and records
//! nothing; the same operation with a different request is refused
//! `OperationReused`.

use std::collections::BTreeSet;

use super::admission::{
    DelegateRequest, RootRequest, Route, effective, judge_delegation, judge_revoke, judge_root,
};
use super::error::GrantError;
use super::events::{GrantChange, GrantEvent};
use super::model::Model;
use super::projection::GrantBook;
use super::types::{Action, Grant, GrantId, Resource, Source};
use crate::id::{IdentityId, PersonId};
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
}

/// A recorded grant change and where it stands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recorded {
    /// The event.
    pub event: GrantEvent,
    /// Its index among the grant events.
    pub index: u64,
}

/// The grants, their book and the model they are judged against.
#[derive(Debug, Clone)]
pub struct Grants {
    model: Model,
    root_authority: PersonId,
    book: GrantBook,
    events: Vec<GrantEvent>,
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

impl Grants {
    /// Grants judged against `model`, with `root_authority` the one person who issues roots.
    pub fn new(model: Model, root_authority: PersonId) -> Self {
        Self {
            model,
            root_authority,
            book: GrantBook::new(),
            events: Vec::new(),
        }
    }

    /// The model new requests are judged against.
    pub fn model(&self) -> &Model {
        &self.model
    }

    /// The grant book.
    pub fn book(&self) -> &GrantBook {
        &self.book
    }

    /// Every recorded grant event, in order.
    pub fn events(&self) -> &[GrantEvent] {
        &self.events
    }

    fn answered(&self, operation: OperationId) -> Option<Recorded> {
        let index = self.book.operation(operation)?;
        let event = self.events.get(usize::try_from(index).ok()?)?;
        Some(Recorded {
            event: event.clone(),
            index,
        })
    }

    fn reused(operation: OperationId) -> GrantError {
        GrantError::OperationReused {
            operation: operation.to_string(),
        }
    }

    fn commit(&mut self, event: GrantEvent) -> Result<Recorded, GrantError> {
        self.book.check(&event)?;
        let index = self.events.len() as u64;
        self.book.apply(&event, index)?;
        self.events.push(event.clone());
        Ok(Recorded { event, index })
    }

    /// Issue a root grant to a person, as the root authority.
    pub fn issue_root(
        &mut self,
        directory: &Projection,
        request: &RootRequest,
        at: u64,
    ) -> Result<Recorded, GrantError> {
        if let Some(recorded) = self.answered(request.operation) {
            return match recorded.event.change() {
                GrantChange::Issue(grant) if root_matches(request, grant) => Ok(recorded),
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
        if let Some(recorded) = self.answered(request.operation) {
            return match recorded.event.change() {
                GrantChange::Issue(grant) if delegation_matches(request, grant) => Ok(recorded),
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
        if let Some(recorded) = self.answered(request.operation) {
            let same = recorded.event.caller() == request.caller
                && recorded.event.change()
                    == &GrantChange::Revoke {
                        grant: request.grant,
                        reason: request.reason.clone(),
                    };
            return if same {
                Ok(recorded)
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
            GrantChange::Revoke {
                grant: request.grant,
                reason: request.reason.clone(),
            },
        )?)
    }

    /// Whether the caller may exercise the action on the resource at `at`,
    /// answered with the authority it rests on or refused by the boundary that blocks it.
    pub fn check(
        &self,
        directory: &Projection,
        request: &ExerciseRequest,
        at: u64,
    ) -> Result<Permit, GrantError> {
        let mut refusal = None;
        let candidates = self.book.held_by(request.caller).filter(|record| {
            record.grant().resource() == &request.resource
                && record.grant().actions().contains(&request.action)
        });
        for record in candidates {
            let grant = record.grant();
            match effective(&self.book, directory, grant.id(), at) {
                Ok(lineage) => {
                    return Ok(Permit {
                        grant: grant.id(),
                        path: lineage.path,
                        root_person: lineage.root_person,
                        actions: grant.actions().clone(),
                        model_version: grant.parts().model_version,
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
