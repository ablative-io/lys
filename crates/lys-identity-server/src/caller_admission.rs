//! Shared lifecycle admission over the caller's current directory projection.
//! Registration is sufficient to act; suspension and retirement are not.
//! This module takes no locks and changes no directory or session state.

use lys_identity::projection::Projection;
use lys_identity::{Actor, IdentityId, LifecycleState};

use crate::error::ServerError;

/// Resolve the caller's bound person or agent and refuse a suspended or retired
/// identity. Registered people retain the authority of existing installations.
pub fn active_caller(directory: &Projection, actor: &Actor) -> Result<IdentityId, ServerError> {
    let identity = directory
        .person_for(actor.binding())
        .map(IdentityId::Person)
        .or_else(|| directory.agent_for(actor.binding()).map(IdentityId::Agent))
        .ok_or(ServerError::NoPerson)?;
    let record = directory.record(identity).ok_or(ServerError::NoPerson)?;
    match record.state() {
        LifecycleState::Registered | LifecycleState::Active => Ok(identity),
        state @ (LifecycleState::Suspended | LifecycleState::Retired) => {
            Err(ServerError::Inactive {
                identity: identity.to_string(),
                state,
            })
        }
    }
}
