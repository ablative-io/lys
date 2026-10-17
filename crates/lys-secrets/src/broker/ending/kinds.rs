//! Lease-ending kinds retain their existing folded-state codes and labels.
use super::{ENDED_BY, RELINQUISHED_BY};
/// The act a lease was ended by.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndAct {
    /// The person the lease is acted for revoked it.
    Revoke,
    /// Its holder gave it back.
    Relinquish,
}

impl EndAct {
    /// The act as a route reads it.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Revoke => "revoke",
            Self::Relinquish => "relinquish",
        }
    }

    /// How the act's drop line opens.
    pub(super) fn opening(self) -> &'static str {
        match self {
            Self::Revoke => ENDED_BY,
            Self::Relinquish => RELINQUISHED_BY,
        }
    }

    /// Who the act's caller is, as a refusal of its name says.
    pub(super) fn whose(self) -> &'static str {
        match self {
            Self::Revoke => "the person revoking a lease",
            Self::Relinquish => "the holder relinquishing a lease",
        }
    }

    /// The act's code in the folded state.
    pub(in crate::broker) fn code(self) -> u8 {
        match self {
            Self::Revoke => 1,
            Self::Relinquish => 2,
        }
    }

    /// The act a code in the folded state names.
    pub(in crate::broker) fn from_code(code: u8) -> Option<Self> {
        match code {
            1 => Some(Self::Revoke),
            2 => Some(Self::Relinquish),
            _ => None,
        }
    }
}

/// How a lease came to end.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndWay {
    /// The person it is acted for revoked it.
    Revoke,
    /// Its holder relinquished it.
    Relinquish,
    /// Its time window closed.
    Expired,
    /// The operator holding the store dropped it from the command line, or
    /// a handle above it was so dropped: an act of the store's keeper, like
    /// an emergency stop, outside the rule of who may revoke.
    Dropped,
}

impl EndWay {
    /// The way as a route reads it.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Revoke => "revoke",
            Self::Relinquish => "relinquish",
            Self::Expired => "expired",
            Self::Dropped => "dropped",
        }
    }
}

impl From<EndAct> for EndWay {
    fn from(act: EndAct) -> Self {
        match act {
            EndAct::Revoke => Self::Revoke,
            EndAct::Relinquish => Self::Relinquish,
        }
    }
}
