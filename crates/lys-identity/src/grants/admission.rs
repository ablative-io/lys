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
//! passed one. A machine never issues, passes on or approves, and is never
//! given a responsibility a person keeps (ACCESS-005 R1).

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
use crate::OrganisationId;
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
        // A connector's record names the administrator who approved its app,
        // and a machine's the administrator who asked for its code.
        // An organisation's record names its founder; its administrator
        // grant is the one grant it issues (DIRECTORY-109 R3).
        IdentityId::Organisation(_) => record.founder().ok_or_else(|| {
            GrantError::from(IdentityError::IdentityUnknown {
                identity: identity.to_string(),
            })
        }),
        IdentityId::Agent(_)
        | IdentityId::ServiceAccount(_)
        | IdentityId::Connector(_)
        | IdentityId::Machine(_) => record.responsible().ok_or_else(|| {
            GrantError::from(IdentityError::IdentityUnknown {
                identity: identity.to_string(),
            })
        }),
    }
}

/// Refuse unless the directory records `identity` as active.
pub(super) fn active(directory: &Projection, identity: IdentityId) -> Result<(), GrantError> {
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
pub(super) fn responsible_is(
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
///
/// # Errors
/// Refuses a machine, a caller who does not hold the source, or an inactive or unrelated boss delegation.
pub fn delegation_authority(
    directory: &Projection,
    source: &Grant,
    caller: IdentityId,
    recipient: IdentityId,
) -> Result<(), GrantError> {
    // Holding a grant that may be passed on admits a holder, but never a
    // machine: a machine issues nothing, so no grant is ever its gift.
    if let IdentityId::Machine(_) = caller {
        return Err(GrantError::MachineRefused {
            machine: caller.to_string(),
            act: "issues or passes on a grant".to_owned(),
        });
    }
    if source.holder() == caller {
        return Ok(());
    }
    let refused = || GrantError::NotHolder {
        caller: caller.to_string(),
        grant: source.id().to_string(),
    };
    let person = match caller {
        IdentityId::Person(person) => person,
        IdentityId::Agent(_)
        | IdentityId::ServiceAccount(_)
        | IdentityId::Connector(_)
        | IdentityId::Machine(_)
        | IdentityId::Organisation(_) => {
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

/// Admit a right an agent holds on an agent it registered (AGENTS-015 R2):
/// its issuer is an active person who answers for the agent it is on, so it
/// stands only while that agent answers to the person who gave it.
///
/// # Errors
/// Refuses an inactive issuer, or an agent that is absent or answers to
/// someone else.
fn created_authority(
    directory: &Projection,
    held: &Grant,
    source: &Grant,
) -> Result<(), GrantError> {
    active(directory, held.parts().issuer)?;
    let answers = held
        .resource()
        .id()
        .parse::<crate::id::AgentId>()
        .ok()
        .and_then(|agent| directory.record(IdentityId::Agent(agent)))
        .and_then(crate::projection::Record::responsible);
    if answers != Some(held.responsible()) {
        return Err(GrantError::NotHolder {
            caller: held.parts().issuer.to_string(),
            grant: source.id().to_string(),
        });
    }
    Ok(())
}

/// The checked ancestry of `grant`, refused unless every grant on it is
/// unrevoked, started and unended at `at`, held by an active identity and
/// answered for by the person the directory records.
///
/// # Errors
/// Refuses broken lineage, revoked or expired grants, inactive identities or mismatched responsibility.
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
        if held.source() == Source::Administrator {
            active(directory, held.parts().issuer)?;
        }
        if let Source::Grant(source) = held.source() {
            let source = book
                .grant(source)
                .ok_or_else(|| GrantError::SourceUnknown {
                    grant: source.to_string(),
                })?;
            if super::lineage::created_hop(held, source) {
                created_authority(directory, held, source)?;
            } else if held.parts().issuer != source.holder() {
                delegation_authority(directory, source, held.parts().issuer, held.holder())?;
            }
        }
    }
    Ok(lineage)
}

/// Whose authority a root grant rests on (ACCESS-009 R2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RootAuthority {
    /// The install's root person, for an install-level resource.
    Install(PersonId),
    /// A person shown to hold organisation.grant on the organisation, for a
    /// resource it owns or that no organisation owns yet.
    Organisation(OrganisationId),
}

/// Whether `resource` is the install's own, rooted by its root person and
/// owned by no organisation: the directory's collections (apps, agents).
#[must_use]
pub fn install_level(resource: &Resource) -> bool {
    resource.kind() == "directory"
}

/// Judge a root request, answering the grant it would issue.
///
/// # Errors
/// Refuses a caller other than the install's root person for an
/// install-level resource, a holder outside the organisation the authority
/// names, inactive people, unknown relations or inadmissible pass-on terms.
pub fn judge_root(
    directory: &Projection,
    model: &Model,
    authority: RootAuthority,
    request: &RootRequest,
    id: GrantId,
) -> Result<Grant, GrantError> {
    match authority {
        RootAuthority::Install(root) => {
            if request.caller != IdentityId::Person(root) {
                return Err(GrantError::RootAuthorityRefused {
                    caller: request.caller.to_string(),
                });
            }
        }
        // A root is issued within its organisation (ACCESS-009 R3): the
        // holder is of the organisation the authority rests on.
        RootAuthority::Organisation(organisation) => {
            if directory.organisation_of(IdentityId::Person(request.holder)) != Some(organisation) {
                return Err(GrantError::OrganisationOutside {
                    grant: format!("a root on {}", request.resource),
                    organisation: organisation.to_string(),
                    recipient: request.holder.to_string(),
                });
            }
        }
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
        refuse_withheld(model, &request.relation, kind, onward)?;
    }
    if let PassOn::To {
        actions: onward,
        recipients,
    } = &request.pass_on
        && recipients.contains(&RecipientKind::Machine)
    {
        refuse_kept_from_machines("a grant passed on to machines", kind, onward)?;
    }
    // A relation the kind does not hold may be one of its roles (ACCESS-004
    // R1): the grant then names the role, and is judged as its actions.
    let (actions, role) = match model.actions_on(kind, &request.relation) {
        Ok(actions) => (actions.clone(), false),
        Err(unknown) => match model.role_on(kind, &request.relation) {
            Some(actions) => (actions.clone(), true),
            None => return Err(unknown),
        },
    };
    let grant = Grant::new(GrantParts {
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
    })?
    // The organisation the authority rests on; an install-level grant
    // carries its holder's, read from the directory and never defaulted:
    // a holder attached to none leaves it unmarked.
    .with_organisation(match authority {
        RootAuthority::Install(_) => directory.organisation_of(IdentityId::Person(request.holder)),
        RootAuthority::Organisation(organisation) => Some(organisation),
    });
    Ok(if role { grant.as_role() } else { grant })
}

/// Refuse, by name, `relation` when any of `actions` on an object of `kind`
/// is one no agent may hold: the one rule every gift to an agent is judged
/// by, and an agent's request for access when it is asked.
///
/// # Errors
/// `GrantError::WithheldFromAgents`, naming the relation and every act it
/// carries that no agent may hold.
pub fn refuse_withheld<'a>(
    model: &Model,
    relation: &Relation,
    kind: &str,
    actions: impl IntoIterator<Item = &'a Action>,
) -> Result<(), GrantError> {
    let withheld: BTreeSet<&str> = actions
        .into_iter()
        .filter(|action| !model.agent_may_hold(kind, action))
        .map(Action::as_str)
        .collect();
    if withheld.is_empty() {
        return Ok(());
    }
    Err(GrantError::WithheldFromAgents {
        relation: relation.to_string(),
        withheld: withheld.into_iter().collect::<Vec<_>>().join(", "),
    })
}

/// Refuse, by name, any of `actions` on an object of `kind` that is a
/// responsibility a person keeps, for `machine` (ACCESS-005 R1).
pub(super) fn refuse_kept_from_machines<'a>(
    machine: &str,
    kind: &str,
    actions: impl IntoIterator<Item = &'a Action>,
) -> Result<(), GrantError> {
    let kept: BTreeSet<&str> = actions
        .into_iter()
        .map(Action::as_str)
        .filter(|action| !super::machine_may_hold(kind, action))
        .collect();
    if kept.is_empty() {
        return Ok(());
    }
    Err(GrantError::MachineRefused {
        machine: machine.to_owned(),
        act: format!(
            "holds {}, which a person keeps",
            kept.into_iter().collect::<Vec<_>>().join(", ")
        ),
    })
}

/// Refuse, by name, an agent's grant that carries, or would let the agent
/// pass on, an act no agent may hold, whatever its source.
pub(super) fn withheld_from_agents(
    model: &Model,
    request: &super::delegation::PassOnRequest,
    actions: &BTreeSet<Action>,
) -> Result<(), GrantError> {
    let onward = match &request.pass_on {
        PassOn::To {
            actions: onward, ..
        } => Some(onward),
        PassOn::UseOnly => None,
    };
    refuse_withheld(
        model,
        &request.relation,
        request.resource.kind(),
        actions.iter().chain(onward.into_iter().flatten()),
    )
}

/// Judge a request to pass on part of a grant, answering the grant it would issue.
///
/// # Errors
/// Refuses ineffective sources, inadmissible recipients or terms outside the source authority.
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
    let grant = super::delegation::judge_pass_on(
        directory,
        model,
        &super::delegation::PassOnRequest::from(request),
        &super::delegation::Authority {
            source: Source::Grant(request.source),
            resource: Some(source.resource()),
            pass_on: source.pass_on(),
            mode: source.mode(),
            organisation: source.organisation(),
        },
        id,
    )?;
    for ancestor in &lineage.path {
        if let Some(ancestor) = book.grant(*ancestor) {
            check_end(request.window.ends_at(), ancestor)?;
        }
    }
    Ok(grant)
}
