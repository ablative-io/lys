//! The grant authority's part of the shared channel membership capability
//! (ACCESS-006 R1): the identity kind a subject must state, and whether a
//! resource is placed under the workspace a membership question names.
//!
//! Membership is decided by the one grant authority, through the same
//! decision every check makes; these are the bindings judged before it is
//! asked, so a forged kind, a guessed id or a resource of another
//! workspace is refused by name and never looked up.

use super::types::Resource;
use crate::id::IdentityId;

/// The kind `identity` is, as the membership contract names it.
#[must_use]
pub fn identity_kind(identity: &IdentityId) -> &'static str {
    match identity {
        IdentityId::Person(_) => "person",
        IdentityId::Agent(_) => "agent",
        IdentityId::ServiceAccount(_) => "service_account",
        IdentityId::Connector(_) => "connector",
    }
}

/// Whether `resource` is `workspace`, or is placed, through the parents
/// `parent` answers, under it. Each placement is read once: the walk ends
/// at a resource placed nowhere or at the first one it has already passed,
/// so a cycle ends it without reaching. Reaching a workspace says nothing
/// of any right on it; that is the grant authority's to decide.
pub fn placed_within(
    resource: &Resource,
    workspace: &Resource,
    mut parent: impl FnMut(&Resource) -> Option<Resource>,
) -> bool {
    let mut passed = vec![resource.clone()];
    let mut current = resource.clone();
    loop {
        if &current == workspace {
            return true;
        }
        let Some(next) = parent(&current) else {
            return false;
        };
        if passed.contains(&next) {
            return false;
        }
        passed.push(next.clone());
        current = next;
    }
}

#[cfg(test)]
#[path = "channel_membership_tests.rs"]
mod tests;
