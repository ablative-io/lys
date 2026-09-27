//! The four lifecycle states, and the only transitions between them.
//!
//! Registration yields `registered`. Activate, suspend, reinstate and retire
//! are the only transitions, and a retired identity is never reactivated. A
//! transition outside the table is refused by name and records nothing. The
//! state is recorded, never enforced: nothing in this crate reads it to admit
//! or refuse an action (CN11).

use std::fmt;

use crate::error::IdentityError;

/// Where an identity is in its life.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LifecycleState {
    /// Exists in the directory, with no grant and no credential handle.
    Registered,
    /// May act, within its grants.
    Active,
    /// Kept whole, with its grants retained but not effective.
    Suspended,
    /// Permanent. Its history is kept and it is never reactivated.
    Retired,
}

/// A change of lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Transition {
    /// Registered to active.
    Activate,
    /// Active to suspended, with a reason.
    Suspend,
    /// Suspended to active.
    Reinstate,
    /// Active or suspended to retired, with a reason.
    Retire,
}

impl Transition {
    /// The state this transition leads to from `from`, or its refusal by name.
    pub fn target(self, from: LifecycleState) -> Result<LifecycleState, IdentityError> {
        match (self, from) {
            (Self::Activate, LifecycleState::Registered)
            | (Self::Reinstate, LifecycleState::Suspended) => Ok(LifecycleState::Active),
            (Self::Suspend, LifecycleState::Active) => Ok(LifecycleState::Suspended),
            (Self::Retire, LifecycleState::Active | LifecycleState::Suspended) => {
                Ok(LifecycleState::Retired)
            }
            _ => Err(IdentityError::TransitionRefused {
                transition: self,
                from,
            }),
        }
    }

    /// Whether this transition must name the reason it was made.
    pub fn requires_reason(self) -> bool {
        matches!(self, Self::Suspend | Self::Retire)
    }
}

impl fmt::Display for LifecycleState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Registered => "registered",
            Self::Active => "active",
            Self::Suspended => "suspended",
            Self::Retired => "retired",
        })
    }
}

impl fmt::Display for Transition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Activate => "activate",
            Self::Suspend => "suspend",
            Self::Reinstate => "reinstate",
            Self::Retire => "retire",
        })
    }
}
