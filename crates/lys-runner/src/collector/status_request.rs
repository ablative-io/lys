//! The snapshot transaction releases the table before durability and process completion.
#[cfg(test)]
use super::BEFORE_END;
use super::{
    Body, PendingStatus, Reading, RunnerError, Sessions, Value, accounts, flush_status, now_ms,
    plan_status, proxied_status, text,
};
impl Sessions {
    pub(in crate::collector) fn status_line(
        &self,
        id: &str,
        input: &Value,
    ) -> Result<String, RunnerError> {
        let mut table = self.lock()?;
        let session = table
            .sessions
            .get(id)
            .ok_or_else(|| crate::session::unknown(id))?;
        if let Some(proxy) = table
            .feed
            .source(id)
            .and_then(|source| session.guard.proxy_source(source))
            .cloned()
        {
            return proxied_status(self.runner(), &mut table, id, &proxy, input);
        }
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
        self.finish_status(id, result, appended, stopping || flushed)
    }

    fn finish_status(
        &self,
        id: &str,
        result: (
            Result<&'static str, RunnerError>,
            Option<crate::peer::Leader>,
        ),
        appended: Result<(), RunnerError>,
        needs_barrier: bool,
    ) -> Result<String, RunnerError> {
        let durable = if needs_barrier {
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
