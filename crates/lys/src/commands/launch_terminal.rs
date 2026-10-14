//! Interactive launch terminals preserve bytes and close their server session on every exit.

use serde::Deserialize;
use serde_json::{Value, json};

use crate::commands::error::CliResult;
use crate::commands::seat_client::{Server, refused, segment};

#[cfg(unix)]
#[path = "launch_terminal/unix.rs"]
mod unix;
#[cfg(unix)]
pub use unix::{Terminal, prepare};

#[cfg(unix)]
#[path = "launch_terminal/managed.rs"]
mod managed;

#[cfg(not(unix))]
/// An interactive terminal is unavailable on this platform.
pub struct Terminal(std::convert::Infallible);

#[cfg(not(unix))]
impl Terminal {
    /// An unsupported platform cannot produce a terminal with dimensions.
    #[must_use]
    pub fn size(&self) -> (u16, u16) {
        match self.0 {}
    }
}

#[cfg(not(unix))]
/// Refuse before starting a session when this platform has no terminal bridge.
///
/// # Errors
/// Always refuses with `launch_terminal_unsupported`.
pub fn prepare() -> CliResult<Terminal> {
    Err(refused(
        "launch_terminal_unsupported",
        "interactive Lys launch requires a Unix terminal",
    ))
}

/// Attach this terminal to a launched session, ending the session even on failure.
///
/// # Errors
/// Names malformed runner answers, terminal failures and the server's refusals.
pub fn attach(server: &Server, session: &str, terminal: Terminal) -> CliResult<()> {
    let id = segment("session_id_invalid", "the launched session id", session)?;
    #[cfg(unix)]
    {
        unix::attach(server, id, terminal)
    }
    #[cfg(not(unix))]
    {
        drop(terminal);
        finish(prepare().map(drop), end(server, id), Ok(()))
    }
}

/// Attach canonical terminal lines to an existing managed seat and close its session.
///
/// # Errors
/// Names seat, session, receipt, terminal and server failures without changing controls.
pub fn attach_managed(
    server: &Server,
    seat: &str,
    session: &str,
    terminal: Terminal,
) -> CliResult<()> {
    let id = segment("session_id_invalid", "the launched session id", session)?;
    let name = match segment("seat_name_invalid", "the seat name", seat) {
        Ok(name) => name,
        Err(error) => return finish(Err(error), end(server, id), Ok(())),
    };
    #[cfg(unix)]
    {
        managed::attach(server, name, id, terminal)
    }
    #[cfg(not(unix))]
    {
        drop(terminal);
        finish(prepare().map(drop), end(server, id), Ok(()))
    }
}

/// Each terminal lane holds its own HTTP connection so a follow cannot lock out input.
trait Transport: Send + Sync + 'static {
    fn post(&self, path: &str, body: &Value) -> CliResult<Value>;
}

impl Transport for Server {
    fn post(&self, path: &str, body: &Value) -> CliResult<Value> {
        Self::post(self, path, body)
    }
}

#[derive(Deserialize)]
struct Reply {
    answer: lys_runner::Answer,
}

fn answer(value: Value, verb: &str) -> CliResult<lys_runner::Answer> {
    let reply: Reply = serde_json::from_value(value).map_err(|error| {
        refused(
            "launch_terminal_answer_unreadable",
            format!(
                "the server's {verb} reply is not a runner answer ({:?})",
                error.classify()
            ),
        )
    })?;
    match reply.answer {
        lys_runner::Answer::Refused { refusal, words, .. } => Err(refused(&refusal, words)),
        answer => Ok(answer),
    }
}

fn delivered(value: Value, session: &str, verb: &str) -> CliResult<()> {
    match answer(value, verb)? {
        lys_runner::Answer::Delivered { session: given } if given == session => Ok(()),
        _ => Err(refused(
            "launch_terminal_answer_unexpected",
            format!("{verb} did not confirm delivery to session {session}"),
        )),
    }
}

fn bytes(
    value: Value,
    session: &str,
    cursor: Option<u64>,
) -> CliResult<lys_runner::terminal_bytes::ByteOutput> {
    let lys_runner::Answer::Bytes { output } = answer(value, "read-bytes")? else {
        return Err(refused(
            "launch_terminal_answer_unexpected",
            format!("read-bytes did not answer with bytes for session {session}"),
        ));
    };
    let length = u64::try_from(output.data.len()).map_err(|error| {
        refused(
            "launch_terminal_cursor_invalid",
            format!("the byte window's length cannot be represented: {error}"),
        )
    })?;
    if output.session != session
        || output.from != cursor.unwrap_or(output.oldest)
        || output.from < output.oldest
        || output.cursor.checked_sub(output.from) != Some(length)
    {
        return Err(refused(
            "launch_terminal_cursor_invalid",
            format!("read-bytes did not preserve session {session}'s requested byte window"),
        ));
    }
    if output.data.is_empty() && output.ended.is_none() {
        return Err(refused(
            "launch_terminal_follow_empty",
            format!("session {session}'s follow answered without output or an end"),
        ));
    }
    Ok(output)
}

/// End a session after a launch reply cannot be attached safely.
///
/// # Errors
/// Preserves server refusals and requires an `Ended` answer naming this session.
pub(crate) fn end(server: &Server, session: &str) -> CliResult<()> {
    let id = segment("session_id_invalid", "the launched session id", session)?;
    end_session(server, id)
}

fn end_session(transport: &impl Transport, session: &str) -> CliResult<()> {
    ended_session(transport, session).map(drop)
}

fn ended_session(transport: &impl Transport, session: &str) -> CliResult<lys_runner::Ended> {
    let route = format!("/runtime/sessions/{session}/end");
    match answer(transport.post(&route, &json!({}))?, "end")? {
        lys_runner::Answer::Ended {
            session: given,
            ended,
        } if given == session => Ok(ended),
        _ => Err(refused(
            "launch_terminal_end_unconfirmed",
            format!("the server did not confirm that session {session} ended"),
        )),
    }
}

fn exit(ended: &lys_runner::Ended) -> CliResult<()> {
    if ended.how == lys_runner::EndedHow::Exited && ended.status == Some(0) {
        return Ok(());
    }
    Err(refused(
        "launch_agent_exit_failed",
        format!(
            "the agent ended as {:?}, status {:?}, signal {:?}; inspect it with `lys agents`",
            ended.how, ended.status, ended.signal
        ),
    ))
}

/// Keep every failure named when terminal restoration or session cleanup also fails.
fn finish(outcome: CliResult<()>, ended: CliResult<()>, restored: CliResult<()>) -> CliResult<()> {
    let mut failures = [outcome, ended, restored]
        .into_iter()
        .filter_map(Result::err);
    let Some(first) = failures.next() else {
        return Ok(());
    };
    let additional: Vec<_> = failures.map(|error| error.to_string()).collect();
    if additional.is_empty() {
        return Err(first);
    }
    Err(refused(
        "launch_terminal_cleanup_failed",
        format!("{first}; also: {}", additional.join("; ")),
    ))
}

#[cfg(test)]
#[path = "launch_terminal_tests.rs"]
mod tests;
