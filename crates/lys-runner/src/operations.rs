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
//! exit is seen. Public receipts contain digests only; a private preparation
//! retains its exact frame so recovery never needs another transcript.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::RunnerError;
use crate::protocol::Ended;
use crate::session::{Sessions, now_ms};

mod control;
mod store;
use control::Control;
pub(crate) use control::Prepared;
pub use control::{Certainty, ControlReceipt, Reconciled, Reconciliation};

mod delivery;
pub(crate) use delivery::{accept, compacting, deliver, ended};
use delivery::{feed, stop};

#[cfg(test)]
#[path = "../tests/operations_index/cases.rs"]
mod index_tests;
mod restart;
mod withdraw;
pub(crate) use restart::{begin_restart, finish_restart};

/// The legacy record format accepted by the forward migration.
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
    /// Compact one context crossing while keeping normal input held.
    ContextCompact {
        /// The configured compaction command.
        text: String,
        /// The existing crossing identity.
        crossing: String,
    },
    /// Queue a stable goal occurrence for current boundary authority.
    GoalReminder {
        /// The initial reminder words.
        text: String,
        /// The saved goal revision and occurrence.
        reference: crate::harness_control::ReminderReference,
    },
    /// Apply the service's current decision to one owned boundary.
    BoundaryReply {
        /// The exact generation and boundary decision.
        reply: crate::harness_control::BoundaryReply,
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
            Self::ContextCompact { .. } => "context_compact",
            Self::GoalReminder { .. } => "goal_reminder",
            Self::BoundaryReply { .. } => "boundary_reply",
            Self::Notice { .. } => "notice",
            Self::Reminder { .. } => "reminder",
            Self::Stop => "stop",
        }
    }

    fn text(&self) -> Option<&str> {
        match self {
            Self::Compact { text }
            | Self::ContextCompact { text, .. }
            | Self::GoalReminder { text, .. }
            | Self::Notice { text }
            | Self::Reminder { text } => Some(text),
            Self::Stop | Self::BoundaryReply { .. } => None,
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

/// The current operation projection and each privately retained preparation.
pub(crate) struct Operations {
    path: PathBuf,
    checkpoint: PathBuf,
    journal_offset: u64,
    checkpoint_offset: u64,
    checkpoint_bytes: u64,
    controls: HashMap<String, Control>,
    by_session: BTreeMap<String, BTreeSet<String>>,
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
        OperationState::Delivered => !matches!(
            outcome.request.as_str(),
            "stop" | "compact" | "context_compact"
        ),
        OperationState::Confirmed | OperationState::Uncertain | OperationState::Refused => true,
    }
}

impl Operations {
    pub(crate) fn writer(&mut self, writer: crate::durable::Writer) {
        self.writer = Some(writer);
    }

    fn fold(&mut self, outcome: OperationOutcome, now: u64) {
        let id = outcome.operation.clone();
        self.seen.insert(id.clone());
        self.by_session
            .entry(outcome.session.clone())
            .or_default()
            .insert(id.clone());
        if let Some(previous) = self.held.get(&id).filter(|held| terminal(held)) {
            let at = self.retention_at(previous);
            self.expires
                .remove(&(at.saturating_add(RETAIN_MS), id.clone()));
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
        let unresolved = outcome.state == OperationState::Uncertain
            && self
                .controls
                .get(&id)
                .is_none_or(|control| control.decision.is_none());
        if terminal(&outcome) {
            if let Some(active) = self.active.get_mut(&outcome.session) {
                active.remove(&id);
                if active.is_empty() {
                    self.active.remove(&outcome.session);
                }
            }
            if !unresolved {
                self.expires.insert((
                    self.retention_at(&outcome).saturating_add(RETAIN_MS),
                    id.clone(),
                ));
            }
        } else {
            self.active
                .entry(outcome.session.clone())
                .or_default()
                .insert(id.clone());
        }
        self.held.insert(id, outcome);
        self.prune(now);
    }

    fn retention_at(&self, outcome: &OperationOutcome) -> u64 {
        if outcome.state == OperationState::Uncertain {
            self.controls
                .get(&outcome.operation)
                .and_then(|control| control.decision.as_ref())
                .map_or(outcome.at, |decision| decision.at)
        } else {
            outcome.at
        }
    }

    pub(crate) fn prune(&mut self, now: u64) {
        while self.expires.first().is_some_and(|(at, _)| *at <= now) {
            let Some((_, id)) = self.expires.pop_first() else {
                break;
            };
            if let Some(outcome) = self.held.remove(&id)
                && let Some(operations) = self.by_session.get_mut(&outcome.session)
            {
                operations.remove(&id);
                if operations.is_empty() {
                    self.by_session.remove(&outcome.session);
                }
            }
            self.controls.remove(&id);
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

    pub(crate) fn keep_control(&mut self, outcome: OperationOutcome) -> Result<(), RunnerError> {
        self.repeated(&outcome.operation)?;
        self.record(outcome)
    }

    pub(crate) fn check_control_identity(&self, operation: &str) -> Result<(), RunnerError> {
        self.repeated(operation)
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
        if matches!(operation.request, OperationRequest::BoundaryReply { .. }) {
            return Err(RunnerError::refused(
                "control_boundary_service_required",
                "only a verified service request may decide a boundary",
            ));
        }
        if matches!(
            operation.request,
            OperationRequest::Compact { .. } | OperationRequest::ContextCompact { .. }
        ) && self.lock()?.responsible.contains_key(&operation.session)
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
