//! Latest context snapshots retain every reported cost delta until a boundary.

use serde_json::Value;

use super::{Sessions, Table, accounts, append, now_ms, text};
use crate::error::RunnerError;
use crate::tracking::{Figures, Reading, UsageRecord};
use crate::tracking_store::{Body, SourceState};

pub(crate) struct PendingStatus {
    record: UsageRecord,
    path: String,
    bound: String,
    generation: u64,
    snapshot: Option<Figures>,
    snapshot_account: Option<String>,
    reported_cost_micros: Option<u64>,
}

impl PendingStatus {
    fn matches(&self, source: &SourceState) -> bool {
        self.path == source.path
            && self.bound == source.bound
            && self.generation == source.generation
    }

    fn apply(&self, source: &mut SourceState) -> Result<(), RunnerError> {
        if !self.matches(source) {
            return Err(RunnerError::refused(
                "status_source_changed",
                "the pending snapshot belongs to a different source generation",
            ));
        }
        source.snapshot.clone_from(&self.snapshot);
        source.snapshot_account.clone_from(&self.snapshot_account);
        source.reported_cost_micros = self.reported_cost_micros;
        Ok(())
    }

    fn new(
        mut record: UsageRecord,
        source: &SourceState,
        previous: Option<&Self>,
    ) -> Result<Self, RunnerError> {
        if let Some(previous) = previous {
            record.figures.dollars_micros = match (
                previous.record.figures.dollars_micros,
                record.figures.dollars_micros,
            ) {
                (Some(before), Some(delta)) => {
                    Some(before.checked_add(delta).ok_or_else(|| {
                        RunnerError::refused(
                            "status_cost_overflow",
                            "coalesced reported cost exceeds the microdollar range",
                        )
                    })?)
                }
                (before, None) => before,
                (None, delta) => delta,
            };
            for note in &previous.record.unavailable {
                if note.reason == "reported_session_cost_reset"
                    && !record.unavailable.contains(note)
                {
                    record.unavailable.push(note.clone());
                }
            }
        }
        Ok(Self {
            record,
            path: source.path.clone(),
            bound: source.bound.clone(),
            generation: source.generation,
            snapshot: source.snapshot.clone(),
            snapshot_account: source.snapshot_account.clone(),
            reported_cost_micros: source.reported_cost_micros,
        })
    }
}

pub(crate) fn pending(
    table: &Table,
    id: &str,
    source: &mut SourceState,
) -> Result<Option<Body>, RunnerError> {
    let pending = table
        .sessions
        .get(id)
        .and_then(|session| session.pending_status.as_ref());
    if let Some(pending) = pending {
        pending.apply(source)?;
        Ok(Some(Body::Usage(pending.record.clone())))
    } else {
        Ok(None)
    }
}

pub(crate) fn clear(table: &mut Table, id: &str) {
    if let Some(session) = table.sessions.get_mut(id) {
        session.pending_status = None;
    }
}

pub(crate) fn flush_status(table: &mut Table, id: &str) -> Result<bool, RunnerError> {
    if table
        .sessions
        .get(id)
        .is_none_or(|session| session.pending_status.is_none())
    {
        return Ok(false);
    }
    let mut source = table.feed.source(id).cloned().ok_or_else(|| {
        RunnerError::refused(
            "status_source_missing",
            "the pending snapshot has no bound source",
        )
    })?;
    if let Some(body) = pending(table, id, &mut source)? {
        append(table, id, vec![body], Some(source))?;
        clear(table, id);
    }
    Ok(true)
}

impl Sessions {
    pub(super) fn status_line(&self, id: &str, input: &Value) -> Result<String, RunnerError> {
        let mut table = self.lock();
        let session = table
            .sessions
            .get(id)
            .ok_or_else(|| crate::session::unknown(id))?;
        let Some(tracking) = session.guard.tracking.as_ref() else {
            return Ok("the session is not tracked: nothing is kept".to_owned());
        };
        let Some(mut source) = table.feed.source(id).cloned() else {
            return Ok("no stream is bound yet: the snapshot is not kept".to_owned());
        };
        if text(input, "session_id") != Some(source.bound.as_str()) {
            return Err(RunnerError::refused(
                "session_mismatch",
                "the status line names another session than the one it is proved to be",
            ));
        }
        let now = now_ms();
        let account = accounts(session, tracking).at(now).0;
        let changed = session
            .pending_status
            .as_ref()
            .is_some_and(|pending| pending.record.account != account);
        let flushed = changed && flush_status(&mut table, id)?;
        if flushed {
            source = table.feed.source(id).cloned().ok_or_else(|| {
                RunnerError::refused(
                    "status_source_missing",
                    "the snapshot's source disappeared at the account boundary",
                )
            })?;
        }
        let session = table
            .sessions
            .get(id)
            .ok_or_else(|| crate::session::unknown(id))?;
        if let Some(pending) = &session.pending_status {
            pending.apply(&mut source)?;
        }
        let tracking = session.guard.tracking.as_ref().ok_or_else(|| {
            RunnerError::refused(
                "status_tracking_missing",
                "the snapshot's tracking contract disappeared",
            )
        })?;
        let reading = Reading {
            runner: self.runner(),
            session: id,
            tracking,
            accounts: accounts(session, tracking),
            now,
        };
        let record = format!("status-line:{id}:{}", table.feed.end());
        let result = if let Some(body) = reading.status(&mut source, input, record) {
            let Body::Usage(record) = body else {
                return Err(RunnerError::refused(
                    "status_record_invalid",
                    "the status adapter returned a non-usage record",
                ));
            };
            let pending =
                PendingStatus::new(record.clone(), &source, session.pending_status.as_ref());
            let leader = crate::session::window_limit(
                &mut table,
                id,
                std::slice::from_ref(&Body::Usage(record)),
            );
            let message = pending.and_then(|pending| {
                table
                    .sessions
                    .get_mut(id)
                    .ok_or_else(|| crate::session::unknown(id))?
                    .pending_status = Some(pending);
                Ok("a context snapshot held until the boundary; it adds no token spend")
            });
            (message, leader)
        } else {
            (Ok("the snapshot repeats the last one held"), None)
        };
        drop(table);
        if let Some(leader) = result.1 {
            crate::pty::end(&leader)?;
        }
        if flushed {
            self.writer.barrier()?;
            self.wake();
        }
        result.0.map(str::to_owned)
    }
}
