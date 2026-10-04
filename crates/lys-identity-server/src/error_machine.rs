//! A machine act whose operation id already names a different act: a
//! machine named in other words, another team assignment, another allowance,
//! another connection code. And the connection code's own refusals: no code
//! while this Lys cannot be reached from another computer, and a code that
//! does not connect, which never says why. And a machine's runner dialling
//! in: a dial its key did not sign, and one signed under another start of
//! this server.

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
    /// A dial request was not signed by the machine's key, or names a
    /// machine with no dialled runner.
    #[error("runner_dial_refused: {reason}")]
    DialRefused {
        /// Why.
        reason: String,
    },
    /// A dial was signed under an epoch not this server's: it was made
    /// before the server last started, or captured and sent again after.
    #[error("runner_dial_stale: {reason}")]
    DialStale {
        /// Why.
        reason: String,
    },
}

impl MachineError {
    /// A dial refused for `reason`.
    pub(crate) fn dial_refused(reason: impl Into<String>) -> crate::error::ServerError {
        Self::DialRefused {
            reason: reason.into(),
        }
        .into()
    }

    pub(crate) const fn name(&self) -> &'static str {
        match self {
            Self::Reused { .. } => "MachineReused",
            Self::TeamReused { .. } => "MachineTeamReused",
            Self::AgentsReused { .. } => "MachineAgentsReused",
            Self::JoinOperationReused { .. } => "RunnerJoinOperationReused",
            Self::JoinUnreachable { .. } => "RunnerJoinUnreachable",
            Self::JoinRefused => "RunnerJoinRefused",
            Self::DialRefused { .. } => "runner_dial_refused",
            Self::DialStale { .. } => "runner_dial_stale",
        }
    }

    pub(crate) const fn status(&self) -> StatusCode {
        match self {
            Self::Reused { .. }
            | Self::TeamReused { .. }
            | Self::AgentsReused { .. }
            | Self::JoinOperationReused { .. }
            | Self::JoinUnreachable { .. }
            | Self::DialStale { .. } => StatusCode::CONFLICT,
            Self::JoinRefused => StatusCode::FORBIDDEN,
            Self::DialRefused { .. } => StatusCode::UNAUTHORIZED,
        }
    }
}
