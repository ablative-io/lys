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
//! Text waits for the harness's turn boundary, or types at once between
//! turns or in a session that reports no turns. A tracked busy session
//! that never signals its boundary is released at the named wait budget;
//! its receipt keeps the unsignalled wait rather than hiding that exception. Before typing, `delivering` is made durable; after, it
//! is `delivered`. A runner that stops between the two cannot know whether
//! the text reached the terminal, so the next one reports the operation
//! `uncertain` and never types it again. A compaction is `confirmed` when
//! the harness says it is compacting, and a stop only when the session's
//! exit is seen. Public receipts contain digests only; a private preparation
//! retains its exact frame so recovery never needs another transcript.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::RunnerError;
use crate::session::now_ms;

mod control;
mod migration;
mod store;
use control::Control;
pub(crate) use control::Prepared;
pub use control::{Certainty, ControlPage, ControlReceipt, Reconciled, Reconciliation};

mod boundary;
mod wake;
pub(crate) use wake::confirm_on_turn_end;
mod delivery;
pub(crate) use delivery::{accept, compacting, deliver, ended};
use delivery::{feed, stop};

#[cfg(test)]
#[path = "../tests/operations_boundary.rs"]
mod boundary_tests;
#[cfg(test)]
#[path = "../tests/operations_index/cases.rs"]
mod index_tests;
mod restart;
mod withdraw;
pub(crate) use restart::{begin_restart, finish_restart};

/// The legacy record format accepted by the forward migration.
pub const FORMAT: &str = "lys-runner-operations/v1";
/// Maximum wait for a tracked terminal to signal a turn boundary.
pub const BOUNDARY_WAIT: std::time::Duration = std::time::Duration::from_secs(20 * 60);
const RETAIN_MS: u64 = 24 * 60 * 60 * 1000;

mod requests;
pub use requests::{Operation, OperationOutcome, OperationRequest, OperationState, TextDigest};

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
    batch: Option<control::JournalBatch>,
    checkpoint_offset: u64,
    checkpoint_bytes: u64,
    controls: HashMap<String, Control>,
    original_requests: HashMap<String, [u8; 32]>,
    by_session: BTreeMap<String, BTreeSet<String>>,
    held: HashMap<String, OperationOutcome>,
    seen: HashSet<String>,
    expires: BTreeSet<(u64, String)>,
    accepted: BTreeMap<String, VecDeque<String>>,
    active: BTreeMap<String, BTreeSet<String>>,
    texts: BTreeMap<String, String>,
    compacting: BTreeSet<String>,
    boundary_waiters: HashMap<String, std::sync::mpsc::Sender<()>>,
    boundary_expired: HashMap<String, u64>,
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
        if outcome.state != OperationState::Accepted {
            self.boundary_waiters.remove(&id);
            self.boundary_expired.remove(&id);
            if let Some(queue) = self.accepted.get_mut(&outcome.session) {
                queue.retain(|operation| operation != &id);
                if queue.is_empty() {
                    self.accepted.remove(&outcome.session);
                }
            }
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
            self.original_requests.remove(&id);
            self.texts.remove(&id);
            self.compacting.remove(&id);
            self.boundary_waiters.remove(&id);
            self.boundary_expired.remove(&id);
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

    pub(crate) fn keep_control(
        &mut self,
        outcome: OperationOutcome,
        request: &OperationRequest,
    ) -> Result<(), RunnerError> {
        self.repeated(&outcome.operation)?;
        self.record_request(outcome, request.identity()?)
    }

    pub(crate) fn check_control_identity(&self, operation: &str) -> Result<(), RunnerError> {
        self.repeated(operation)
    }

    pub(crate) fn original_request(&self, operation: &str) -> Option<&[u8; 32]> {
        self.original_requests.get(operation)
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
        outcome.words = receipt_words(&outcome.words, words);
        self.record(outcome.clone())?;
        Ok(outcome)
    }
}

mod session;

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

impl Operations {
    pub(crate) fn begin_journal_batch(&mut self) -> Result<(), RunnerError> {
        if self.batch.is_some() {
            return Err(unavailable("a journal decision batch is already active"));
        }
        self.batch = Some(control::JournalBatch::default());
        Ok(())
    }
    pub(crate) fn finish_journal_batch(&mut self) -> Result<(), RunnerError> {
        self.batch
            .take()
            .ok_or_else(|| unavailable("no journal decision batch is active"))?
            .finish()?;
        self.checkpoint_if_due()
    }
    pub(crate) fn cancel_journal_batch(&mut self) {
        drop(self.batch.take());
    }
}

fn receipt_words(previous: &str, words: String) -> String {
    match previous.split_once("; ") {
        Some((note, _))
            if (note.starts_with("wake_up ")
                || note.starts_with("boundary_wait=")
                || note.contains("boundary_unsignalled"))
                && !words.contains(note) =>
        {
            format!("{note}; {words}")
        }
        _ => words,
    }
}
