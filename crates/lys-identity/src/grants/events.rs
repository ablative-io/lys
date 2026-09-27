//! The grant event: one grant change and its audit record at once.
//!
//! An event names the caller's operation id, the identity that made the
//! request, when the service recorded it, and the change: a grant issued, or
//! a grant revoked. A grant change is recorded only as one of these events,
//! and both the grant book and the permission relationships are derived from
//! it, never written beside it.

use super::error::GrantError;
use super::types::{Grant, GrantId};
use crate::id::IdentityId;
use crate::operation::OperationId;

/// The longest reason a revocation may give, in bytes.
pub const REVOKE_REASON_MAX_BYTES: usize = 1024;

/// One change to the grants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GrantChange {
    /// A grant is issued, as a root or derived from another grant.
    Issue(Box<Grant>),
    /// A grant is revoked, and with it everything derived from it.
    Revoke {
        /// The grant revoked.
        grant: GrantId,
        /// Why it was revoked.
        reason: String,
    },
}

/// A grant change, before it is signed or after it is verified.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrantEvent {
    operation: OperationId,
    caller: IdentityId,
    recorded_at: u64,
    change: GrantChange,
}

impl GrantEvent {
    /// The event recording `change`, requested by `caller` under `operation`.
    ///
    /// An issued grant names this event's operation as the one that
    /// authorised it, and its issuer is the caller; a revocation names a
    /// reason of 1 to 1024 bytes.
    pub fn new(
        operation: OperationId,
        caller: IdentityId,
        recorded_at: u64,
        change: GrantChange,
    ) -> Result<Self, GrantError> {
        match &change {
            GrantChange::Issue(grant) => {
                if grant.parts().operation != operation {
                    return Err(GrantError::EventMismatch {
                        reason: "an issued grant names the operation of the event that issues it",
                    });
                }
                if grant.parts().issuer != caller {
                    return Err(GrantError::EventMismatch {
                        reason: "an issued grant names the caller as its issuer",
                    });
                }
            }
            GrantChange::Revoke { reason, .. } => {
                if reason.trim().is_empty() || reason.len() > REVOKE_REASON_MAX_BYTES {
                    return Err(GrantError::EventMismatch {
                        reason: "a revocation names its reason in 1 to 1024 bytes",
                    });
                }
            }
        }
        Ok(Self {
            operation,
            caller,
            recorded_at,
            change,
        })
    }

    /// The operation id the caller gave the change.
    pub fn operation(&self) -> OperationId {
        self.operation
    }

    /// The identity that made the request.
    pub fn caller(&self) -> IdentityId {
        self.caller
    }

    /// When the service recorded the change, in seconds since the Unix epoch.
    pub fn recorded_at(&self) -> u64 {
        self.recorded_at
    }

    /// The change.
    pub fn change(&self) -> &GrantChange {
        &self.change
    }

    /// The grant the change is about.
    pub fn grant(&self) -> GrantId {
        match &self.change {
            GrantChange::Issue(grant) => grant.id(),
            GrantChange::Revoke { grant, .. } => *grant,
        }
    }
}
