//! Admission: whether a grant is effective now, and whether a request to
//! issue, pass on or revoke one is admitted.
//!
//! Each judgement reads the grant book, the directory and the model, and
//! nothing else. It changes nothing, so a refused request leaves no trace but
//! its refusal. Passing on is judged against the caller's own current
//! authority, every effective ancestor and the model; being a person or an
//! agent is neither a permit nor a prohibition, and pass-on is established
//! only by an affirmative pass-on member of the source grant.

use super::error::GrantError;
use super::lineage::{Lineage, check_end};
use super::model::Model;
use super::projection::GrantBook;
use super::types::{
    Grant, GrantId, GrantParts, PassOn, RecipientKind, Relation, Resource, Source, Window,
};
use crate::error::IdentityError;
use crate::id::{IdentityId, PersonId};
use crate::lifecycle::LifecycleState;
use crate::operation::OperationId;
use crate::projection::Projection;

/// The way a request reached the service. It is recorded for the caller's
/// own account and never changes a decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Route {
    /// The browser screens.
    Browser,
    /// The API, called directly.
    Api,
    /// An agent's tool.
    Tool,
}

/// A request to issue a root grant to a person, under the root authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RootRequest {
    /// The caller's operation id, kept across retries.
    pub operation: OperationId,
    /// The authenticated caller.
    pub caller: IdentityId,
    /// How the request arrived.
    pub route: Route,
    /// The person who will hold the grant, and answer for it.
    pub holder: PersonId,
    /// The object it is on.
    pub resource: Resource,
    /// The relation granted.
    pub relation: Relation,
    /// What the holder may pass on.
    pub pass_on: PassOn,
    /// When it may be exercised.
    pub window: Window,
}

/// A request to pass on part of a grant the caller holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DelegateRequest {
    /// The caller's operation id, kept across retries.
    pub operation: OperationId,
    /// The authenticated caller, a person or an agent.
    pub caller: IdentityId,
    /// How the request arrived.
    pub route: Route,
    /// The grant the caller passes on part of.
    pub source: GrantId,
    /// The identity that will hold the new grant.
    pub recipient: IdentityId,
    /// The person the caller names as responsible for the recipient.
    pub responsible: PersonId,
    /// The object it is on.
    pub resource: Resource,
    /// The relation requested.
    pub relation: Relation,
    /// What the recipient may pass on in turn.
    pub pass_on: PassOn,
    /// When it may be exercised.
    pub window: Window,
}

/// The person the directory records as answering for `identity`.
fn answered_for_by(directory: &Projection, identity: IdentityId) -> Result<PersonId, GrantError> {
    let record = directory
        .record(identity)
        .ok_or_else(|| IdentityError::IdentityUnknown {
            identity: identity.to_string(),
        })?;
    match identity {
        IdentityId::Person(person) => Ok(person),
        IdentityId::Agent(_) => record.responsible().ok_or_else(|| {
            GrantError::from(IdentityError::IdentityUnknown {
                identity: identity.to_string(),
            })
        }),
    }
}

/// Refuse unless the directory records `identity` as active.
fn active(directory: &Projection, identity: IdentityId) -> Result<(), GrantError> {
    let record = directory
        .record(identity)
        .ok_or_else(|| IdentityError::IdentityUnknown {
            identity: identity.to_string(),
        })?;
    match record.state() {
        LifecycleState::Active => Ok(()),
        state => Err(GrantError::IdentityNotActive {
            identity: identity.to_string(),
            state,
        }),
    }
}

/// Refuse unless `named` is the person the directory records for `identity`.
fn responsible_is(
    directory: &Projection,
    identity: IdentityId,
    named: PersonId,
) -> Result<(), GrantError> {
    let recorded = answered_for_by(directory, identity)?;
    if recorded == named {
        Ok(())
    } else {
        Err(GrantError::ResponsibleMismatch {
            identity: identity.to_string(),
            named: named.to_string(),
            recorded: recorded.to_string(),
        })
    }
}

/// The checked ancestry of `grant`, refused unless every grant on it is
/// unrevoked, started and unended at `at`, held by an active identity and
/// answered for by the person the directory records.
pub fn effective(
    book: &GrantBook,
    directory: &Projection,
    grant: GrantId,
    at: u64,
) -> Result<Lineage, GrantError> {
    let lineage = book.lineage(grant)?;
    for id in &lineage.path {
        let record = book.record(*id).ok_or_else(|| GrantError::SourceUnknown {
            grant: id.to_string(),
        })?;
        if record.revoked().is_some() {
            return Err(GrantError::Revoked {
                grant: id.to_string(),
            });
        }
    }
    if let Some((ended_at, by)) = lineage.ends
        && at >= ended_at
    {
        return Err(GrantError::Expired {
            grant: by.to_string(),
            ended_at,
        });
    }
    for id in &lineage.path {
        let held = book.grant(*id).ok_or_else(|| GrantError::SourceUnknown {
            grant: id.to_string(),
        })?;
        if at < held.window().starts_at() {
            return Err(GrantError::NotStarted {
                grant: id.to_string(),
                starts_at: held.window().starts_at(),
            });
        }
        active(directory, held.holder())?;
        responsible_is(directory, held.holder(), held.responsible())?;
    }
    Ok(lineage)
}

/// Judge a root request, answering the grant it would issue.
pub fn judge_root(
    directory: &Projection,
    model: &Model,
    root_authority: PersonId,
    request: &RootRequest,
    id: GrantId,
) -> Result<Grant, GrantError> {
    if request.caller != IdentityId::Person(root_authority) {
        return Err(GrantError::RootAuthorityRefused {
            caller: request.caller.to_string(),
        });
    }
    active(directory, request.caller)?;
    active(directory, IdentityId::Person(request.holder))?;
    let actions = model.actions(&request.relation)?.clone();
    Grant::new(GrantParts {
        id,
        issuer: request.caller,
        holder: IdentityId::Person(request.holder),
        responsible: request.holder,
        resource: request.resource.clone(),
        relation: request.relation.clone(),
        actions,
        pass_on: request.pass_on.clone(),
        source: Source::Root,
        window: request.window,
        model_version: model.version(),
        operation: request.operation,
    })
}

/// Judge a request to pass on part of a grant, answering the grant it would issue.
pub fn judge_delegation(
    book: &GrantBook,
    directory: &Projection,
    model: &Model,
    request: &DelegateRequest,
    at: u64,
    id: GrantId,
) -> Result<Grant, GrantError> {
    let source = book
        .grant(request.source)
        .ok_or_else(|| GrantError::SourceUnknown {
            grant: request.source.to_string(),
        })?;
    if source.holder() != request.caller {
        return Err(GrantError::NotHolder {
            caller: request.caller.to_string(),
            grant: request.source.to_string(),
        });
    }
    let lineage = effective(book, directory, request.source, at)?;
    let PassOn::To {
        actions: passable,
        recipients,
    } = source.pass_on()
    else {
        return Err(GrantError::UseOnly {
            grant: request.source.to_string(),
        });
    };
    let kind = RecipientKind::of(request.recipient);
    if !recipients.contains(&kind) {
        return Err(GrantError::RecipientRefused {
            kind,
            grant: request.source.to_string(),
        });
    }
    responsible_is(directory, request.recipient, request.responsible)?;
    if &request.resource != source.resource() {
        return Err(GrantError::ResourceOutside {
            requested: request.resource.to_string(),
            source_grant: request.source.to_string(),
        });
    }
    let within = model.within(&request.relation, passable)?;
    if let PassOn::To {
        actions: onward,
        recipients: onward_to,
    } = &request.pass_on
        && (!onward.is_subset(passable) || !onward_to.is_subset(recipients))
    {
        return Err(GrantError::PassOnBeyondSource {
            source_grant: request.source.to_string(),
        });
    }
    for ancestor in &lineage.path {
        if let Some(ancestor) = book.grant(*ancestor) {
            check_end(request.window.ends_at(), ancestor)?;
        }
    }
    Grant::new(GrantParts {
        id,
        issuer: request.caller,
        holder: request.recipient,
        responsible: request.responsible,
        resource: request.resource.clone(),
        relation: request.relation.clone(),
        actions: within.actions,
        pass_on: request.pass_on.clone(),
        source: Source::Grant(request.source),
        window: request.window,
        model_version: within.model_version,
        operation: request.operation,
    })
}

/// Refuse a revocation unless `caller` issued the grant, holds a grant it
/// derives from, or is the root authority.
pub fn judge_revoke(
    book: &GrantBook,
    root_authority: PersonId,
    caller: IdentityId,
    grant: GrantId,
) -> Result<(), GrantError> {
    let record = book.record(grant).ok_or_else(|| GrantError::GrantUnknown {
        grant: grant.to_string(),
    })?;
    if record.revoked().is_some() {
        return Err(GrantError::AlreadyRevoked {
            grant: grant.to_string(),
        });
    }
    if caller == IdentityId::Person(root_authority) || record.grant().parts().issuer == caller {
        return Ok(());
    }
    let lineage = book.lineage(grant)?;
    let holds_ancestor = lineage
        .path
        .iter()
        .skip(1)
        .filter_map(|ancestor| book.grant(*ancestor))
        .any(|ancestor| ancestor.holder() == caller);
    if holds_ancestor {
        Ok(())
    } else {
        Err(GrantError::RevokeRefused {
            caller: caller.to_string(),
            grant: grant.to_string(),
        })
    }
}
