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

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::RunnerError;
use crate::protocol::Ended;
use crate::session::{Sessions, now_ms};

mod delivery;
pub(crate) use delivery::{accept, compacting, deliver, ended};
use delivery::{feed, stop};

#[cfg(test)]
#[path = "../tests/operations_index/cases.rs"]
mod index_tests;
mod restart;
mod withdraw;
pub(crate) use restart::{begin_restart, finish_restart};

/// The format of the record of operations.
pub const FORMAT: &str = "lys-runner-operations/v1";
const RETAIN_MS: u64 = 24 * 60 * 60 * 1000;

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
    held: HashMap<String, OperationOutcome>,
    seen: HashSet<String>,
    expires: BTreeSet<(u64, String)>,
    accepted: BTreeMap<String, VecDeque<String>>,
    active: BTreeMap<String, BTreeSet<String>>,
    texts: BTreeMap<String, String>,
    compacting: BTreeSet<String>,
    writer: Option<crate::durable::Writer>,
}

fn unavailable(what: impl std::fmt::Display) -> RunnerError {
    RunnerError::State {
        reason: format!("the operations record: {what}"),
    }
}

fn terminal(outcome: &OperationOutcome) -> bool {
    match outcome.state {
        OperationState::Accepted | OperationState::Delivering => false,
        OperationState::Delivered => !matches!(outcome.request.as_str(), "stop" | "compact"),
        OperationState::Confirmed | OperationState::Uncertain | OperationState::Refused => true,
    }
}

impl Operations {
    /// The operations recorded in `dir`. One a stopped runner left being
    /// typed is uncertain; one it left waiting is refused, its session gone.
    pub(crate) fn open(dir: &Path) -> Result<Self, RunnerError> {
        Self::open_at(dir, now_ms())
    }

    fn open_at(dir: &Path, now: u64) -> Result<Self, RunnerError> {
        let path = dir.join("operations.jsonl");
        let old = dir.join("operations.json");
        if !path.exists() {
            let outcomes = match std::fs::read(&old) {
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
            let mut bytes = Vec::new();
            for outcome in outcomes {
                serde_json::to_writer(&mut bytes, &outcome).map_err(unavailable)?;
                bytes.push(b'\n');
            }
            crate::state::replace(&path, &bytes).map_err(unavailable)?;
        }
        let mut operations = Self {
            path,
            held: HashMap::new(),
            seen: HashSet::new(),
            expires: BTreeSet::new(),
            accepted: BTreeMap::new(),
            active: BTreeMap::new(),
            texts: BTreeMap::new(),
            compacting: BTreeSet::new(),
            writer: None,
        };
        let file = std::fs::File::open(&operations.path).map_err(unavailable)?;
        let length = file.metadata().map_err(unavailable)?.len();
        let mut reader = BufReader::new(file);
        let mut committed = 0;
        loop {
            let mut line = Vec::new();
            let read = reader.read_until(b'\n', &mut line).map_err(unavailable)?;
            if read == 0 {
                break;
            }
            if line.last() != Some(&b'\n') {
                break;
            }
            let outcome: OperationOutcome = serde_json::from_slice(&line).map_err(unavailable)?;
            operations.fold(outcome, now);
            committed += read as u64;
        }
        if committed < length {
            let file = std::fs::OpenOptions::new()
                .write(true)
                .open(&operations.path)
                .map_err(unavailable)?;
            file.set_len(committed)
                .and_then(|()| file.sync_all())
                .map_err(unavailable)?;
            crate::error::said(
                "operations_tail_incomplete: the uncommitted last record was removed",
            );
        }
        match std::fs::remove_file(&old) {
            Ok(()) => std::fs::File::open(dir)
                .and_then(|file| file.sync_all())
                .map_err(unavailable)?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(unavailable(error)),
        }
        let interrupted = operations
            .held
            .values()
            .filter(|outcome| {
                matches!(
                    outcome.state,
                    OperationState::Accepted | OperationState::Delivering
                )
            })
            .cloned()
            .collect::<Vec<_>>();
        for mut outcome in interrupted {
            let (state, words) = match outcome.state {
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
            outcome.at = now_ms();
            words.clone_into(&mut outcome.words);
            operations.record(outcome)?;
        }
        Ok(operations)
    }

    pub(crate) fn writer(&mut self, writer: crate::durable::Writer) {
        self.writer = Some(writer);
    }

    fn fold(&mut self, outcome: OperationOutcome, now: u64) {
        let id = outcome.operation.clone();
        self.seen.insert(id.clone());
        if let Some(previous) = self.held.get(&id).filter(|held| terminal(held)) {
            self.expires
                .remove(&(previous.at.saturating_add(RETAIN_MS), id.clone()));
        }
        if outcome.state == OperationState::Accepted
            && outcome.request != "stop"
            && !self.held.contains_key(&id)
        {
            self.accepted
                .entry(outcome.session.clone())
                .or_default()
                .push_back(id.clone());
        }
        if terminal(&outcome) {
            if let Some(active) = self.active.get_mut(&outcome.session) {
                active.remove(&id);
                if active.is_empty() {
                    self.active.remove(&outcome.session);
                }
            }
            self.expires
                .insert((outcome.at.saturating_add(RETAIN_MS), id.clone()));
        } else {
            self.active
                .entry(outcome.session.clone())
                .or_default()
                .insert(id.clone());
        }
        self.held.insert(id, outcome);
        self.prune(now);
    }

    pub(crate) fn prune(&mut self, now: u64) {
        while self.expires.first().is_some_and(|(at, _)| *at <= now) {
            let Some((_, id)) = self.expires.pop_first() else {
                break;
            };
            self.held.remove(&id);
            self.texts.remove(&id);
            self.compacting.remove(&id);
        }
    }

    fn repeated(&self, id: &str) -> Result<(), RunnerError> {
        if self.seen.contains(id) {
            return Err(RunnerError::refused(
                "operation_repeated",
                format!("operation {id} was already used; its terminal outcome has expired"),
            ));
        }
        Ok(())
    }

    fn record(&mut self, outcome: OperationOutcome) -> Result<(), RunnerError> {
        let mut bytes = serde_json::to_vec(&outcome).map_err(unavailable)?;
        bytes.push(b'\n');
        if let Some(writer) = &self.writer {
            writer.append(&self.path, bytes)?;
        } else {
            let mut file = std::fs::OpenOptions::new()
                .append(true)
                .open(&self.path)
                .map_err(unavailable)?;
            file.write_all(&bytes)
                .and_then(|()| file.sync_data())
                .map_err(unavailable)?;
        }
        self.fold(outcome, now_ms());
        Ok(())
    }

    /// The outcome of `operation`.
    pub(crate) fn get(&self, operation: &str) -> Option<&OperationOutcome> {
        self.held.get(operation)
    }

    fn set(
        &mut self,
        operation: &str,
        state: OperationState,
        words: String,
    ) -> Result<OperationOutcome, RunnerError> {
        let mut outcome = self
            .get(operation)
            .cloned()
            .ok_or_else(|| unavailable(format!("no operation {operation} is held")))?;
        outcome.state = state;
        outcome.at = now_ms();
        outcome.words = words;
        self.record(outcome.clone())?;
        Ok(outcome)
    }
}

impl Sessions {
    /// Accept `operation` under its stable id, answering how it stands.
    pub fn operate(&self, operation: Operation) -> Result<OperationOutcome, RunnerError> {
        if matches!(operation.request, OperationRequest::Compact { .. })
            && self.lock()?.responsible.contains_key(&operation.session)
        {
            return Err(RunnerError::refused(
                "SessionInputContextMissing",
                "owned compaction requires a verified caller and live grant judge",
            ));
        }
        self.operate_admitted(operation)
    }

    pub(crate) fn operate_admitted(
        &self,
        operation: Operation,
    ) -> Result<OperationOutcome, RunnerError> {
        let mut table = self.lock()?;
        let mut outcome = accept(&mut table, operation)?;
        drop(table);
        self.writer.barrier()?;
        if outcome.request == "stop" && outcome.state == OperationState::Accepted {
            let mut table = self.lock()?;
            outcome = stop(&mut table, &outcome.session, &outcome.operation)?;
            drop(table);
            self.writer.barrier()?;
        }
        self.wake();
        if outcome.state != OperationState::Delivering {
            return Ok(outcome);
        }
        let outcome = self.until_any(&AtomicBool::new(false), |table| {
            table
                .operations
                .get(&outcome.operation)
                .filter(|held| held.state != OperationState::Delivering)
                .cloned()
        })?;
        self.writer.barrier()?;
        Ok(outcome)
    }

    /// How operation `operation` stands, as the record keeps it.
    pub fn outcome(&self, operation: &str) -> Result<OperationOutcome, RunnerError> {
        let outcome = self
            .lock()?
            .operations
            .get(operation)
            .cloned()
            .ok_or_else(|| {
                RunnerError::refused(
                    "operation_unknown",
                    format!("no operation {operation} is held"),
                )
            })?;
        self.writer.barrier()?;
        Ok(outcome)
    }
}

pub(crate) fn managed_take(
    table: &mut crate::session::Table,
    session: &str,
    operation: &str,
) -> Result<String, RunnerError> {
    let queue = table
        .operations
        .accepted
        .get_mut(session)
        .ok_or_else(|| unavailable("managed operation has no accepted queue"))?;
    if queue.back().map(String::as_str) != Some(operation) {
        return Err(unavailable(
            "managed operation is not the newly accepted input",
        ));
    }
    queue.pop_back();
    table
        .operations
        .texts
        .remove(operation)
        .ok_or_else(|| unavailable("managed operation has no held text"))
}

pub(crate) fn managed_state(
    table: &mut crate::session::Table,
    operation: &str,
    state: OperationState,
    reason: String,
) -> Result<OperationOutcome, RunnerError> {
    let outcome = table.operations.set(operation, state, reason)?;
    feed(table, &outcome);
    Ok(outcome)
}
