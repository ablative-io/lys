//! The runner's refusals, each by a stable name held in its error shape.
//! Callers key on that name; its explanatory words are for a person.
//! No refusal carries a key, an account value or the
//! text a session was given.

/// Every way the runner, its client or its dial bridge refuses.
#[derive(Debug, thiserror::Error)]
pub enum RunnerError {
    /// The request carried no signature, or one the server's key does not verify.
    #[error("runner_request_unsigned: {reason}")]
    Unsigned {
        /// Why the signature was refused.
        reason: String,
    },
    /// The request is not one line of the protocol's JSON.
    #[error("runner_request_malformed: {reason}")]
    Malformed {
        /// What did not read.
        reason: String,
    },
    /// The two sides speak different protocol versions.
    #[error(
        "runner_protocol_mismatch: the other side speaks runner protocol {theirs}, this side speaks {ours}"
    )]
    ProtocolMismatch {
        /// The version the other side answered or asked in.
        theirs: u32,
        /// The version this side speaks.
        ours: u32,
    },
    /// The request names a challenge its connection was not given: it was
    /// made for another connection, and is answered on none but that one.
    #[error("runner_request_replayed: {reason}")]
    Replayed {
        /// Why.
        reason: String,
    },
    /// The request names another runner than the one it reached.
    #[error("runner_request_misaddressed: {reason}")]
    Misaddressed {
        /// Which runner it names, and which it reached.
        reason: String,
    },
    /// The runner could not be reached, or it closed before it answered.
    #[error("runner_unreachable: {reason}")]
    Unreachable {
        /// What failed.
        reason: String,
    },
    /// The runner's answer is not one line of the protocol's JSON.
    #[error("runner_reply_malformed: {reason}")]
    ReplyMalformed {
        /// What did not read.
        reason: String,
    },
    /// The runner refused the act, by its own name.
    #[error("{refusal}: {words}")]
    Refused {
        /// The refusal's name, as the runner gave it.
        refusal: String,
        /// Why, in the runner's words.
        words: String,
        /// For `cursor_expired`, the oldest cursor the runner holds.
        oldest: Option<u64>,
    },
    /// The runner's record of its sessions could not be read or written.
    #[error("runner_state_unavailable: {reason}")]
    State {
        /// What failed.
        reason: String,
    },
    /// Another runner holds the state directory.
    #[error("runner_state_held: another runner holds {state}")]
    StateHeld {
        /// The directory.
        state: String,
    },
    /// Another runner answers on the socket already.
    #[error("runner_already_running: a runner already answers on {socket}")]
    AlreadyRunning {
        /// The socket's path.
        socket: String,
    },
    /// The socket could not be made or kept.
    #[error("runner_socket_unavailable: {reason}")]
    Socket {
        /// What failed.
        reason: String,
    },
    /// A key could not be read, or does not read as a key.
    #[error("runner_key_unavailable: {reason}")]
    Key {
        /// What failed.
        reason: String,
    },
    /// The server refused a dial signed under an epoch not its own: it has
    /// started again since the bridge read its epoch.
    #[error("runner_dial_stale: {reason}")]
    DialStale {
        /// The server's words.
        reason: String,
    },
    /// The dial bridge could not reach the server or was refused by it.
    #[error("runner_dial_failed: {reason}")]
    Dial {
        /// What failed.
        reason: String,
    },
}

impl RunnerError {
    /// The stable refusal name, independent of its explanatory words.
    pub fn name(&self) -> String {
        match self {
            Self::Unsigned { .. } => "runner_request_unsigned",
            Self::Malformed { .. } => "runner_request_malformed",
            Self::ProtocolMismatch { .. } => "runner_protocol_mismatch",
            Self::Replayed { .. } => "runner_request_replayed",
            Self::Misaddressed { .. } => "runner_request_misaddressed",
            Self::Unreachable { .. } => "runner_unreachable",
            Self::ReplyMalformed { .. } => "runner_reply_malformed",
            Self::Refused { refusal, .. } => return refusal.clone(),
            Self::State { .. } => "runner_state_unavailable",
            Self::StateHeld { .. } => "runner_state_held",
            Self::AlreadyRunning { .. } => "runner_already_running",
            Self::Socket { .. } => "runner_socket_unavailable",
            Self::Key { .. } => "runner_key_unavailable",
            Self::DialStale { .. } => "runner_dial_stale",
            Self::Dial { .. } => "runner_dial_failed",
        }
        .to_owned()
    }

    /// The refusal `refusal`, in `words`.
    pub fn refused(refusal: &str, words: impl Into<String>) -> Self {
        Self::Refused {
            refusal: refusal.to_owned(),
            words: words.into(),
            oldest: None,
        }
    }
}

/// Where the runner's log lines go beside its standard error.
pub type Sink = Box<dyn Fn(&str) + Send + Sync>;

type HeldSink = Option<std::sync::Arc<Sink>>;

static SINK: std::sync::Mutex<HeldSink> = std::sync::Mutex::new(None);

/// Send every line the runner says to `sink` as well as to its standard
/// error, replacing any sink given before: how a process that holds a
/// runner keeps its log, and how a test reads every line it said.
///
/// # Errors
/// Returns `runner_log_unavailable` if the sink lock is poisoned.
pub fn also_to(sink: Sink) -> Result<(), RunnerError> {
    install_sink(&SINK, sink)
}

fn install_sink(slot: &std::sync::Mutex<HeldSink>, sink: Sink) -> Result<(), RunnerError> {
    let mut held = slot.lock().map_err(|error| {
        RunnerError::refused(
            "runner_log_unavailable",
            format!("log sink lock poisoned: {error}"),
        )
    })?;
    *held = Some(std::sync::Arc::new(sink));
    Ok(())
}

fn read_sink(slot: &std::sync::Mutex<HeldSink>) -> Result<HeldSink, RunnerError> {
    let held = slot.lock().map_err(|error| {
        RunnerError::refused(
            "runner_log_unavailable",
            format!("log sink lock poisoned: {error}"),
        )
    })?;
    Ok(held.as_ref().map(std::sync::Arc::clone))
}

/// Say `line` on the runner's standard error, which the install keeps as
/// the runner's log, and to the sink [`also_to`] gave: what a thread with
/// no caller to answer saw, by name. A line names a session, a process or
/// an account's handle; never an account's value or the text a session was
/// given.
pub fn said(line: &str) {
    let line = format!("lys-runner {line}");
    eprintln!("{line}");
    match read_sink(&SINK) {
        Ok(Some(sink)) => sink(&line),
        Ok(None) => {}
        Err(error) => eprintln!("lys-runner {error}"),
    }
}

#[cfg(test)]
mod poison_tests {
    use super::{HeldSink, install_sink, read_sink};

    #[test]
    fn a_poisoned_log_sink_is_named_and_never_called_or_replaced() {
        let slot = std::sync::Mutex::new(HeldSink::None);
        let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut held = match slot.lock() {
                Ok(held) => held,
                Err(error) => panic!("fixture_lock_poisoned: {error}"),
            };
            *held = Some(std::sync::Arc::new(Box::new(|_| {
                panic!("poisoned sink was called");
            })));
            panic!("injected partial sink update");
        }));
        assert!(panic.is_err());
        match read_sink(&slot) {
            Err(error) => assert_eq!(error.name(), "runner_log_unavailable"),
            Ok(_) => panic!("poisoned sink was exposed"),
        }
        match install_sink(&slot, Box::new(|_| {})) {
            Err(error) => assert_eq!(error.name(), "runner_log_unavailable"),
            Ok(()) => panic!("poisoned sink was replaced without reopening"),
        }
    }
}

#[cfg(test)]
mod name_tests {
    use super::RunnerError;

    #[test]
    fn refusal_names_come_from_the_error_shape_and_preserve_given_names() {
        let reason = || "detail: another detail".to_owned();
        let cases = [
            (
                RunnerError::Unsigned { reason: reason() },
                "runner_request_unsigned",
            ),
            (
                RunnerError::Malformed { reason: reason() },
                "runner_request_malformed",
            ),
            (
                RunnerError::ProtocolMismatch { theirs: 2, ours: 1 },
                "runner_protocol_mismatch",
            ),
            (
                RunnerError::Replayed { reason: reason() },
                "runner_request_replayed",
            ),
            (
                RunnerError::Misaddressed { reason: reason() },
                "runner_request_misaddressed",
            ),
            (
                RunnerError::Unreachable { reason: reason() },
                "runner_unreachable",
            ),
            (
                RunnerError::ReplyMalformed { reason: reason() },
                "runner_reply_malformed",
            ),
            (
                RunnerError::refused("authority:qualified", reason()),
                "authority:qualified",
            ),
            (
                RunnerError::State { reason: reason() },
                "runner_state_unavailable",
            ),
            (
                RunnerError::StateHeld { state: reason() },
                "runner_state_held",
            ),
            (
                RunnerError::AlreadyRunning { socket: reason() },
                "runner_already_running",
            ),
            (
                RunnerError::Socket { reason: reason() },
                "runner_socket_unavailable",
            ),
            (
                RunnerError::Key { reason: reason() },
                "runner_key_unavailable",
            ),
            (
                RunnerError::DialStale { reason: reason() },
                "runner_dial_stale",
            ),
            (RunnerError::Dial { reason: reason() }, "runner_dial_failed"),
        ];
        let mut names = std::collections::BTreeSet::new();
        for (error, expected) in cases {
            assert_eq!(error.name(), expected);
            assert!(names.insert(error.name()), "a refusal name is duplicated");
        }
    }
}
