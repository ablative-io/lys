//! Giving the owner relation on a project.
//!
//! Ownership of a project is the explicit relation
//! [`OWNER_RELATION`](crate::grants::types::OWNER_RELATION) on the project, written
//! as one signed grant event through the grants' commit path and rooted in
//! the person it is given to: that person is the grant's holder and its
//! responsible person, and the giver is the event's authorising actor. It is
//! a root of authority with no ancestor grant, so its admission is this
//! module's rule and not a delegation's: a give is admitted only when the
//! giver is the directory's configured administrator, who needs no grant on
//! the project, or already holds the owner relation on that project, as the
//! seam answers at the moment of the give. It is never given to an agent,
//! and holding it passes nothing else on.

use lys_log_store::LeafStore;

use super::check::Check;
use super::error::RoleError;
use super::holding::Acting;
use super::types::Capacity;
use crate::error::IdentityError;
use crate::grants::types::PROJECT_KIND;
use crate::grants::{Recorded, RelationshipStore, Resource};
use crate::id::IdentityId;
use crate::operation::OperationId;

/// A request to give the owner relation on a project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GiveOwner {
    /// The caller's operation id.
    pub operation: OperationId,
    /// The authenticated giver.
    pub giver: IdentityId,
    /// The person who will hold it.
    pub holder: IdentityId,
    /// The project.
    pub project: Resource,
}

/// Give the owner relation on a project, as one signed grant event.
pub fn give_owner<S: LeafStore, R: RelationshipStore>(
    acting: &mut Acting<'_, S, R>,
    request: &GiveOwner,
) -> Result<Recorded, RoleError> {
    if request.project.kind() != PROJECT_KIND {
        return Err(RoleError::NotAProject {
            resource: request.project.to_string(),
        });
    }
    let refused = || RoleError::NotPermitted {
        actor: request.giver.to_string(),
        check: Check::GiveOwner,
    };
    let IdentityId::Person(holder) = request.holder else {
        return Err(refused());
    };
    if !acting.is_administrator(request.giver) {
        let capacity = acting.admit(request.giver, Check::GiveOwner, &request.project, None)?;
        if capacity != Capacity::ProjectOwner {
            return Err(refused());
        }
    }
    if acting.directory.record(request.holder).is_none() {
        return Err(RoleError::Identity(IdentityError::IdentityUnknown {
            identity: request.holder.to_string(),
        }));
    }
    Ok(acting.grants.commit_owner(
        request.operation,
        request.giver,
        holder,
        &request.project,
        acting.at,
    )?)
}
