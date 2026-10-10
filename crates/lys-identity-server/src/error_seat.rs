//! The seats' refusals (AGENTS-002): each keeps its name, its words and its
//! status together, as the other refusal families here do.

use axum::http::StatusCode;

/// Everything a seat's record, start, stop, message or attach can refuse.
#[derive(Debug, thiserror::Error)]
pub enum SeatError {
    /// The seats' log could not be read or written.
    #[error("seats_unavailable: {reason}")]
    Unavailable {
        /// Why.
        reason: String,
    },
    /// No seat by that name was ever added.
    #[error("seat_unknown: no seat is named `{name}`")]
    Unknown {
        /// The name asked for.
        name: String,
    },
    /// Another seat already holds the name.
    #[error("seat_name_taken: a seat named `{name}` already exists")]
    NameTaken {
        /// The name.
        name: String,
    },
    /// The name is not 1 to 64 lowercase letters, digits and hyphens.
    #[error(
        "seat_name_invalid: `{name}` is not a seat name: use 1 to 64 lowercase letters, digits and hyphens"
    )]
    NameInvalid {
        /// The name given.
        name: String,
    },
    /// The operation id already names another seat act, in other words.
    #[error(
        "seat_operation_reused: operation `{operation}` already names another seat act: send this one under a new operation id"
    )]
    OperationReused {
        /// The operation id.
        operation: String,
    },
    /// The seat's profile does not require controls, so it would not start managed.
    #[error(
        "seat_not_managed: profile version {version} of agent `{agent}` does not require controls, so the seat would not start managed: record a version whose session requires controls"
    )]
    NotManaged {
        /// The agent.
        agent: String,
        /// The profile version.
        version: u32,
    },
    /// The seat's agent has no responsible person, so nobody answers for its runs.
    #[error("seat_no_responsible: seat `{name}` runs agent `{agent}`, for whom no person answers")]
    NoResponsible {
        /// The seat.
        name: String,
        /// The agent.
        agent: String,
    },
    /// The seat's session is already running.
    #[error("seat_running: seat `{name}` is running session `{session}`: stop it first")]
    Running {
        /// The seat.
        name: String,
        /// Its running session.
        session: String,
    },
    /// The seat has no running session.
    #[error("seat_not_running: seat `{name}` has no running session")]
    NotRunning {
        /// The seat.
        name: String,
    },
    /// The harness reports a turn in progress, and force was not named.
    #[error(
        "seat_turn_in_progress: seat `{name}` is in the middle of a turn in session `{session}`: wait for it to finish, or name force"
    )]
    TurnInProgress {
        /// The seat.
        name: String,
        /// Its session.
        session: String,
    },
    /// The monitor's registry still lists the seat's name online.
    #[error(
        "seat_online_in_monitor: the monitor at {monitor} lists seat `{name}` online: stop it there and turn its delivery off before starting it from Lys"
    )]
    OnlineInMonitor {
        /// The seat.
        name: String,
        /// The monitor base asked.
        monitor: String,
    },
    /// Another live machine already names this install's own runner.
    #[error(
        "runner_lys_taken: machine `{machine}` already names this install's own runner; an install holds one such machine"
    )]
    RunnerLysTaken {
        /// The machine that holds it.
        machine: String,
    },
    /// The caller is neither the seat's responsible person nor an administrator.
    #[error(
        "seat_attach_refused: only the person responsible for seat `{name}` or an administrator may attach to it"
    )]
    AttachRefused {
        /// The seat.
        name: String,
    },
    /// The message is empty.
    #[error("seat_text_empty: a message to seat `{name}` says nothing")]
    TextEmpty {
        /// The seat.
        name: String,
    },
    /// The message carries a control character.
    #[error(
        "seat_text_control: a message to seat `{name}` carries the control character U+{code:04X}; only line breaks and tabs may be sent"
    )]
    TextControl {
        /// The seat.
        name: String,
        /// The character's code point.
        code: u32,
    },
}

impl SeatError {
    pub(crate) const fn name(&self) -> &'static str {
        match self {
            Self::Unavailable { .. } => "seats_unavailable",
            Self::Unknown { .. } => "seat_unknown",
            Self::NameTaken { .. } => "seat_name_taken",
            Self::NameInvalid { .. } => "seat_name_invalid",
            Self::OperationReused { .. } => "seat_operation_reused",
            Self::NotManaged { .. } => "seat_not_managed",
            Self::NoResponsible { .. } => "seat_no_responsible",
            Self::Running { .. } => "seat_running",
            Self::NotRunning { .. } => "seat_not_running",
            Self::TurnInProgress { .. } => "seat_turn_in_progress",
            Self::OnlineInMonitor { .. } => "seat_online_in_monitor",
            Self::RunnerLysTaken { .. } => "runner_lys_taken",
            Self::AttachRefused { .. } => "seat_attach_refused",
            Self::TextEmpty { .. } => "seat_text_empty",
            Self::TextControl { .. } => "seat_text_control",
        }
    }

    pub(crate) const fn status(&self) -> StatusCode {
        match self {
            Self::Unavailable { .. } => StatusCode::SERVICE_UNAVAILABLE,
            Self::Unknown { .. } => StatusCode::NOT_FOUND,
            Self::NameInvalid { .. } | Self::TextEmpty { .. } | Self::TextControl { .. } => {
                StatusCode::BAD_REQUEST
            }
            Self::AttachRefused { .. } => StatusCode::FORBIDDEN,
            Self::NameTaken { .. }
            | Self::OperationReused { .. }
            | Self::NotManaged { .. }
            | Self::NoResponsible { .. }
            | Self::Running { .. }
            | Self::NotRunning { .. }
            | Self::TurnInProgress { .. }
            | Self::OnlineInMonitor { .. }
            | Self::RunnerLysTaken { .. } => StatusCode::CONFLICT,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SeatError;
    use crate::error::ServerError;

    #[test]
    fn every_seat_refusal_keeps_its_name_through_the_server_error() {
        let name = || "waffles".to_owned();
        let cases = [
            (
                SeatError::Unavailable {
                    reason: "disk".to_owned(),
                },
                "seats_unavailable",
            ),
            (SeatError::Unknown { name: name() }, "seat_unknown"),
            (SeatError::NameTaken { name: name() }, "seat_name_taken"),
            (SeatError::NameInvalid { name: name() }, "seat_name_invalid"),
            (
                SeatError::OperationReused {
                    operation: "op".to_owned(),
                },
                "seat_operation_reused",
            ),
            (
                SeatError::NotManaged {
                    agent: "agent".to_owned(),
                    version: 1,
                },
                "seat_not_managed",
            ),
            (
                SeatError::NoResponsible {
                    name: name(),
                    agent: "agent".to_owned(),
                },
                "seat_no_responsible",
            ),
            (
                SeatError::Running {
                    name: name(),
                    session: "op".to_owned(),
                },
                "seat_running",
            ),
            (SeatError::NotRunning { name: name() }, "seat_not_running"),
            (
                SeatError::TurnInProgress {
                    name: name(),
                    session: "op".to_owned(),
                },
                "seat_turn_in_progress",
            ),
            (
                SeatError::OnlineInMonitor {
                    name: name(),
                    monitor: "http://monitor".to_owned(),
                },
                "seat_online_in_monitor",
            ),
            (
                SeatError::RunnerLysTaken {
                    machine: "op-1".to_owned(),
                },
                "runner_lys_taken",
            ),
            (SeatError::AttachRefused { name: name() }, "seat_attach_refused"),
            (SeatError::TextEmpty { name: name() }, "seat_text_empty"),
            (
                SeatError::TextControl {
                    name: name(),
                    code: 0x1b,
                },
                "seat_text_control",
            ),
        ];
        for (error, expected) in cases {
            let error = ServerError::from(error);
            assert_eq!(error.name(), expected, "{error}");
            assert!(error.to_string().starts_with(expected), "{error}");
        }
    }
}
