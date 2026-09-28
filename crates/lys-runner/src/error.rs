//! The runner's refusals, each by name. The name is the first word of the
//! message, before its colon, and is what a caller keys on; the words after
//! it are for a person. No refusal carries a key, an account value or the
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
    /// The refusal's name: the first word of its message.
    pub fn name(&self) -> String {
        let text = self.to_string();
        text.split(':').next().unwrap_or_default().to_owned()
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

static SINK: std::sync::Mutex<Option<Sink>> = std::sync::Mutex::new(None);

/// Send every line the runner says to `sink` as well as to its standard
/// error, replacing any sink given before: how a process that holds a
/// runner keeps its log, and how a test reads every line it said.
pub fn also_to(sink: Sink) {
    *SINK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(sink);
}

/// Say `line` on the runner's standard error, which the install keeps as
/// the runner's log, and to the sink [`also_to`] gave: what a thread with
/// no caller to answer saw, by name. A line names a session, a process or
/// an account's handle; never an account's value or the text a session was
/// given.
pub fn said(line: &str) {
    let line = format!("lys-runner {line}");
    eprintln!("{line}");
    if let Some(sink) = SINK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .as_ref()
    {
        sink(&line);
    }
}
