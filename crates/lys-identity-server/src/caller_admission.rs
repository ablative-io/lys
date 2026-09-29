//! Shared lifecycle admission over the caller's current directory projection.
//! Acting requires Active; Registered people may only authenticate and read their own account.
//! This module takes no locks and changes no directory or session state.

use lys_identity::projection::Projection;
use lys_identity::{Actor, IdentityId, LifecycleState};

use crate::error::ServerError;

/// Resolve the caller and enforce ADR-011/C39: both an acting identity and
/// an agent's responsible person must be Active, before an authority query.
pub fn active_caller(directory: &Projection, actor: &Actor) -> Result<IdentityId, ServerError> {
    resolve(directory, actor, false)
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
        IdentityId::Agent(_) => Err(ServerError::NoPerson),
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
        let person = directory
            .record(identity)
            .and_then(lys_identity::projection::Record::responsible)
            .ok_or(ServerError::NoPerson)?;
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
