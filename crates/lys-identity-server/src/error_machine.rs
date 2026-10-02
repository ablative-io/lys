//! A machine act whose operation id already names a different act: a
//! machine named in other words, another team assignment, another allowance.

use axum::http::StatusCode;

/// A machine act's operation id is already spent on a different act.
#[derive(Debug, thiserror::Error)]
pub enum MachineError {
    /// The operation id already names a machine named in other words.
    #[error("MachineReused: operation `{machine}` already names a machine named in other words")]
    Reused {
        /// The operation id.
        machine: String,
    },
    /// The operation already names an ownership act in different words.
    #[error(
        "MachineTeamReused: operation `{operation}` already assigned this computer to a different team; a new assignment needs a new operation"
    )]
    TeamReused {
        /// The reused operation.
        operation: String,
    },
    /// The operation already names an agent allowance act in different words.
    #[error(
        "MachineAgentsReused: operation `{operation}` already names another computer allowance; a new allowance needs a new operation"
    )]
    AgentsReused {
        /// The reused operation.
        operation: String,
    },
}

impl MachineError {
    pub(crate) const fn name(&self) -> &'static str {
        match self {
            Self::Reused { .. } => "MachineReused",
            Self::TeamReused { .. } => "MachineTeamReused",
            Self::AgentsReused { .. } => "MachineAgentsReused",
        }
    }

    pub(crate) const fn status(&self) -> StatusCode {
        match self {
            Self::Reused { .. } | Self::TeamReused { .. } | Self::AgentsReused { .. } => {
                StatusCode::CONFLICT
            }
        }
    }
}
