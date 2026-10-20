//! Shared lifecycle admission over the caller's current directory projection.
//! Acting requires Active; Registered people may only authenticate and read their own account.
//! This module takes no locks and changes no directory or session state.

use lys_identity::projection::Projection;
use lys_identity::{Actor, IdentityId, LifecycleState};

use crate::error::ServerError;

/// Resolve the caller and enforce ADR-011/C39: both an acting identity and
/// an agent's responsible person must be Active, before an authority query.
///
/// # Errors
/// Refuses an unknown or inactive acting identity or an agent whose
/// responsible person is absent or inactive.
pub fn active_caller(directory: &Projection, actor: &Actor) -> Result<IdentityId, ServerError> {
    resolve(directory, actor, false)
}

/// The organisation the active caller acts within (DIRECTORY-109 R1): the
/// one their own record carries. A caller with a record under no
/// organisation (a login admitted as the configured administrator before
/// first-run setup, or an identity the directory does not hold) is refused
/// by name: nothing is registered into no organisation.
///
/// # Errors
/// Refuses an unknown or inactive caller, or one that belongs to no
/// organisation.
pub(crate) fn caller_organisation(
    directory: &Projection,
    actor: &Actor,
) -> Result<lys_identity::OrganisationId, ServerError> {
    let identity = active_caller(directory, actor)?;
    directory
        .organisation_of(identity)
        .ok_or(ServerError::NotAdmitted {
            reason: "the caller belongs to no organisation; first-run setup founds one",
        })
}

/// The organisation a signed-in caller reads within (DIRECTORY-109 R4):
/// the active caller's own, read from the directory. None for a login bound
/// to no identity, or one that is not active, who reads nothing: a record of
/// another organisation, or of none, is answered as absent, never refused,
/// so no read says whether another organisation exists. The reads call this
/// after their own admission; it admits nothing itself.
pub(crate) fn reading_organisation(
    directory: &Projection,
    actor: &Actor,
) -> Option<lys_identity::OrganisationId> {
    active_caller(directory, actor)
        .ok()
        .and_then(|identity| directory.organisation_of(identity))
}

/// Personal mutations admit the caller before reading their requested target.
/// Missing personal authority must not expose a directory lookup refusal.
pub(crate) fn admitted_person(
    directory: &Projection,
    actor: &Actor,
) -> Result<lys_identity::PersonId, ServerError> {
    let refused = || ServerError::NotAdmitted {
        reason: "only an active person may do this",
    };
    let person = directory.person_for(actor.binding()).ok_or_else(refused)?;
    if active_caller(directory, actor)? != IdentityId::Person(person) {
        return Err(refused());
    }
    Ok(person)
}

/// Authentication and bounded own-account reads do not grant acting authority.
/// The lifecycle contract section 2 permits first sign-in before activation.
pub(crate) fn authenticating_caller(
    directory: &Projection,
    actor: &Actor,
) -> Result<IdentityId, ServerError> {
    resolve(directory, actor, true)
}

/// A person's own account/session view. Agents do not borrow this exception.
pub(crate) fn own_account_person(
    directory: &Projection,
    actor: &Actor,
) -> Result<lys_identity::PersonId, ServerError> {
    match authenticating_caller(directory, actor)? {
        IdentityId::Person(person) => Ok(person),
        IdentityId::Agent(_)
        | IdentityId::ServiceAccount(_)
        | IdentityId::Connector(_)
        | IdentityId::Machine(_)
        | IdentityId::Organisation(_) => Err(ServerError::NoPerson),
    }
}

fn resolve(
    directory: &Projection,
    actor: &Actor,
    own_account: bool,
) -> Result<IdentityId, ServerError> {
    let identity = actor
        .provenance()
        .agent()
        .map(IdentityId::Agent)
        .or_else(|| {
            directory
                .person_for(actor.binding())
                .map(IdentityId::Person)
        })
        .or_else(|| directory.agent_for(actor.binding()).map(IdentityId::Agent))
        .ok_or(ServerError::NoPerson)?;
    admit_identity(
        directory,
        identity,
        own_account && matches!(identity, IdentityId::Person(_)),
    )?;
    if let IdentityId::Agent(_) = identity {
        let record = directory.record(identity).ok_or(ServerError::NoPerson)?;
        if let Some(gap) = record.reporting_gap()
            && matches!(gap.identity, IdentityId::Agent(_))
        {
            return Err(lys_identity::IdentityError::AnswersToInactive {
                identity: gap.identity.to_string(),
                state: gap.state,
            }
            .into());
        }
        let person = record.responsible().ok_or(ServerError::NoPerson)?;
        admit_identity(directory, IdentityId::Person(person), false)?;
    }
    Ok(identity)
}

fn admit_identity(
    directory: &Projection,
    identity: IdentityId,
    own_account: bool,
) -> Result<IdentityId, ServerError> {
    let record = directory.record(identity).ok_or(ServerError::NoPerson)?;
    match record.state() {
        LifecycleState::Active => Ok(identity),
        LifecycleState::Registered if own_account => Ok(identity),
        state => Err(ServerError::Inactive {
            identity: identity.to_string(),
            state,
        }),
    }
}
