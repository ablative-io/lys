//! The refusals of an agent's own acts on agents (AGENTS-015, AGENTS-016)
//! and on drafts (AGENTS-019), each keeping its name, its words and its
//! status together, as the seats' refusals do (`error_seat.rs`).

use axum::http::StatusCode;

/// What an agent acting on agents is refused by name.
#[derive(Debug, thiserror::Error)]
pub enum AgentActsError {
    /// The one who posts a profile never approves it: an agent reviewing a
    /// version it set itself.
    #[error(
        "provisioning_review_own_version: agent `{agent}` set version {version} of this profile, so someone else reviews it"
    )]
    ReviewOwnVersion {
        /// The agent that set the version and asked to review it.
        agent: String,
        /// The version.
        version: u32,
    },
    /// An agent asked for the first certificate of an agent whose
    /// registration names another as the one that registered it
    /// (AGENTS-016 R4).
    #[error(
        "agent_not_yours: agent `{agent}` was not registered by agent `{caller}`, so its first certificate is not the caller's to issue"
    )]
    AgentNotYours {
        /// The agent the certificate was asked for.
        agent: String,
        /// The agent that asked.
        caller: String,
    },
    /// The agent that registered an agent asked for its first certificate
    /// without a live grant of `agent.create` from that agent's person at
    /// that moment: revoked since, or never held (AGENTS-016 R4).
    #[error(
        "agent_create_revoked: agent `{caller}` holds no live grant of agent.create from person `{person}` now, so it cannot finish the identity of agent `{agent}`; a person issues its certificate"
    )]
    AgentCreateRevoked {
        /// The agent the certificate was asked for.
        agent: String,
        /// The agent that registered it and asked.
        caller: String,
        /// The person both answer to.
        person: String,
    },
    /// The agent that registered an agent asked for a certificate of an
    /// agent that already holds one, issued or withdrawn: every later
    /// certificate is a person's (AGENTS-016 R4).
    #[error(
        "certificate_not_first: agent `{agent}` already holds a certificate, issued or withdrawn, so only a person issues it another"
    )]
    CertificateNotFirst {
        /// The agent the certificate was asked for.
        agent: String,
    },
    /// An agent asked to decide a draft it drafted itself: the one who
    /// prepares an act never approves it (AGENTS-019 R4).
    #[error("own_draft: agent `{agent}` drafted `{draft}`, so someone else decides it")]
    OwnDraft {
        /// The agent that drafted it and asked to decide it.
        agent: String,
        /// The draft.
        draft: String,
    },
}

impl AgentActsError {
    pub(crate) const fn name(&self) -> &'static str {
        match self {
            Self::ReviewOwnVersion { .. } => "provisioning_review_own_version",
            Self::AgentNotYours { .. } => "agent_not_yours",
            Self::AgentCreateRevoked { .. } => "agent_create_revoked",
            Self::CertificateNotFirst { .. } => "certificate_not_first",
            Self::OwnDraft { .. } => "own_draft",
        }
    }

    pub(crate) const fn status(&self) -> StatusCode {
        match self {
            Self::ReviewOwnVersion { .. }
            | Self::AgentNotYours { .. }
            | Self::AgentCreateRevoked { .. }
            | Self::OwnDraft { .. } => StatusCode::FORBIDDEN,
            Self::CertificateNotFirst { .. } => StatusCode::CONFLICT,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::AgentActsError;
    use crate::error::ServerError;

    #[test]
    fn an_own_version_review_keeps_its_name_and_status_through_the_server_error() {
        let error = ServerError::from(AgentActsError::ReviewOwnVersion {
            agent: "agent-1".to_owned(),
            version: 2,
        });
        assert_eq!(error.name(), "provisioning_review_own_version");
        assert_eq!(error.status().as_u16(), 403);
        assert!(error.to_string().contains("version 2"));
    }
}
