//! Operations on a session under stable identities, and the turn boundaries
//! they wait for.
//!
//! An operation is asked under an id the server chose and keeps: a
//! compaction, a notice or a reminder typed into the session, or a stop.
//! Its acceptance is made durable before it is answered, and asked again
//! under the same id it is answered as it stands and never done twice; the
//! same id asked for something else is refused `operation_reused`. A fresh
//! connection's challenge is never its identity.
//!
//! Text is typed only at a turn boundary the harness signalled: after a
//! turn ends, or at once when the session is already between turns or
//! reports no turns. Before typing, `delivering` is made durable; after, it
//! is `delivered`. A runner that stops between the two cannot know whether
//! the text reached the terminal, so the next one reports the operation
//! `uncertain` and never types it again. A compaction is `confirmed` when
//! the harness says it is compacting, and a stop only when the session's
//! exit is seen. The record keeps each outcome and a digest of its text,
//! never the text itself.

use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::RunnerError;
use crate::protocol::{Ended, Key};
use crate::session::{Session, Sessions, Table, now_ms};
use crate::tracking_store::{Body, Commit};

pub mod control;

/// The format of the record of operations.
pub const FORMAT: &str = "lys-runner-operations/v1";

/// What an operation asks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "request", rename_all = "snake_case", deny_unknown_fields)]
pub enum OperationRequest {
    /// Type the harness's compaction command at the next turn boundary.
    Compact {
        /// The command.
        text: String,
    },
    /// Type a notice to the session at the next turn boundary.
    Notice {
        /// The notice.
        text: String,
    },
    /// Type a reminder at the next turn boundary.
    Reminder {
        /// The reminder.
        text: String,
    },
    /// End the session's process.
    Stop,
}

impl OperationRequest {
    /// Its name.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Compact { .. } => "compact",
            Self::Notice { .. } => "notice",
            Self::Reminder { .. } => "reminder",
            Self::Stop => "stop",
        }
    }

    fn text(&self) -> Option<&str> {
        match self {
            Self::Compact { text } | Self::Notice { text } | Self::Reminder { text } => Some(text),
            Self::Stop => None,
        }
    }
}

/// An operation as it is asked.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Operation {
    /// Its stable id, chosen and kept by the server.
    pub operation: String,
    /// The session.
    pub session: String,
    /// What it asks.
    pub request: OperationRequest,
}

/// Where an operation stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationState {
    /// Accepted, waiting for its boundary.
    Accepted,
    /// Being typed; a runner stopped here leaves it uncertain.
    Delivering,
    /// Typed, or for a stop, the end sent.
    Delivered,
    /// Seen done: a compaction begun, a stop's exit seen.
    Confirmed,
    /// Its delivery cannot be known; it is never typed again.
    Uncertain,
    /// Not done, by name.
    Refused,
}

/// Text as the record keeps it: its length and digest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TextDigest {
    /// Its length in bytes.
    pub length: usize,
    /// The lowercase hex SHA-256 of its bytes.
    pub sha256: String,
}

impl TextDigest {
    /// `text`'s length and digest.
    pub fn of(text: &str) -> Self {
        Self {
            length: text.len(),
            sha256: crate::protocol::hex(&Sha256::digest(text.as_bytes())),
        }
    }
}

/// An operation's outcome.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperationOutcome {
    /// The operation.
    pub operation: String,
    /// The session.
    pub session: String,
    /// What it asked, by name.
    pub request: String,
    /// Where it stands.
    pub state: OperationState,
    /// When it came to stand there, in milliseconds since the Unix epoch.
    pub at: u64,
    /// Why, in words.
    pub words: String,
    /// The text it types, as length and digest.
    pub text: Option<TextDigest>,
    /// The session's end, for a confirmed stop.
    pub ended: Option<Ended>,
    /// Bound native delivery metadata; absent for legacy/manual operations.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub control: Option<Box<control::Delivery>>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Kept {
    format: String,
    operations: Vec<OperationOutcome>,
}

/// Every operation the runner holds, and the text of those not yet typed,
/// in memory only.
pub(crate) struct Operations {
    path: PathBuf,
    held: Vec<OperationOutcome>,
    texts: BTreeMap<String, String>,
}

fn unavailable(what: impl std::fmt::Display) -> RunnerError {
    RunnerError::State {
        reason: format!("the operations record: {what}"),
    }
}

impl Operations {
    /// The operations recorded in `dir`. One a stopped runner left being
    /// typed is uncertain; one it left waiting is refused, its session gone.
    pub(crate) fn open(dir: &Path) -> Result<Self, RunnerError> {
        let path = dir.join("operations.json");
        let mut held = match std::fs::read(&path) {
            Ok(bytes) => {
                let kept: Kept = serde_json::from_slice(&bytes).map_err(unavailable)?;
                if kept.format != FORMAT {
                    return Err(unavailable(format!("in format {}", kept.format)));
                }
                kept.operations
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Vec::new(),
            Err(error) => return Err(unavailable(error)),
        };
        let at = now_ms();
        for outcome in &mut held {
            let (state, words) = match outcome.state {
                OperationState::Delivering if outcome.control.is_some() => (
                    OperationState::Uncertain,
                    "the runner stopped after recording a possible managed pipe write; reconcile native evidence before any further delivery",
                ),
                OperationState::Delivering => (
                    OperationState::Uncertain,
                    "the runner stopped while typing it: whether it reached the session cannot be known, and it is not typed again",
                ),
                OperationState::Accepted => (
                    OperationState::Refused,
                    "session_ended: the runner stopped before its boundary came",
                ),
                _ => continue,
            };
            outcome.state = state;
            outcome.at = at;
            words.clone_into(&mut outcome.words);
        }
        let operations = Self {
            path,
            held,
            texts: BTreeMap::new(),
        };
        operations.persist()?;
        Ok(operations)
    }

    fn persist(&self) -> Result<(), RunnerError> {
        let kept = Kept {
            format: FORMAT.to_owned(),
            operations: self.held.clone(),
        };
        let bytes = serde_json::to_vec(&kept).map_err(unavailable)?;
        crate::state::replace(&self.path, &bytes).map_err(unavailable)
    }

    /// The outcome of `operation`.
    pub(crate) fn get(&self, operation: &str) -> Option<&OperationOutcome> {
        self.held.iter().find(|held| held.operation == operation)
    }

    fn set(
        &mut self,
        operation: &str,
        state: OperationState,
        words: String,
    ) -> Result<OperationOutcome, RunnerError> {
        let outcome = self
            .held
            .iter_mut()
            .find(|held| held.operation == operation)
            .ok_or_else(|| unavailable(format!("no operation {operation} is held")))?;
        outcome.state = state;
        outcome.at = now_ms();
        outcome.words = words;
        let changed = outcome.clone();
        self.persist()?;
        Ok(changed)
    }
}

/// Say `outcome` in the feed, naming in the log a feed that cannot take it:
/// the record of operations still answers it.
fn feed(table: &mut Table, outcome: &OperationOutcome) {
    let appended = table.feed.append(
        &outcome.session,
        outcome.at,
        vec![Body::Operation(outcome.clone())],
        Commit::default(),
    );
    if let Err(error) = appended {
        crate::error::said(&format!(
            "operation {}: its outcome was not fed: {error}",
            outcome.operation
        ));
    }
}

/// Accept `operation` on the table, answering how it stands.
pub(crate) fn accept(
    table: &mut Table,
    operation: Operation,
) -> Result<OperationOutcome, RunnerError> {
    if let Some(held) = table.operations.get(&operation.operation) {
        let same = held.session == operation.session
            && held.request == operation.request.name()
            && held.text == operation.request.text().map(TextDigest::of);
        if !same {
            return Err(RunnerError::refused(
                "operation_reused",
                format!(
                    "operation {} already names another act",
                    operation.operation
                ),
            ));
        }
        return Ok(held.clone());
    }
    let session = table.sessions.get(&operation.session).ok_or_else(|| {
        RunnerError::refused(
            "session_unknown",
            format!("no session {} is held", operation.session),
        )
    })?;
    let (state, words) = if session.ended.is_some() {
        (
            OperationState::Refused,
            "session_ended: the session has ended and does nothing more".to_owned(),
        )
    } else {
        (OperationState::Accepted, "accepted".to_owned())
    };
    let outcome = OperationOutcome {
        operation: operation.operation.clone(),
        session: operation.session.clone(),
        request: operation.request.name().to_owned(),
        state,
        at: now_ms(),
        words,
        text: operation.request.text().map(TextDigest::of),
        ended: None,
        control: None,
    };
    table.operations.held.push(outcome.clone());
    table.operations.persist()?;
    feed(table, &outcome);
    if state == OperationState::Refused {
        return Ok(outcome);
    }
    if let Some(text) = operation.request.text() {
        table
            .operations
            .texts
            .insert(operation.operation.clone(), text.to_owned());
    }
    let id = operation.session;
    if operation.request == OperationRequest::Stop {
        return stop(table, &id, &operation.operation);
    }
    let waits = table
        .sessions
        .get(&id)
        .is_some_and(|session| session.guard.tracking.is_some() && !session.guard.idle);
    if !waits {
        deliver(table, &id);
    }
    Ok(table
        .operations
        .get(&operation.operation)
        .cloned()
        .unwrap_or(outcome))
}

fn stop(table: &mut Table, id: &str, operation: &str) -> Result<OperationOutcome, RunnerError> {
    let Some(session) = table.sessions.get_mut(id) else {
        return Err(RunnerError::refused(
            "session_unknown",
            format!("no session {id} is held"),
        ));
    };
    session.ending = true;
    if let Some(live) = &session.live {
        live.end(id);
    }
    let outcome = table.operations.set(
        operation,
        OperationState::Delivered,
        "the end was sent; it is confirmed when the exit is seen".to_owned(),
    )?;
    feed(table, &outcome);
    Ok(outcome)
}

/// Type the first accepted operation of session `id` that waits for a
/// boundary: `delivering` made durable first, `delivered` after.
pub(crate) fn deliver(table: &mut Table, id: &str) {
    let next = table
        .operations
        .held
        .iter()
        .find(|held| {
            held.session == id && held.state == OperationState::Accepted && held.request != "stop"
        })
        .map(|held| held.operation.clone());
    let Some(operation) = next else {
        return;
    };
    let text = table
        .operations
        .texts
        .remove(&operation)
        .unwrap_or_default();
    let marked = table.operations.set(
        &operation,
        OperationState::Delivering,
        "being typed".to_owned(),
    );
    if let Err(error) = marked {
        crate::error::said(&format!(
            "operation {operation} was not typed: its delivery could not be recorded first: {error}"
        ));
        return;
    }
    let typed = table
        .sessions
        .get_mut(id)
        .ok_or_else(|| RunnerError::refused("session_unknown", format!("no session {id}")))
        .and_then(|session| type_text(session, id, &text));
    let (state, words) = match typed {
        Ok(()) => (
            OperationState::Delivered,
            "typed into the session".to_owned(),
        ),
        Err(error) => (OperationState::Refused, error.to_string()),
    };
    if let Some(session) = table.sessions.get_mut(id) {
        session.guard.idle = false;
    }
    match table.operations.set(&operation, state, words) {
        Ok(outcome) => feed(table, &outcome),
        Err(error) => crate::error::said(&format!(
            "operation {operation}: its delivery was not recorded, and it stays uncertain: {error}"
        )),
    }
}

fn type_text(session: &mut Session, id: &str, text: &str) -> Result<(), RunnerError> {
    let live = session.live(id)?;
    let mut bytes = text.as_bytes().to_vec();
    bytes.extend_from_slice(Key::Enter.bytes());
    live.writer
        .write_all(&bytes)
        .and_then(|()| live.writer.flush())
        .map_err(|error| RunnerError::refused("write_failed", error.to_string()))
}

/// The harness says it is compacting: a delivered compaction is confirmed.
pub(crate) fn compacting(table: &mut Table, id: &str) {
    let delivered: Vec<String> = table
        .operations
        .held
        .iter()
        .filter(|held| {
            held.session == id
                && held.request == "compact"
                && held.state == OperationState::Delivered
                && held.control.is_none()
        })
        .map(|held| held.operation.clone())
        .collect();
    for operation in delivered {
        match table.operations.set(
            &operation,
            OperationState::Confirmed,
            "the harness said it is compacting".to_owned(),
        ) {
            Ok(outcome) => feed(table, &outcome),
            Err(error) => crate::error::said(&format!("operation {operation}: {error}")),
        }
    }
}

/// Session `id` ended with `ended`: a sent stop is confirmed by it, what
/// was being typed is uncertain, and what waited is refused.
pub(crate) fn ended(table: &mut Table, id: &str, ended: &Ended) {
    let open: Vec<(String, OperationState, String)> = table
        .operations
        .held
        .iter()
        .filter(|held| held.session == id)
        .map(|held| (held.operation.clone(), held.state, held.request.clone()))
        .collect();
    for (operation, state, request) in open {
        let (next, words) = match (state, request.as_str()) {
            (OperationState::Delivered, "stop") => (
                OperationState::Confirmed,
                format!("the session's exit was seen at {} ms", ended.at),
            ),
            (OperationState::Accepted, _) => (
                OperationState::Refused,
                "session_ended: the session ended before its boundary came".to_owned(),
            ),
            (OperationState::Delivering, _) => (
                OperationState::Uncertain,
                "the session ended while it was typed".to_owned(),
            ),
            _ => continue,
        };
        table.operations.texts.remove(&operation);
        let changed = table
            .operations
            .set(&operation, next, words)
            .map(|mut outcome| {
                if next == OperationState::Confirmed {
                    outcome.ended = Some(ended.clone());
                }
                outcome
            });
        match changed {
            Ok(outcome) => {
                if let Some(held) = table
                    .operations
                    .held
                    .iter_mut()
                    .find(|held| held.operation == operation)
                {
                    held.ended.clone_from(&outcome.ended);
                }
                feed(table, &outcome);
            }
            Err(error) => crate::error::said(&format!("operation {operation}: {error}")),
        }
    }
    if let Err(error) = table.operations.persist() {
        crate::error::said(&format!("the operations record was not written: {error}"));
    }
}

impl Sessions {
    /// Accept `operation` under its stable id, answering how it stands.
    pub fn operate(&self, operation: Operation) -> Result<OperationOutcome, RunnerError> {
        let mut table = self.lock();
        let outcome = accept(&mut table, operation);
        drop(table);
        self.wake();
        outcome
    }

    /// How operation `operation` stands, as the record keeps it.
    pub fn outcome(&self, operation: &str) -> Result<OperationOutcome, RunnerError> {
        self.lock()
            .operations
            .get(operation)
            .cloned()
            .ok_or_else(|| {
                RunnerError::refused(
                    "operation_unknown",
                    format!("no operation {operation} is held"),
                )
            })
    }
}
