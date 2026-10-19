//! The refusals of an agent's own acts on agents (AGENTS-015), each keeping
//! its name, its words and its status together, as the seats' refusals do
//! (`error_seat.rs`).

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
}

impl AgentActsError {
    pub(crate) const fn name(&self) -> &'static str {
        match self {
            Self::ReviewOwnVersion { .. } => "provisioning_review_own_version",
        }
    }

    pub(crate) const fn status(&self) -> StatusCode {
        match self {
            Self::ReviewOwnVersion { .. } => StatusCode::FORBIDDEN,
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
