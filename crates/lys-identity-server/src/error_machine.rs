//! A machine act whose operation id already names a different act: a
//! machine named in other words, another team assignment, another allowance,
//! another connection code. And the connection code's own refusals: no code
//! while this Lys cannot be reached from another computer, and a code that
//! does not connect, which never says why.

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
    /// The operation already gave a connection code, which is never shown
    /// again.
    #[error(
        "RunnerJoinOperationReused: operation `{operation}` already gave a connection code, and Lys does not show a code twice; ask for a new code with a new operation"
    )]
    JoinOperationReused {
        /// The reused operation.
        operation: String,
    },
    /// This Lys is served where another computer cannot reach it, so no
    /// connection code is given.
    #[error("RunnerJoinUnreachable: {reason}")]
    JoinUnreachable {
        /// Where this Lys is served, and why another computer cannot reach it.
        reason: String,
    },
    /// A connection code that is wrong, already used or replaced by a newer
    /// one: which of these is never said.
    #[error(
        "RunnerJoinRefused: this connection code does not connect this computer; ask the administrator for a new code"
    )]
    JoinRefused,
}

impl MachineError {
    pub(crate) const fn name(&self) -> &'static str {
        match self {
            Self::Reused { .. } => "MachineReused",
            Self::TeamReused { .. } => "MachineTeamReused",
            Self::AgentsReused { .. } => "MachineAgentsReused",
            Self::JoinOperationReused { .. } => "RunnerJoinOperationReused",
            Self::JoinUnreachable { .. } => "RunnerJoinUnreachable",
            Self::JoinRefused => "RunnerJoinRefused",
        }
    }

    pub(crate) const fn status(&self) -> StatusCode {
        match self {
            Self::Reused { .. }
            | Self::TeamReused { .. }
            | Self::AgentsReused { .. }
            | Self::JoinOperationReused { .. }
            | Self::JoinUnreachable { .. } => StatusCode::CONFLICT,
            Self::JoinRefused => StatusCode::FORBIDDEN,
        }
    }
}
