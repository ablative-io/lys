//! The role store, and assigning a role to an agent.
//!
//! [`Roles`] holds the signed role events and the book they fold into. Each
//! act is judged, its grant changes are made through the grants' own
//! admission, and only then is its one role event signed, checked against
//! the book and committed. The events are held in memory: no role event kind
//! is durably signed before the envelope review accepts it.
//!
//! An assign makes the holding at the role's current version and copies
//! each of that version's templates into a grant at grant time: the
//! template's fields with the agent as holder, the source the grant
//! admission admits for the assigning actor, and in the responsible field
//! the person the agent's registration record names, never the template's
//! own value. The holding's end date, when given, is the end of each grant's
//! window. No link is kept by which a later version or edit reaches a
//! granted holding's grants. Every template's grant is judged before any is
//! made, so a refused assign commits no holding, no grant and no event.

use lys_core::Ed25519Identity;
use lys_log_store::LeafStore;

use super::check::{Asked, Check, Facts, RoleCheck, admit, responsible_for};
use super::error::{RoleError, TemplateReason, TemplateRefusal};
use super::events::{RoleChange, RoleEvent, SignedRoleEvent, sign_role_event};
use super::holders::RoleBook;
use super::types::{Capacity, Holding, HoldingId, MovePolicy, RoleId, Template};
use crate::grants::admission::judge_delegation;
use crate::grants::revocation::judge_revoke;
use crate::grants::{
    DelegateRequest, GrantError, GrantId, Grants, RelationshipStore, Resource, RevokeRequest,
    Route, Source, Window,
};
use crate::id::{AgentId, IdentityId};
use crate::operation::OperationId;
use crate::projection::Projection;

/// What a role act is judged and made with.
pub struct Acting<'a, S: LeafStore, R: RelationshipStore> {
    /// The grants, through whose admission every grant change is made.
    pub grants: &'a mut Grants<S, R>,
    /// The directory's projection.
    pub directory: &'a Projection,
    /// The seam every role-version check is answered through.
    pub check: &'a dyn RoleCheck,
    /// When the act is made, by the service's clock.
    pub at: u64,
}

impl<S: LeafStore, R: RelationshipStore> Acting<'_, S, R> {
    /// What the seam may read.
    pub fn facts(&self) -> Facts<'_> {
        Facts {
            directory: self.directory,
            relationships: self.grants.relationships(),
        }
    }

    /// The capacity `actor` holds for `check`, refused `not_permitted` when neither.
    pub fn admit(
        &self,
        actor: IdentityId,
        check: Check,
        project: &Resource,
        holder: Option<AgentId>,
    ) -> Result<Capacity, RoleError> {
        admit(
            self.check,
            &self.facts(),
            &Asked {
                actor,
                check,
                project,
                holder,
                at: self.at,
            },
        )
    }

    /// Whether `actor` is the directory's configured administrator.
    pub fn is_administrator(&self, actor: IdentityId) -> bool {
        actor == IdentityId::Person(self.grants.root_authority())
    }
}

/// The reason row 2.4 gives for the first refusal of a covering grant.
fn refusal(template: &Template, first: Option<(GrantError, bool)>) -> TemplateRefusal {
    let (reason, detail) = match first {
        None => (
            TemplateReason::AboveHeld,
            "no grant the actor holds on this resource carries these actions".to_owned(),
        ),
        Some((error @ GrantError::UseOnly { .. }, lent)) => (
            if lent {
                TemplateReason::LentToYou
            } else {
                TemplateReason::UseOnly
            },
            error.to_string(),
        ),
        Some((error, _)) => (TemplateReason::AboveHeld, error.to_string()),
    };
    TemplateRefusal {
        relation: template.relation().clone(),
        resource: template.resource().clone(),
        reason,
        detail,
    }
}

/// The grant requests copying `templates` to `agent` as `actor`, each
/// judged by the grant admission against a grant `actor` holds, ending at
/// `ends_at`, refused as a whole naming every template refused.
pub(crate) fn plan_copies<S: LeafStore, R: RelationshipStore>(
    acting: &Acting<'_, S, R>,
    actor: IdentityId,
    agent: AgentId,
    templates: &[Template],
    ends_at: Option<u64>,
    check: Check,
) -> Result<Vec<DelegateRequest>, RoleError> {
    let responsible = responsible_for(acting.directory, agent, check)?;
    let book = acting.grants.book();
    let mut requests = Vec::new();
    let mut refusals = Vec::new();
    for template in templates {
        let window = Window::new(template.window().starts_at(), ends_at)?;
        let mut first = None;
        let mut chosen = None;
        let covering = book.held_by(actor).filter(|record| {
            record.revoked().is_none()
                && record.grant().resource() == template.resource()
                && template.actions().is_subset(record.grant().actions())
        });
        for record in covering {
            let request = DelegateRequest {
                operation: OperationId::generate()?,
                caller: actor,
                route: Route::Api,
                source: record.grant().id(),
                recipient: IdentityId::Agent(agent),
                responsible,
                resource: template.resource().clone(),
                relation: template.relation().clone(),
                pass_on: template.pass_on().clone(),
                window,
            };
            let judged = judge_delegation(
                book,
                acting.directory,
                acting.grants.model(),
                &request,
                acting.at,
                GrantId::generate()?,
            );
            match judged {
                Ok(_) => {
                    chosen = Some(request);
                    break;
                }
                Err(error) => {
                    if first.is_none() {
                        let lent = matches!(record.grant().source(), Source::Grant(_));
                        first = Some((error, lent));
                    }
                }
            }
        }
        match chosen {
            Some(request) => requests.push(request),
            None => refusals.push(refusal(template, first)),
        }
    }
    if refusals.is_empty() {
        Ok(requests)
    } else {
        Err(RoleError::TemplatesRefused { refusals })
    }
}

/// Make each planned grant, answering their ids in order.
pub(crate) fn commit_copies<S: LeafStore, R: RelationshipStore>(
    acting: &mut Acting<'_, S, R>,
    requests: &[DelegateRequest],
) -> Result<Vec<GrantId>, RoleError> {
    let mut made = Vec::new();
    for request in requests {
        let recorded = acting.grants.delegate(acting.directory, request, acting.at)?;
        made.push(recorded.event.grant());
    }
    Ok(made)
}

/// The grants of `grants` still unrevoked.
fn live<S: LeafStore, R: RelationshipStore>(
    acting: &Acting<'_, S, R>,
    grants: &[GrantId],
) -> Vec<GrantId> {
    grants
        .iter()
        .copied()
        .filter(|grant| {
            acting
                .grants
                .book()
                .record(*grant)
                .is_some_and(|record| record.revoked().is_none())
        })
        .collect()
}

/// Refuse unless the grant admission lets `actor` withdraw each of `grants`.
pub(crate) fn judge_withdrawals<S: LeafStore, R: RelationshipStore>(
    acting: &Acting<'_, S, R>,
    actor: IdentityId,
    grants: &[GrantId],
) -> Result<(), RoleError> {
    for grant in live(acting, grants) {
        judge_revoke(
            acting.grants.book(),
            acting.grants.root_authority(),
            actor,
            grant,
        )?;
    }
    Ok(())
}

/// Withdraw each of `grants` still unrevoked, as `actor`, for `reason`.
pub(crate) fn withdraw<S: LeafStore, R: RelationshipStore>(
    acting: &mut Acting<'_, S, R>,
    actor: IdentityId,
    grants: &[GrantId],
    reason: &str,
) -> Result<(), RoleError> {
    for grant in live(acting, grants) {
        let request = RevokeRequest {
            operation: OperationId::generate()?,
            caller: actor,
            route: Route::Api,
            grant,
            reason: reason.to_owned(),
        };
        acting.grants.revoke(&request, acting.at)?;
    }
    Ok(())
}

/// The role events and the book they fold into.
pub struct Roles {
    key: Ed25519Identity,
    events: Vec<SignedRoleEvent>,
    book: RoleBook,
}

impl Roles {
    /// An empty store signing its events with the service's `key`.
    pub fn new(key: Ed25519Identity) -> Self {
        Self {
            key,
            events: Vec::new(),
            book: RoleBook::default(),
        }
    }

    /// The book, as the committed events make it.
    pub fn book(&self) -> &RoleBook {
        &self.book
    }

    /// Every committed role event, in the order it was committed.
    pub fn events(&self) -> &[SignedRoleEvent] {
        &self.events
    }

    /// The service's public key, which verifies every role event.
    pub fn service_key(&self) -> [u8; 32] {
        self.key.public_key_bytes()
    }

    /// Refuse an operation that already names a committed role event.
    pub(crate) fn fresh(&self, operation: OperationId) -> Result<(), RoleError> {
        if self.book.answered(operation) {
            return Err(RoleError::OperationReused {
                operation: operation.to_string(),
            });
        }
        Ok(())
    }

    /// Sign `event`, and commit it once the book admits it.
    pub(crate) fn commit(&mut self, event: RoleEvent) -> Result<RoleEvent, RoleError> {
        self.book.check(&event)?;
        let signed = sign_role_event(event, &self.key)?;
        self.book.apply(signed.event())?;
        let event = signed.event().clone();
        self.events.push(signed);
        Ok(event)
    }
}

/// A request to assign a role to an agent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Assign {
    /// The caller's operation id.
    pub operation: OperationId,
    /// The authenticated actor.
    pub actor: IdentityId,
    /// The role.
    pub role: RoleId,
    /// The agent that will hold it.
    pub agent: AgentId,
    /// The holding's end date, or none.
    pub ends_at: Option<u64>,
}

/// Refuse an end date that does not lie after `at`.
pub(crate) fn end_after(ends_at: Option<u64>, at: u64) -> Result<(), RoleError> {
    if ends_at.is_some_and(|ends| ends <= at) {
        return Err(RoleError::EndDateInvalid {
            reason: "an end date lies after the time it is given at",
        });
    }
    Ok(())
}

impl Roles {
    /// Assign a role to an agent, admitted for the person the agent answers
    /// to or an owner of the role's project, at the role's current version.
    pub fn assign<S: LeafStore, R: RelationshipStore>(
        &mut self,
        acting: &mut Acting<'_, S, R>,
        request: &Assign,
    ) -> Result<RoleEvent, RoleError> {
        self.fresh(request.operation)?;
        let role = self.book.role_or_refuse(request.role)?;
        let project = role.project().clone();
        let version = role.current().number();
        let templates = role.current().templates().to_vec();
        let policy = match request.ends_at {
            Some(_) => role.default_policy(),
            None => MovePolicy::DeliberateOnly,
        };
        let capacity = acting.admit(request.actor, Check::Assign, &project, Some(request.agent))?;
        end_after(request.ends_at, acting.at)?;
        let requests = plan_copies(
            acting,
            request.actor,
            request.agent,
            &templates,
            request.ends_at,
            Check::Assign,
        )?;
        let id = HoldingId::generate()?;
        let grants = commit_copies(acting, &requests)?;
        let holding = Holding {
            id,
            holder: request.agent,
            role: request.role,
            version,
            project,
            grants,
            ends_at: request.ends_at,
            policy,
        };
        self.commit(RoleEvent::new(
            request.operation,
            request.actor,
            capacity,
            acting.at,
            RoleChange::HoldingGranted(Box::new(holding)),
        )?)
    }
}
