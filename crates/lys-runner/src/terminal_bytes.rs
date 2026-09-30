//! Byte-preserving terminal reads share the runner's session ownership and cancellation.

use std::sync::atomic::AtomicBool;

use serde::{Deserialize, Serialize};

use crate::{Answer, Ended, RunnerError, Sessions};

/// Raw terminal output. Every byte between `from` and `cursor` is present,
/// including incomplete UTF-8 and control bytes; the emulator owns decoding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ByteOutput {
    /// Owning session.
    pub session: String,
    /// Position of the first byte in data.
    pub from: u64,
    /// Position immediately after data.
    pub cursor: u64,
    /// Oldest position the runner retains.
    pub oldest: u64,
    /// Original PTY bytes, without transcoding.
    pub data: Vec<u8>,
    /// Exit evidence only after the process has ended.
    pub ended: Option<Ended>,
}

pub(crate) fn read(
    sessions: &Sessions,
    id: &str,
    cursor: Option<u64>,
    follow: bool,
    left: &AtomicBool,
) -> Result<Answer, RunnerError> {
    sessions.until(id, left, |session, id| {
        let scrollback = session.scrollback();
        let from = cursor.unwrap_or(scrollback.oldest());
        if from > scrollback.end() {
            return Some(Err(RunnerError::refused(
                "cursor_ahead",
                format!("session {id}: cursor {from} is past {}", scrollback.end()),
            )));
        }
        let data = match scrollback.from(from) {
            Ok(data) => data,
            Err(error) => return Some(Err(error)),
        };
        let ended = session.ended();
        if follow && data.is_empty() && ended.is_none() {
            return None;
        }
        Some(Ok(Answer::Bytes {
            output: ByteOutput {
                session: id.to_owned(),
                from,
                cursor: scrollback.end(),
                oldest: scrollback.oldest(),
                data,
                ended,
            },
        }))
    })
}
