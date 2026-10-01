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
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Weak, atomic::AtomicBool};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::RunnerError;
use crate::protocol::{Ended, Key};
use crate::session::{Sessions, Table, now_ms};
use crate::tracking_store::{Body, Commit};

#[cfg(test)]
#[path = "../tests/operations_index/cases.rs"]
mod index_tests;
mod restart;
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
            let read = reader
                .by_ref()
                .take(1_048_577)
                .read_until(b'\n', &mut line)
                .map_err(unavailable)?;
            if read == 0 {
                break;
            }
            if line.len() > 1_048_576 {
                return Err(unavailable("operation_record_too_large"));
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
        let gap = table.gaps.entry(outcome.session.clone()).or_default();
        gap.lost += 1;
        gap.since.get_or_insert(outcome.at);
        gap.words = format!("an operation's outcome was not fed: {error}");
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
    table.operations.prune(now_ms());
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
    table.operations.repeated(&operation.operation)?;
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
    };
    table.operations.record(outcome.clone())?;
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
        return Ok(outcome);
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
        live.kill()?;
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
        .accepted
        .get_mut(id)
        .and_then(VecDeque::pop_front);
    let Some(operation) = next else {
        return;
    };
    let marked = table.operations.set(
        &operation,
        OperationState::Delivering,
        "being typed".to_owned(),
    );
    if let Err(error) = marked {
        crate::error::said(&format!(
            "operation {operation} was not typed: its delivery could not be recorded first: {error}"
        ));
        finish(table, &operation, Err(error));
        return;
    }
    let submitted = enqueue(table, id, &operation);
    if let Err(error) = submitted {
        finish(table, &operation, Err(error));
    }
}

fn enqueue(table: &mut Table, id: &str, operation: &str) -> Result<(), RunnerError> {
    let text = table.operations.texts.remove(operation).ok_or_else(|| {
        RunnerError::refused(
            "operation_text_missing",
            format!("operation {operation} has no text"),
        )
    })?;
    let session = table
        .sessions
        .get_mut(id)
        .ok_or_else(|| RunnerError::refused("session_unknown", format!("no session {id}")))?;
    let writer = session.live(id)?.writer.clone();
    session.guard.idle = false;
    let mut bytes = text.into_bytes();
    bytes.extend_from_slice(Key::Enter.bytes());
    let owner = Weak::clone(&table.owner);
    let durable = owner
        .upgrade()
        .ok_or_else(|| RunnerError::refused("runner_stopping", "the runner ended before delivery"))?
        .writer
        .clone();
    let operation = operation.to_owned();
    writer.submit_after(bytes, durable, move |result| {
        let Some(sessions) = owner.upgrade() else {
            crate::error::said(&format!(
                "operation {operation}: input completed after the runner ended"
            ));
            return;
        };
        let mut table = sessions.lock();
        finish(&mut table, &operation, result);
        drop(table);
        if let Err(error) = sessions.writer.barrier() {
            crate::error::said(&format!(
                "operation {operation}: delivery_record_uncertain: {error}"
            ));
        }
        sessions.wake();
    })
}

fn finish(table: &mut Table, operation: &str, result: Result<(), RunnerError>) {
    if !table
        .operations
        .get(operation)
        .is_some_and(|outcome| outcome.state == OperationState::Delivering)
    {
        if let Err(error) = result {
            crate::error::said(&format!(
                "operation {operation}: input ended after its outcome changed: {error}"
            ));
        }
        return;
    }
    let compacting = table.operations.compacting.remove(operation);
    let (state, words) = match result {
        Ok(()) if compacting => (
            OperationState::Confirmed,
            "the harness said it is compacting".to_owned(),
        ),
        Ok(()) => (
            OperationState::Delivered,
            "typed into the session".to_owned(),
        ),
        Err(error) => (OperationState::Refused, error.to_string()),
    };
    match table.operations.set(operation, state, words) {
        Ok(outcome) => feed(table, &outcome),
        Err(error) => crate::error::said(&format!(
            "operation {operation}: its delivery was not recorded, and it stays uncertain: {error}"
        )),
    }
}

/// The harness says it is compacting: a delivered compaction is confirmed.
pub(crate) fn compacting(table: &mut Table, id: &str) {
    let delivered: Vec<(String, OperationState)> = table
        .operations
        .active
        .get(id)
        .into_iter()
        .flatten()
        .filter_map(|operation| table.operations.get(operation))
        .filter(|held| {
            held.session == id
                && held.request == "compact"
                && matches!(
                    held.state,
                    OperationState::Delivering | OperationState::Delivered
                )
        })
        .map(|held| (held.operation.clone(), held.state))
        .collect();
    for (operation, state) in delivered {
        if state == OperationState::Delivering {
            table.operations.compacting.insert(operation);
            continue;
        }
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
        .active
        .get(id)
        .into_iter()
        .flatten()
        .filter_map(|operation| table.operations.get(operation))
        .filter(|held| held.session == id && held.request != "restart")
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
        table.operations.compacting.remove(&operation);
        let changed = table
            .operations
            .get(&operation)
            .cloned()
            .ok_or_else(|| unavailable(format!("no operation {operation} is held")))
            .and_then(|mut outcome| {
                outcome.state = next;
                outcome.at = now_ms();
                outcome.words = words;
                if next == OperationState::Confirmed {
                    outcome.ended = Some(ended.clone());
                }
                table.operations.record(outcome.clone())?;
                Ok(outcome)
            });
        match changed {
            Ok(outcome) => {
                feed(table, &outcome);
            }
            Err(error) => crate::error::said(&format!("operation {operation}: {error}")),
        }
    }
}

impl Sessions {
    /// Accept `operation` under its stable id, answering how it stands.
    pub fn operate(&self, operation: Operation) -> Result<OperationOutcome, RunnerError> {
        let mut table = self.lock();
        let mut outcome = accept(&mut table, operation)?;
        drop(table);
        self.writer.barrier()?;
        if outcome.request == "stop" && outcome.state == OperationState::Accepted {
            outcome = stop(&mut self.lock(), &outcome.session, &outcome.operation)?;
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
            .lock()
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
