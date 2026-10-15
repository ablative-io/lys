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
use std::sync::Arc;

use lys_core::Ed25519Identity;
use lys_log_store::witness::TailWitnessProvider;
use lys_log_store::{LeafStore, SnapshotRefusal};

use super::admission::{DelegateRequest, RootRequest, Route, judge_delegation, judge_root};
use super::error::GrantError;
use super::events::SignedGrantEvent;
use super::events::{GrantChange, GrantEvent};
use super::model::Model;
use super::permission::RelationshipStore;
use super::projection::GrantBook;
use super::receipt::GrantReceipt;
use super::recovery::{GrantLedger, checkpoint_refused};
use super::revocation::judge_revoke;
use super::settlement::ProjectionDegraded;
use super::state;
use super::types::{Action, Grant, GrantId, Mode, Resource, Source};
use super::usage::Unreported;
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
    /// Whether the ancestry starts at administrator standing.
    pub administrator: bool,
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
    /// The shared original failure of the selected relationship reading.
    pub degraded: Option<Arc<ProjectionDegraded>>,
}

impl Permit {
    /// The complete ancestry in the wire's names, including standing.
    #[must_use]
    pub fn path_names(&self) -> Vec<String> {
        super::lineage::path_names(&self.path, self.administrator)
    }
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
    /// The shared original projection failure of this acknowledgement's reading.
    pub degraded: Option<Arc<ProjectionDegraded>>,
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
    pub(super) startup_degraded: Option<Arc<ProjectionDegraded>>,
}

pub(super) fn root_matches(request: &RootRequest, grant: &Grant, mode: Mode) -> bool {
    let parts = grant.parts();
    parts.issuer == request.caller
        && parts.holder == IdentityId::Person(request.holder)
        && parts.source == Source::Root
        && parts.resource == request.resource
        && parts.relation == request.relation
        && parts.pass_on == request.pass_on
        && parts.window == request.window
        && grant.mode() == mode
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
    ///
    /// # Errors
    /// Reports unreadable logs, invalid snapshots, signatures or permission projection failures.
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
    ///
    /// # Errors
    /// Reports unreadable logs, invalid snapshots, signatures or permission projection failures.
    pub fn open_with(
        reopen: Reopen<S>,
        key: Ed25519Identity,
        relationships: R,
        model: Model,
        root_authority: PersonId,
        every: NonZeroU64,
    ) -> Result<Self, GrantError> {
        Self::open_with_tail_provider(
            reopen,
            key,
            relationships,
            model,
            root_authority,
            every,
            None,
        )
    }

    /// Open with an explicit optional capability for authenticated tail readings.
    /// Ordinary permission reads do not acquire or verify tail evidence.
    ///
    /// # Errors
    /// Retains the original opening, state and projection failures.
    pub fn open_with_tail_provider(
        reopen: Reopen<S>,
        key: Ed25519Identity,
        relationships: R,
        model: Model,
        root_authority: PersonId,
        every: NonZeroU64,
        tail_provider: Option<Arc<dyn TailWitnessProvider + Send + Sync>>,
    ) -> Result<Self, GrantError> {
        let (ledger, opening) =
            GrantLedger::open_with_tail_provider(reopen, &key, every, tail_provider)?;
        // A snapshot whose state does not read is refused like any other:
        // the grants are held unready, never rebuilt from history here.
        let (book, folded) = match opening
            .state
            .as_deref()
            .map(|held| state::decode(held, opening.size))
        {
            None => (GrantBook::new(), 0),
            Some(Ok(book)) => (book, opening.size),
            Some(Err(reason)) => {
                return Err(checkpoint_refused(&SnapshotRefusal::StateUnreadable {
                    reason,
                }));
            }
        };
        Self::opened(
            ledger,
            (book, folded, opening.events),
            (key, relationships, model, root_authority),
        )
    }

    /// Take the opening projection failure for its operator report.
    #[must_use]
    pub fn take_startup_degradation(&mut self) -> Option<Arc<ProjectionDegraded>> {
        self.startup_degraded.take()
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
    ///
    /// # Errors
    /// Reports an unreadable or unverifiable log entry.
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
    ///
    /// # Errors
    /// Reports an unreadable or unverifiable log entry.
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
    ///
    /// # Errors
    /// Refuses an unauthorized caller, invalid grant terms, operation reuse or a failed commit.
    pub fn issue_root(
        &mut self,
        directory: &Projection,
        request: &RootRequest,
        at: u64,
    ) -> Result<Recorded, GrantError> {
        self.settle_for_change()?;
        if let Some((event, receipt)) = self.answered(request.operation)? {
            return match event.change() {
                GrantChange::Issue(grant) if root_matches(request, grant, Mode::Outright) => {
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
    ///
    /// # Errors
    /// Refuses an ineffective source, invalid delegation terms, operation reuse or a failed commit.
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
    ///
    /// # Errors
    /// Refuses invalid delegation terms, further pass-on, operation reuse or a failed commit.
    pub fn delegate_once(
        &mut self,
        directory: &Projection,
        request: &DelegateRequest,
        at: u64,
    ) -> Result<Recorded, GrantError> {
        self.delegate_as(directory, request, at, true)
    }

    pub(super) fn is_one_time(&self, grant: GrantId) -> bool {
        self.book
            .grant(grant)
            .is_some_and(super::types::Grant::is_once)
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
            Grant::once(grant.parts().clone())?.with_mode(grant.mode())
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
    ///
    /// # Errors
    /// Refuses an unauthorized revocation, operation reuse or a failed commit.
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
}
