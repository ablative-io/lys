//! The recipient and source coverage rules shared by every pass-on.

use super::admission::{
    DelegateRequest, Route, active, refuse_kept_from_machines, responsible_is, withheld_from_agents,
};
use super::{
    Grant, GrantError, GrantId, GrantParts, Mode, Model, PassOn, RecipientKind, Relation, Resource,
    Source, Window,
};
use crate::projection::Projection;
use crate::{IdentityId, OperationId, PersonId};
use std::collections::BTreeSet;

/// The wire name of the administrator's standing.
pub const ADMINISTRATOR: &str = "administrator";

/// What is passed on, independently of where the authority comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PassOnRequest {
    /// The operation kept across retries.
    pub operation: OperationId,
    /// The authenticated giver.
    pub caller: IdentityId,
    /// How the request arrived.
    pub route: Route,
    /// The identity receiving the grant.
    pub recipient: IdentityId,
    /// The person answering for the recipient.
    pub responsible: PersonId,
    /// The resource given.
    pub resource: Resource,
    /// The relation requested.
    pub relation: Relation,
    /// What may be passed on in turn.
    pub pass_on: PassOn,
    /// When the grant may be exercised.
    pub window: Window,
}

impl From<&DelegateRequest> for PassOnRequest {
    fn from(request: &DelegateRequest) -> Self {
        Self {
            operation: request.operation,
            caller: request.caller,
            route: request.route,
            recipient: request.recipient,
            responsible: request.responsible,
            resource: request.resource.clone(),
            relation: request.relation.clone(),
            pass_on: request.pass_on.clone(),
            window: request.window,
        }
    }
}

pub(super) struct Authority<'a> {
    pub source: Source,
    pub resource: Option<&'a Resource>,
    pub pass_on: &'a PassOn,
    pub mode: Mode,
    /// The organisation the authority belongs to, when it carries one: a
    /// grant is passed on within it (DIRECTORY-109 R3).
    pub organisation: Option<crate::OrganisationId>,
}

impl Authority<'_> {
    fn name(&self) -> String {
        match self.source {
            Source::Grant(id) => id.to_string(),
            Source::Administrator => ADMINISTRATOR.to_owned(),
            Source::Root => "root".to_owned(),
        }
    }
}

pub(super) fn judge_pass_on(
    directory: &Projection,
    model: &Model,
    request: &PassOnRequest,
    source: &Authority<'_>,
    id: GrantId,
) -> Result<Grant, GrantError> {
    let PassOn::To {
        actions: passable,
        recipients,
    } = source.pass_on
    else {
        return Err(GrantError::UseOnly {
            grant: source.name(),
        });
    };
    let kind = RecipientKind::of(request.recipient);
    if !recipients.contains(&kind) {
        return Err(GrantError::RecipientRefused {
            kind,
            grant: source.name(),
        });
    }
    active(directory, request.recipient)?;
    responsible_is(directory, request.recipient, request.responsible)?;
    if let Some(organisation) = source.organisation
        && directory.organisation_of(request.recipient) != Some(organisation)
    {
        return Err(GrantError::OrganisationOutside {
            grant: source.name(),
            organisation: organisation.to_string(),
            recipient: request.recipient.to_string(),
        });
    }
    if source
        .resource
        .is_some_and(|resource| resource != &request.resource)
    {
        return Err(GrantError::ResourceOutside {
            requested: request.resource.to_string(),
            source_grant: source.name(),
        });
    }
    let within = model.within_on(request.resource.kind(), &request.relation, passable)?;
    if kind == RecipientKind::Agent {
        withheld_from_agents(model, request, &within.actions)?;
    }
    if kind == RecipientKind::Machine {
        refuse_kept_from_machines(
            &request.recipient.to_string(),
            request.resource.kind(),
            &within.actions,
        )?;
    }
    if let PassOn::To {
        actions: onward,
        recipients: onward_to,
    } = &request.pass_on
    {
        if onward_to.contains(&RecipientKind::Machine) {
            refuse_kept_from_machines(
                "a grant passed on to machines",
                request.resource.kind(),
                onward,
            )?;
        }
        if !onward.is_subset(passable) || !onward_to.is_subset(recipients) {
            return Err(GrantError::PassOnBeyondSource {
                source_grant: source.name(),
            });
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
        source: source.source,
        window: request.window,
        model_version: within.model_version,
        operation: request.operation,
    })
    .map(|grant| {
        grant
            .with_mode(source.mode)
            .with_organisation(source.organisation)
    })
}

pub(super) fn recipients() -> BTreeSet<RecipientKind> {
    BTreeSet::from([
        RecipientKind::Person,
        RecipientKind::Agent,
        RecipientKind::ServiceAccount,
        RecipientKind::Connector,
        RecipientKind::Machine,
    ])
}
