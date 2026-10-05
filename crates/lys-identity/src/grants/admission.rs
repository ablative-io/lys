//! Admission: whether a grant is effective now, and whether a request to
//! issue, pass on or revoke one is admitted.
//!
//! Each judgement reads the grant book, the directory and the model, and
//! nothing else. It changes nothing, so a refused request leaves no trace but
//! its refusal. Passing on is judged against the caller's own current
//! authority, every effective ancestor and the model; being a person or an
//! agent is no permit, and pass-on is established only by an affirmative
//! pass-on member of the source grant. An agent is never given, nor let
//! pass on, an act withheld from agents, and no root lets an agent be
//! passed one.

use std::collections::BTreeSet;

use super::error::GrantError;
use super::expiry::within_window;
use super::lineage::{Lineage, check_end};
use super::model::Model;
use super::projection::GrantBook;
use super::revocation::unrevoked;
use super::types::{
    Action, Grant, GrantId, GrantParts, PassOn, RecipientKind, Relation, Resource, Source, Window,
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
        // A connector's record names the administrator who approved its app.
        IdentityId::Agent(_) | IdentityId::ServiceAccount(_) | IdentityId::Connector(_) => {
            record.responsible().ok_or_else(|| {
                GrantError::from(IdentityError::IdentityUnknown {
                    identity: identity.to_string(),
                })
            })
        }
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

/// Admit a holder, or the active person answering for a boss and its direct child.
pub fn delegation_authority(
    directory: &Projection,
    source: &Grant,
    caller: IdentityId,
    recipient: IdentityId,
) -> Result<(), GrantError> {
    if source.holder() == caller {
        return Ok(());
    }
    let refused = || GrantError::NotHolder {
        caller: caller.to_string(),
        grant: source.id().to_string(),
    };
    let person = match caller {
        IdentityId::Person(person) => person,
        IdentityId::Agent(_) | IdentityId::ServiceAccount(_) | IdentityId::Connector(_) => {
            return Err(refused());
        }
    };
    if !super::lineage::is_agent(source.holder())
        || !super::lineage::is_agent(recipient)
        || source.responsible() != person
        || directory
            .record(source.holder())
            .and_then(crate::projection::Record::responsible)
            != Some(person)
        || !directory.record(recipient).is_some_and(|child| {
            child.responsible() == Some(person) && child.reports_to() == Some(source.holder())
        })
    {
        return Err(refused());
    }
    active(directory, caller)?;
    active(directory, source.holder())?;
    Ok(())
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
    unrevoked(book, &lineage)?;
    within_window(book, &lineage, at)?;
    for id in &lineage.path {
        let held = book.grant(*id).ok_or_else(|| GrantError::SourceUnknown {
            grant: id.to_string(),
        })?;
        active(directory, held.holder())?;
        responsible_is(directory, held.holder(), held.responsible())?;
        if let Source::Grant(source) = held.source() {
            let source = book
                .grant(source)
                .ok_or_else(|| GrantError::SourceUnknown {
                    grant: source.to_string(),
                })?;
            if held.parts().issuer != source.holder() {
                delegation_authority(directory, source, held.parts().issuer, held.holder())?;
            }
        }
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
    let kind = request.resource.kind();
    if let PassOn::To {
        actions: onward,
        recipients,
    } = &request.pass_on
        && recipients.contains(&RecipientKind::Agent)
    {
        refuse_withheld(&request.relation, kind, onward)?;
    }
    let actions = model.actions_on(kind, &request.relation)?.clone();
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
        model_version: model.version_on(kind),
        operation: request.operation,
    })
}

/// Refuse, by name, `relation` when any of `actions` on an object of `kind`
/// is one no agent may hold.
fn refuse_withheld<'a>(
    relation: &Relation,
    kind: &str,
    actions: impl IntoIterator<Item = &'a Action>,
) -> Result<(), GrantError> {
    let withheld: BTreeSet<&str> = actions
        .into_iter()
        .map(Action::as_str)
        .filter(|action| !super::agent_may_hold(kind, action))
        .collect();
    if withheld.is_empty() {
        return Ok(());
    }
    Err(GrantError::WithheldFromAgents {
        relation: relation.to_string(),
        withheld: withheld.into_iter().collect::<Vec<_>>().join(", "),
    })
}

/// Refuse, by name, an agent's grant that carries, or would let the agent
/// pass on, an act no agent may hold, whatever its source.
fn withheld_from_agents(
    request: &DelegateRequest,
    actions: &BTreeSet<Action>,
) -> Result<(), GrantError> {
    let onward = match &request.pass_on {
        PassOn::To {
            actions: onward, ..
        } => Some(onward),
        PassOn::UseOnly => None,
    };
    refuse_withheld(
        &request.relation,
        request.resource.kind(),
        actions.iter().chain(onward.into_iter().flatten()),
    )
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
    delegation_authority(directory, source, request.caller, request.recipient)?;
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
    active(directory, request.recipient)?;
    responsible_is(directory, request.recipient, request.responsible)?;
    if &request.resource != source.resource() {
        return Err(GrantError::ResourceOutside {
            requested: request.resource.to_string(),
            source_grant: request.source.to_string(),
        });
    }
    let within = model.within_on(request.resource.kind(), &request.relation, passable)?;
    if kind == RecipientKind::Agent {
        withheld_from_agents(request, &within.actions)?;
    }
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
