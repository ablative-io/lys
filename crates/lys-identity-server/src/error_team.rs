//! Team refusals keep their response words and status together.

use axum::http::StatusCode;

/// Everything a team change or read can refuse.
#[derive(Debug, thiserror::Error)]
pub enum TeamError {
    /// The teams are not configured, or their log could not be read or written.
    #[error("TeamsUnavailable: {reason}")]
    Unavailable {
        /// Why.
        reason: String,
    },
    /// No team by that id was ever created.
    #[error("TeamUnknown: no team by that id was ever created")]
    Unknown,
    /// The operation id already names a team act sent in other words.
    #[error(
        "TeamReused: operation `{operation}` already names a team act in other words: send this act under a new operation id"
    )]
    Reused {
        /// The operation id.
        operation: String,
    },
    /// The team is retired and takes no more changes.
    #[error("TeamRetired: team `{team}` is retired and takes no more changes")]
    Retired {
        /// The team.
        team: String,
    },
    /// The member is already in the team.
    #[error("TeamMemberHeld: that member is already in the team")]
    MemberHeld,
    /// The member is not in the team.
    #[error("TeamMemberAbsent: that member is not in the team")]
    MemberAbsent,
    /// The parent would close a team cycle.
    #[error(
        "team_parent_cycle: team `{team}` cannot have parent `{parent}` because that closes a cycle"
    )]
    ParentCycle {
        /// The team whose parent would change.
        team: String,
        /// The requested parent that would close a cycle.
        parent: String,
    },
    /// A lead must remain an admitted member.
    #[error("team_lead_not_member: agent `{lead}` is not an admitted member of team `{team}`")]
    LeadNotMember {
        /// The team requiring an admitted lead.
        team: String,
        /// The requested lead that is not an admitted member.
        lead: String,
    },
    /// The member named is not a person or agent the directory holds, or is retired.
    #[error(
        "TeamMemberUnknown: a member is a person or agent the directory holds and has not retired"
    )]
    MemberUnknown,
}

impl TeamError {
    pub(crate) fn status(&self) -> StatusCode {
        match self {
            Self::Unavailable { .. } => StatusCode::SERVICE_UNAVAILABLE,
            Self::Unknown | Self::MemberUnknown => StatusCode::NOT_FOUND,
            Self::Reused { .. }
            | Self::Retired { .. }
            | Self::MemberHeld
            | Self::MemberAbsent
            | Self::ParentCycle { .. }
            | Self::LeadNotMember { .. } => StatusCode::CONFLICT,
        }
    }
}
