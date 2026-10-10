//! Attach: a managed session's live frames, rendered as lines of text a
//! person reads in their terminal.
//!
//! Every frame the managed channel's single reader takes is rendered as it
//! arrives: the person's turns, the assistant's text, each tool call and its
//! result, and the harness's status. The lines are kept in a ring of their
//! own beside the session's output, bounded by the same scrollback size in
//! bytes of text, each at a cursor that never changes: the count of lines
//! the session rendered before it. A read from a cursor older than the
//! oldest kept is refused `cursor_expired`, naming the oldest.
//!
//! A read that follows waits on the session table until a line is rendered
//! after its cursor, the session ends, or the caller leaves; nothing ends
//! one on a clock. Reading never writes to the session: attach is read-only,
//! and a person's typed line reaches a session only through its input.
//!
//! A session started in a pseudo-terminal has no frames: its attach is a
//! follow of its terminal bytes (`read_bytes`), and asking for its lines is
//! refused `attach_pty_use_read_bytes`.

use std::collections::VecDeque;
use std::sync::atomic::AtomicBool;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::RunnerError;
use crate::harness_control::Transport;
use crate::protocol::Answer;
use crate::session::{Sessions, now_ms, unknown};

/// A turn the person, or the runner for them, gave the harness.
pub const USER: &str = "user";
/// Text the assistant wrote.
pub const ASSISTANT: &str = "assistant";
/// A tool the assistant called, with its input.
pub const TOOL_CALL: &str = "tool_call";
/// What a tool call gave back.
pub const TOOL_RESULT: &str = "tool_result";
/// Where the harness stands: ready, a turn begun or ended, compacted.
pub const STATUS: &str = "status";
/// Anything else the harness said on its channel.
pub const SYSTEM: &str = "system";

/// One rendered line of a managed session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct AttachLine {
    /// When its frame was read, in milliseconds since the Unix epoch.
    pub at: u64,
    /// What it is: `user`, `assistant`, `tool_call`, `tool_result`,
    /// `status` or `system`.
    pub kind: String,
    /// Its text, which may run over several lines. A line longer than the
    /// session's whole scrollback is cut to it at a character boundary.
    pub text: String,
}

/// The rendered lines a session keeps, oldest first.
#[derive(Debug, Clone)]
pub struct Ring {
    lines: VecDeque<AttachLine>,
    oldest: u64,
    kept: usize,
    limit: usize,
}

/// Lines read from a ring, and the cursor to read on from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lines {
    /// The lines from the cursor asked for to the end.
    pub lines: Vec<AttachLine>,
    /// The cursor after the last of them.
    pub cursor: u64,
}

impl Ring {
    /// An empty ring keeping at most `limit` bytes of text.
    pub fn new(limit: usize) -> Self {
        Self {
            lines: VecDeque::new(),
            oldest: 0,
            kept: 0,
            limit,
        }
    }

    /// The cursor of the oldest line kept.
    pub fn oldest(&self) -> u64 {
        self.oldest
    }

    /// The cursor after the last line rendered.
    pub fn end(&self) -> u64 {
        self.oldest + self.lines.len() as u64
    }

    /// Keep `line`, letting the oldest lines go while the text kept is over
    /// the limit.
    pub fn push(&mut self, mut line: AttachLine) {
        if line.text.len() > self.limit {
            let mut cut = self.limit;
            while !line.text.is_char_boundary(cut) {
                cut -= 1;
            }
            line.text.truncate(cut);
        }
        self.kept += line.text.len();
        self.lines.push_back(line);
        while self.kept > self.limit {
            let Some(gone) = self.lines.pop_front() else {
                break;
            };
            self.kept -= gone.text.len();
            self.oldest += 1;
        }
    }

    /// The lines from `cursor`, the oldest kept when none is given, refused
    /// `cursor_expired` when it is older than the oldest kept and
    /// `cursor_ahead` when it is past the end.
    pub fn from(&self, cursor: Option<u64>) -> Result<Lines, RunnerError> {
        let from = cursor.unwrap_or(self.oldest);
        if from < self.oldest {
            return Err(RunnerError::Refused {
                refusal: "cursor_expired".to_owned(),
                words: format!(
                    "cursor {from} is older than the lines kept; the oldest cursor held is {}",
                    self.oldest
                ),
                oldest: Some(self.oldest),
            });
        }
        if from > self.end() {
            return Err(RunnerError::refused(
                "cursor_ahead",
                format!(
                    "cursor {from} is past the last line rendered, {}",
                    self.end()
                ),
            ));
        }
        let skip = usize::try_from(from - self.oldest)
            .map_err(|error| RunnerError::refused("cursor_invalid", error.to_string()))?;
        Ok(Lines {
            lines: self.lines.iter().skip(skip).cloned().collect(),
            cursor: self.end(),
        })
    }
}

fn line(kind: &str, text: impl Into<String>, at: u64) -> AttachLine {
    AttachLine {
        at,
        kind: kind.to_owned(),
        text: text.into(),
    }
}

fn field<'a>(value: &'a Value, pointer: &str) -> Option<&'a str> {
    value.pointer(pointer).and_then(Value::as_str)
}

/// The text a content value carries: a string as it is, the text of each
/// text part of an array joined by newlines, anything else as its JSON.
fn text_of(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Array(parts) => parts
            .iter()
            .map(|part| match part.get("text").and_then(Value::as_str) {
                Some(text) => text.to_owned(),
                None => format!(
                    "[{}]",
                    part.get("type").and_then(Value::as_str).unwrap_or("part")
                ),
            })
            .collect::<Vec<_>>()
            .join("\n"),
        other => other.to_string(),
    }
}

/// The lines `frame` renders as, read at `at`, on a channel of `transport`.
/// A frame that is only part of a message still being written, or the
/// assistant's private reasoning, renders as no line.
pub fn render(transport: Transport, frame: &Value, at: u64) -> Vec<AttachLine> {
    match transport {
        Transport::Claude => claude(frame, at),
        Transport::Codex => codex(frame, at),
        Transport::Pty => Vec::new(),
    }
}

fn claude(frame: &Value, at: u64) -> Vec<AttachLine> {
    let kind = frame.get("type").and_then(Value::as_str).unwrap_or("frame");
    match kind {
        "user" => match frame.pointer("/message/content") {
            Some(Value::Array(parts)) => parts
                .iter()
                .filter_map(|part| match part.get("type").and_then(Value::as_str) {
                    Some("tool_result") => {
                        let text = part.get("content").map(text_of).unwrap_or_default();
                        let failed = part.get("is_error").and_then(Value::as_bool) == Some(true);
                        Some(line(
                            TOOL_RESULT,
                            if failed {
                                format!("error: {text}")
                            } else {
                                text
                            },
                            at,
                        ))
                    }
                    Some("text") => part
                        .get("text")
                        .and_then(Value::as_str)
                        .map(|text| line(USER, text, at)),
                    other => Some(line(USER, format!("[{}]", other.unwrap_or("part")), at)),
                })
                .collect(),
            Some(content) => vec![line(USER, text_of(content), at)],
            None => Vec::new(),
        },
        "assistant" => frame
            .pointer("/message/content")
            .and_then(Value::as_array)
            .map(|parts| {
                parts
                    .iter()
                    .filter_map(|part| match part.get("type").and_then(Value::as_str) {
                        Some("text") => part
                            .get("text")
                            .and_then(Value::as_str)
                            .map(|text| line(ASSISTANT, text, at)),
                        Some("tool_use") => Some(line(
                            TOOL_CALL,
                            format!(
                                "{} {}",
                                part.get("name").and_then(Value::as_str).unwrap_or("tool"),
                                part.get("input").map(Value::to_string).unwrap_or_default()
                            ),
                            at,
                        )),
                        _ => None,
                    })
                    .collect()
            })
            .unwrap_or_default(),
        "system" => match field(frame, "/subtype") {
            Some("init") => vec![line(
                STATUS,
                format!(
                    "session {} ready, model {}",
                    field(frame, "/session_id").unwrap_or("unnamed"),
                    field(frame, "/model").unwrap_or("unnamed")
                ),
                at,
            )],
            Some("compact_boundary") => vec![line(
                STATUS,
                format!(
                    "compacted ({})",
                    field(frame, "/compact_metadata/trigger").unwrap_or("unnamed")
                ),
                at,
            )],
            other => vec![line(SYSTEM, other.unwrap_or("system").to_owned(), at)],
        },
        "result" => {
            let failed = frame.get("is_error").and_then(Value::as_bool) == Some(true);
            vec![line(
                STATUS,
                format!(
                    "turn ended: {}{}",
                    field(frame, "/subtype").unwrap_or("result"),
                    if failed { " (error)" } else { "" }
                ),
                at,
            )]
        }
        "stream_event" => Vec::new(),
        other => vec![line(
            SYSTEM,
            format!(
                "{other} {}",
                field(frame, "/request/subtype")
                    .or_else(|| field(frame, "/response/subtype"))
                    .unwrap_or("")
            )
            .trim_end()
            .to_owned(),
            at,
        )],
    }
}

fn codex(frame: &Value, at: u64) -> Vec<AttachLine> {
    let Some(method) = frame.get("method").and_then(Value::as_str) else {
        // An answer to a request the runner made: the harness's own words
        // are in the notifications that follow it.
        return Vec::new();
    };
    if frame.get("id").is_some() {
        return vec![line(SYSTEM, format!("asks {method}"), at)];
    }
    match method {
        "item/completed" => frame
            .pointer("/params/item")
            .map(|item| codex_item(item, at))
            .unwrap_or_default(),
        "turn/started" => vec![line(STATUS, "turn started", at)],
        "turn/completed" => vec![line(
            STATUS,
            format!(
                "turn ended: {}",
                field(frame, "/params/turn/status").unwrap_or("completed")
            ),
            at,
        )],
        "thread/started" => vec![line(
            STATUS,
            format!(
                "thread {} ready",
                field(frame, "/params/thread/id").unwrap_or("unnamed")
            ),
            at,
        )],
        "error" => vec![line(
            SYSTEM,
            format!(
                "error: {}",
                field(frame, "/params/error/message").unwrap_or("unnamed")
            ),
            at,
        )],
        _ => Vec::new(),
    }
}

fn codex_item(item: &Value, at: u64) -> Vec<AttachLine> {
    match item.get("type").and_then(Value::as_str) {
        Some("userMessage") => vec![line(
            USER,
            item.get("content").map(text_of).unwrap_or_default(),
            at,
        )],
        Some("agentMessage") => vec![line(
            ASSISTANT,
            field(item, "/text").unwrap_or("").to_owned(),
            at,
        )],
        Some("reasoning") => Vec::new(),
        Some("commandExecution") => {
            let mut lines = vec![line(
                TOOL_CALL,
                format!("command {}", field(item, "/command").unwrap_or("")),
                at,
            )];
            if let Some(output) = field(item, "/aggregatedOutput") {
                lines.push(line(TOOL_RESULT, output, at));
            }
            lines
        }
        Some("mcpToolCall") => {
            let mut lines = vec![line(
                TOOL_CALL,
                format!(
                    "{}.{} {}",
                    field(item, "/server").unwrap_or("server"),
                    field(item, "/tool").unwrap_or("tool"),
                    item.get("arguments")
                        .map(Value::to_string)
                        .unwrap_or_default()
                ),
                at,
            )];
            if let Some(result) = item.get("result").filter(|result| !result.is_null()) {
                lines.push(line(TOOL_RESULT, text_of(result), at));
            }
            lines
        }
        Some("fileChange") => vec![line(
            TOOL_CALL,
            format!(
                "file change {}",
                item.get("changes")
                    .and_then(Value::as_array)
                    .map(|changes| {
                        changes
                            .iter()
                            .filter_map(|change| change.get("path").and_then(Value::as_str))
                            .collect::<Vec<_>>()
                            .join(", ")
                    })
                    .unwrap_or_default()
            ),
            at,
        )],
        other => vec![line(SYSTEM, other.unwrap_or("item").to_owned(), at)],
    }
}

impl Sessions {
    /// Render `frame`, read from session `id`'s managed channel in process
    /// generation `generation`, into its attach lines, and record it as the
    /// session's last signal. Whatever follows its lines is woken.
    pub(crate) fn attach_frame(
        &self,
        id: &str,
        generation: u64,
        transport: Transport,
        frame: &Value,
    ) -> Result<(), RunnerError> {
        let at = now_ms();
        let lines = render(transport, frame, at);
        let mut table = self.lock()?;
        let Some(session) = table
            .sessions
            .get_mut(id)
            .filter(|session| session.generation == generation)
        else {
            // A frame of a generation no longer held is the managed
            // reader's to refuse, on its next look at the table.
            return Ok(());
        };
        session.last_signal = Some(at);
        let rendered = !lines.is_empty();
        for each in lines {
            session.attach.push(each);
        }
        drop(table);
        if rendered {
            self.wake();
        }
        Ok(())
    }

    /// Session `id`'s rendered lines from `cursor`, the oldest kept when none
    /// is given. With `follow`, the answer waits until a line is rendered
    /// after the cursor or the session ends. A session with no managed
    /// channel is refused `attach_pty_use_read_bytes`.
    pub fn attach_read(
        &self,
        id: &str,
        cursor: Option<u64>,
        follow: bool,
        left: &AtomicBool,
    ) -> Result<Answer, RunnerError> {
        self.until_any(left, |table| {
            let Some(session) = table.sessions.get(id) else {
                return Some(Err(unknown(id)));
            };
            if session.managed.is_none() {
                return Some(Err(RunnerError::refused(
                    "attach_pty_use_read_bytes",
                    format!(
                        "session {id} was not started managed, so it has no frames: follow its terminal with read_bytes"
                    ),
                )));
            }
            let ended = session.ended.is_some();
            let read = match session.attach.from(cursor) {
                Ok(read) => read,
                Err(error) => return Some(Err(error)),
            };
            if follow && read.lines.is_empty() && !ended {
                return None;
            }
            Some(Ok(Answer::AttachLines {
                lines: read.lines,
                cursor: read.cursor,
                ended,
            }))
        })?
    }
}

#[cfg(test)]
#[path = "../tests/attach/cases.rs"]
mod cases;
