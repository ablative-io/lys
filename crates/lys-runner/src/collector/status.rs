//! Latest context snapshots retain every reported cost delta until a boundary.

use serde_json::Value;

use super::{Sessions, Table, accounts, append, now_ms, text};
use crate::error::RunnerError;
use crate::tracking::{Figures, Reading, UsageRecord};
use crate::tracking_store::{Body, SourceState};

#[cfg(test)]
thread_local! {
    pub(super) static BEFORE_END: std::cell::RefCell<Option<Box<dyn FnOnce()>>> = const { std::cell::RefCell::new(None) };
}

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

/// Hold what the status line of session `id`, tracked through the proxy,
/// reported of its dollars and running time. It is written with the next
/// calls read from the run's usage file, and when the session ends.
fn proxied_status(
    runner: &str,
    table: &mut Table,
    id: &str,
    proxy: &crate::tracking_proxy::ProxyTracking,
    input: &Value,
) -> Result<String, RunnerError> {
    let Some(mut source) = table.feed.source(id).cloned() else {
        return Ok("no usage file is bound yet: the figures are not kept".to_owned());
    };
    let previous = table
        .sessions
        .get(id)
        .and_then(|session| session.pending_status.as_ref());
    if let Some(pending) = previous {
        pending.apply(&mut source)?;
    }
    let reading = crate::tracking_proxy::ProxyReading {
        runner,
        session: id,
        tracking: proxy,
        now: now_ms(),
    };
    let record = format!("status-line:{id}:{}", table.feed.end());
    let Some(Body::Usage(record)) = reading.status(&mut source, input, record) else {
        return Ok("the figures repeat the last ones held".to_owned());
    };
    let pending = PendingStatus::new(record, &source, previous)?;
    table
        .sessions
        .get_mut(id)
        .ok_or_else(|| crate::session::unknown(id))?
        .pending_status = Some(pending);
    Ok("dollars and running time held until the run's next calls are read".to_owned())
}

/// Keep what the status line of session `id`, tracked neither natively nor
/// through the proxy, reported of its plan's windows. A run on a Claude
/// subscription goes through no proxy, and Claude Code reports a plan's
/// windows (`rate_limits`) to its status line and nowhere else, so without
/// this a `plan_percent` budget on it was refused as "plan window
/// unreported". Only a change is kept, as a snapshot claiming no tokens,
/// dollars or running time. The account is the rotation's current one, or,
/// for a run that does not rotate, the sign-in of this runner's own login,
/// named `login@<runner>`.
fn plan_status(
    runner: &str,
    table: &mut Table,
    id: &str,
    input: &Value,
) -> Result<String, RunnerError> {
    let now = now_ms();
    let mut unavailable = Vec::new();
    let mut windows = crate::tracking_budget::windows(input, &mut unavailable);
    crate::tracking_budget::live(&mut windows, now, &mut unavailable);
    if windows.is_empty() {
        return Ok("the status line named no plan window: nothing is kept".to_owned());
    }
    let session = table
        .sessions
        .get_mut(id)
        .ok_or_else(|| crate::session::unknown(id))?;
    let account = session.rotation.as_ref().map_or_else(
        || format!("login@{runner}"),
        |rotation| rotation.account().to_owned(),
    );
    let held = (account.clone(), windows.clone());
    if session.plan_windows.as_ref() == Some(&held) {
        return Ok("the plan's windows repeat the last ones kept".to_owned());
    }
    for figure in [
        "input_tokens",
        "output_tokens",
        "cache_creation_tokens",
        "cache_read_tokens",
        "context_tokens",
        "dollars_micros",
        "running_ms",
    ] {
        unavailable.push(crate::tracking::Unavailable {
            figure: figure.to_owned(),
            reason: "a plan run's status line is read for its plan windows only".to_owned(),
        });
    }
    let model = input
        .get("model")
        .and_then(|model| model.get("id"))
        .and_then(Value::as_str)
        .map(str::to_owned);
    let record = UsageRecord {
        version: crate::tracking::RECORD_VERSION,
        id: format!("plan-status:{id}:{}", table.feed.end()),
        runner: runner.to_owned(),
        session: id.to_owned(),
        generation: 0,
        offset: None,
        turn: None,
        observed_at: now,
        measure: crate::tracking::Measure::Snapshot,
        figures: Figures {
            plan_windows: windows,
            ..Figures::default()
        },
        unavailable,
        adapter: crate::tracking::CLAUDE_ADAPTER.to_owned(),
        model,
        account: Some(account),
        account_unknown: None,
        context_window: 0,
        profile_version: 0,
        run: None,
        record: None,
    };
    append(table, id, vec![Body::Usage(record)], None)?;
    table
        .sessions
        .get_mut(id)
        .ok_or_else(|| crate::session::unknown(id))?
        .plan_windows = Some(held);
    Ok("the plan's windows are kept".to_owned())
}

impl Sessions {
    pub(super) fn status_line(&self, id: &str, input: &Value) -> Result<String, RunnerError> {
        let mut table = self.lock()?;
        let session = table
            .sessions
            .get(id)
            .ok_or_else(|| crate::session::unknown(id))?;
        let Some(tracking) = session.guard.tracking.as_ref() else {
            let proxy = session.guard.proxy.clone();
            return match proxy {
                Some(proxy) => proxied_status(self.runner(), &mut table, id, &proxy, input),
                None => plan_status(self.runner(), &mut table, id, input),
            };
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
        let stopping = result.1.is_some();
        let appended = if stopping {
            flush_status(&mut table, id).map(|_| ())
        } else {
            Ok(())
        };
        drop(table);
        let durable = if stopping || flushed {
            let durable = self.writer.barrier();
            self.wake();
            durable
        } else {
            Ok(())
        };
        let ended = if let Some(leader) = result.1 {
            #[cfg(test)]
            BEFORE_END.with(|probe| {
                if let Some(probe) = probe.borrow_mut().take() {
                    probe();
                }
            });
            crate::pty::end(&leader)
        } else {
            Ok(())
        };
        for error in [
            result.0.as_ref().err(),
            appended.as_ref().err(),
            durable.as_ref().err(),
            ended.as_ref().err(),
        ]
        .into_iter()
        .flatten()
        {
            crate::error::said(&format!("session {id}: status_boundary_failed: {error}"));
        }
        let words = result.0?;
        appended.and(durable).and(ended)?;
        Ok(words.to_owned())
    }
}
