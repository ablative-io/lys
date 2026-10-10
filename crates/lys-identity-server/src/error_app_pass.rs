//! An agent's pass to an app (AGENTS-006): the refusals of
//! `POST /agents/{id}/pass`, each keeping its name, its words and its status
//! together, as the seats' refusals do (`error_seat.rs`).

use axum::http::StatusCode;
use lys_identity::LifecycleState;

/// Everything an agent's ask for a pass to an app can refuse by name.
#[derive(Debug, thiserror::Error)]
pub enum AppPassError {
    /// The request carries neither proof that it is the agent's own, or
    /// carries one that does not admit; the reason never holds the proof.
    #[error("agent_pass_unproven: {reason}")]
    Unproven {
        /// Why, in words naming no credential value.
        reason: String,
    },
    /// The agent is retired or suspended, and is issued no pass.
    #[error("agent_retired: agent `{agent}` is {state} and is issued no pass")]
    Retired {
        /// The agent.
        agent: String,
        /// Its recorded lifecycle state.
        state: LifecycleState,
    },
    /// The audience is not an app approved on the Apps screen.
    #[error(
        "audience_not_approved: `{audience}` is not an app approved on the Apps screen: {reason}"
    )]
    AudienceNotApproved {
        /// The audience asked for.
        audience: String,
        /// The apps' own words for why.
        reason: String,
    },
    /// The grant credential presented is not for a live grant the agent holds.
    #[error("agent_not_holder: {reason}")]
    NotHolder {
        /// Why, naming the grant and its holder, never the credential.
        reason: String,
    },
}

impl AppPassError {
    pub(crate) const fn name(&self) -> &'static str {
        match self {
            Self::Unproven { .. } => "agent_pass_unproven",
            Self::Retired { .. } => "agent_retired",
            Self::AudienceNotApproved { .. } => "audience_not_approved",
            Self::NotHolder { .. } => "agent_not_holder",
        }
    }

    pub(crate) const fn status(&self) -> StatusCode {
        match self {
            Self::Unproven { .. } => StatusCode::UNAUTHORIZED,
            Self::AudienceNotApproved { .. } => StatusCode::BAD_REQUEST,
            Self::Retired { .. } | Self::NotHolder { .. } => StatusCode::FORBIDDEN,
        }
    }
}

#[cfg(test)]
mod tests {
    use lys_identity::LifecycleState;

    use super::AppPassError;
    use crate::error::ServerError;

    #[test]
    fn every_app_pass_refusal_keeps_its_name_and_status_through_the_server_error() {
        let cases = [
            (
                AppPassError::Unproven {
                    reason: "no proof".to_owned(),
                },
                "agent_pass_unproven",
                401,
            ),
            (
                AppPassError::Retired {
                    agent: "agent".to_owned(),
                    state: LifecycleState::Retired,
                },
                "agent_retired",
                403,
            ),
            (
                AppPassError::AudienceNotApproved {
                    audience: "notes".to_owned(),
                    reason: "app_not_approved".to_owned(),
                },
                "audience_not_approved",
                400,
            ),
            (
                AppPassError::NotHolder {
                    reason: "another holds it".to_owned(),
                },
                "agent_not_holder",
                403,
            ),
        ];
        for (error, expected, status) in cases {
            let error = ServerError::from(error);
            assert_eq!(error.name(), expected, "{error}");
            assert_eq!(error.status().as_u16(), status, "{error}");
            assert!(error.to_string().starts_with(expected), "{error}");
        }
    }
}
