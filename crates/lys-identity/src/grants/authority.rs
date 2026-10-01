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

use std::collections::{BTreeMap, BTreeSet};
use std::num::NonZeroU64;

use lys_core::Ed25519Identity;
use lys_log_store::LeafStore;

use super::admission::{DelegateRequest, RootRequest, Route, judge_delegation, judge_root};
use super::error::GrantError;
use super::events::SignedGrantEvent;
use super::events::{GrantChange, GrantEvent};
use super::model::Model;
use super::permission::RelationshipStore;
use super::projection::GrantBook;
use super::receipt::GrantReceipt;
use super::recovery::GrantLedger;
use super::revocation::judge_revoke;
use super::state;
use super::types::{Action, Grant, GrantId, Resource, Source};
use super::usage::{self, Unreported};
use crate::id::{IdentityId, PersonId};
use crate::log::Reopen;
use crate::operation::OperationId;
use crate::projection::Projection;
use crate::restart::SNAPSHOT_EVERY;

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

/// The reason a one-time grant's revocation names.
pub const ONE_TIME_SPENT: &str = "a one-time grant is spent by its first use";

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
    /// The index of the use event recording this exercise in the grant log,
    /// or why it could not be recorded. None for an explanation, which
    /// records no use.
    pub use_event: Option<Result<u64, GrantError>>,
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
    pub(super) folded: u64,
    pub(super) relationships: R,
    pub(super) unreported: BTreeMap<GrantId, Unreported>,
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
    /// Open the grants over the grant log `reopen` gives from its signed
    /// snapshot, reading only the leaves after it, fold every event into the
    /// book, and project what the relationships lack. A snapshot is written
    /// every [`SNAPSHOT_EVERY`] entries.
    pub fn open(
        reopen: Reopen<S>,
        key: Ed25519Identity,
        relationships: R,
        model: Model,
        root_authority: PersonId,
    ) -> Result<Self, GrantError> {
        Self::open_with(
            reopen,
            key,
            relationships,
            model,
            root_authority,
            SNAPSHOT_EVERY,
        )
    }

    /// As [`Grants::open`], writing a snapshot every `every` entries.
    pub fn open_with(
        reopen: Reopen<S>,
        key: Ed25519Identity,
        relationships: R,
        model: Model,
        root_authority: PersonId,
        every: NonZeroU64,
    ) -> Result<Self, GrantError> {
        let (mut ledger, opening) = GrantLedger::open(reopen, &key, every)?;
        let read = opening
            .state
            .as_deref()
            .map(|held| state::decode(held, opening.size));
        let (book, folded, events) = match read {
            None => (GrantBook::new(), 0, opening.events),
            Some(Ok(book)) => (book, opening.size, opening.events),
            Some(Err(reason)) => (GrantBook::new(), 0, ledger.refuse_state(reason, &key)?),
        };
        let mut grants = Self {
            model,
            root_authority,
            key,
            ledger,
            book,
            folded,
            relationships,
            unreported: BTreeMap::new(),
        };
        for (signed, coordinate) in events {
            grants.record_committed(&signed, coordinate)?;
        }
        grants.snapshot();
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

    /// Every committed grant event with its receipt, in log order, built from
    /// the log in one pass.
    pub fn events(&self) -> Result<Vec<(SignedGrantEvent, GrantReceipt)>, GrantError> {
        Ok(self
            .ledger
            .entries()?
            .into_iter()
            .map(|(signed, coordinate)| {
                let receipt = GrantReceipt::of(&signed, coordinate);
                (signed, receipt)
            })
            .collect())
    }

    /// The receipt of the event at log index `index`, built from the log from
    /// the nearest checkpoint. `None` past the events folded.
    pub fn receipt(&self, index: u64) -> Result<Option<GrantReceipt>, GrantError> {
        if index >= self.folded {
            return Ok(None);
        }
        Ok(self
            .ledger
            .entry(index)?
            .map(|(signed, coordinate)| GrantReceipt::of(&signed, coordinate)))
    }

    /// The number of committed grant events: the revision a fully fresh decision stands at.
    pub fn revision(&self) -> u64 {
        self.folded
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
        if let Some((event, receipt)) = self.answered(request.operation)? {
            return match event.change() {
                GrantChange::Issue(grant) if root_matches(request, grant) => {
                    self.answer(event, receipt)
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
        self.delegate_as(directory, request, at, false)
    }

    /// Pass on part of a grant as [`Grants::delegate`] does, as a one-time
    /// grant: use-only, and spent by its first exercise.
    pub fn delegate_once(
        &mut self,
        directory: &Projection,
        request: &DelegateRequest,
        at: u64,
    ) -> Result<Recorded, GrantError> {
        self.delegate_as(directory, request, at, true)
    }

    fn is_one_time(&self, grant: GrantId) -> bool {
        self.book.grant(grant).is_some_and(super::types::Grant::is_once)
    }

    fn delegate_as(
        &mut self,
        directory: &Projection,
        request: &DelegateRequest,
        at: u64,
        once: bool,
    ) -> Result<Recorded, GrantError> {
        self.settle_for_change()?;
        if let Some((event, receipt)) = self.answered(request.operation)? {
            return match event.change() {
                GrantChange::Issue(grant)
                    if delegation_matches(request, grant) && grant.is_once() == once =>
                {
                    self.answer(event, receipt)
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
        let grant = if once {
            Grant::once(grant.parts().clone())?
        } else {
            grant
        };
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
        if let Some((event, receipt)) = self.answered(request.operation)? {
            return if event.caller() == request.caller && event.change() == &change {
                self.answer(event, receipt)
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
    pub(super) fn fresh(&self, path: &[GrantId], projected: u64) -> Result<(), GrantError> {
        for grant in path {
            let Some(record) = self.book.record(*grant) else {
                continue;
            };
            let (latest, operation) = record.revoked().map_or(
                (record.index(), record.grant().parts().operation),
                |revocation| (revocation.index, revocation.operation),
            );
            if latest >= projected {
                return Err(GrantError::ProjectionPending {
                    operation: operation.to_string(),
                    grant: grant.to_string(),
                    index: latest,
                });
            }
        }
        Ok(())
    }

    /// Whether the caller may exercise the action on the resource at `at`,
    /// answered with the authority it rests on or refused by the boundary that
    /// blocks it, and the exercise recorded as a use event. `at_least` is a
    /// revision the caller requires the decision to reflect, such as the one a
    /// receipt names. This is the enforcement point: call it when the action is
    /// about to be taken.
    pub fn check(
        &mut self,
        directory: &Projection,
        request: &ExerciseRequest,
        at: u64,
        at_least: Option<u64>,
    ) -> Result<Permit, GrantError> {
        let mut permit = self.explain(directory, request, at, at_least)?;
        let used = self.record_use(request.caller, permit.grant, request.route, at);
        if let Err(error) = &used {
            usage::note(&mut self.unreported, permit.grant, request.route, at, error);
        }
        if self.is_one_time(permit.grant) {
            // A one-time grant admits only an exercise whose use is recorded;
            // that one leaf spends it.
            used.clone()?;
        }
        permit.use_event = Some(used);
        Ok(permit)
    }

    /// The same decision as [`Grants::check`], by the same evaluator at the
    /// same revision, and nothing recorded: answering why an identity may act,
    /// or who can, is not an exercise.
    pub fn explain(
        &mut self,
        directory: &Projection,
        request: &ExerciseRequest,
        at: u64,
        at_least: Option<u64>,
    ) -> Result<Permit, GrantError> {
        let settled = self.settle(at_least)?;
        self.unresolved_issue(request)?;
        let frame = super::frame::Frame::read(self, directory, settled)?;
        self.explain_in(&frame, request, at)
    }
}
