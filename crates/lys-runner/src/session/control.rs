//! What callers do to running sessions: wait on them, type into them,
//! resize and end them, read their status, and stop them all.

use super::{RUNNER, Sessions, Table, output, unknown};
use crate::error::RunnerError;
use crate::protocol::{Ended, Key, StatusView};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

impl Sessions {
    /// Run `check` on the table each time it changes, until it answers, the
    /// caller leaves, or the runner stops.
    pub(crate) fn until_any<T>(
        &self,
        left: &AtomicBool,
        mut check: impl FnMut(&mut Table) -> Option<T>,
    ) -> Result<T, RunnerError> {
        let mut table = self.lock()?;
        loop {
            if let Some(answer) = check(&mut table) {
                return Ok(answer);
            }
            if left.load(Ordering::SeqCst) {
                return Err(RunnerError::refused(
                    "caller_left",
                    "the caller closed the request",
                ));
            }
            if table.stopping {
                return Err(RunnerError::refused(
                    "runner_stopping",
                    "the runner is stopping",
                ));
            }
            table = self.wait(table)?;
        }
    }

    /// Type `bytes` into session `id`.
    ///
    /// # Errors
    /// - `RunnerError::Refused` if the table is poisoned, the session is unknown or ended, owned input lacks caller context, or the input writer cannot accept the bytes.
    pub fn write(&self, id: &str, bytes: &[u8]) -> Result<(), RunnerError> {
        let writer = {
            let mut table = self.lock()?;
            if table.responsible.contains_key(id) {
                return Err(RunnerError::refused(
                    "SessionInputContextMissing",
                    "owned session input requires a verified caller and live grant judge",
                ));
            }
            let session = table.sessions.get_mut(id).ok_or_else(|| unknown(id))?;
            session.live(id)?.writer.clone()
        };
        writer.write(bytes.to_vec())
    }

    /// Type `text`, then Enter when asked.
    ///
    /// # Errors
    /// - `RunnerError::Refused` if the table is poisoned, the session is unknown or ended, owned input lacks caller context, or the input writer cannot accept the bytes.
    pub fn input(&self, id: &str, text: &str, enter: bool) -> Result<(), RunnerError> {
        self.write(id, &self.input_bytes(id, text, enter)?)
    }

    pub(crate) fn input_bytes(
        &self,
        id: &str,
        text: &str,
        enter: bool,
    ) -> Result<Vec<u8>, RunnerError> {
        let table = self.lock()?;
        let session = table.sessions.get(id).ok_or_else(|| unknown(id))?;
        Ok(crate::input::text_bytes(session, text, enter))
    }

    /// Send `keys`, in order.
    ///
    /// # Errors
    /// - `RunnerError::Refused` if the table is poisoned, the session is unknown or ended, owned input lacks caller context, or the input writer cannot accept the bytes.
    pub fn keys(&self, id: &str, keys: &[Key]) -> Result<(), RunnerError> {
        let bytes: Vec<u8> = keys
            .iter()
            .flat_map(|key| key.bytes().iter().copied())
            .collect();
        self.write(id, &bytes)
    }

    /// Resize session `id`'s terminal.
    ///
    /// # Errors
    /// - `RunnerError::Refused` if the table is poisoned, the session is unknown or ended, the transport has no terminal, the size is zero or resizing fails.
    pub fn resize(&self, id: &str, columns: u16, rows: u16) -> Result<(), RunnerError> {
        let mut table = self.lock()?;
        let session = table.sessions.get_mut(id).ok_or_else(|| unknown(id))?;
        let master = session.live(id)?.master.as_ref().ok_or_else(|| {
            RunnerError::refused(
                "control_transport_unsupported",
                "managed pipes have no terminal to resize",
            )
        })?;
        crate::pty::resize(&**master, columns, rows)?;
        session.columns = columns;
        session.rows = rows;
        Ok(())
    }

    /// Run `check` on session `id` each time its output changes, until it
    /// answers, the caller leaves, or the runner stops.
    pub(crate) fn until<T>(
        &self,
        id: &str,
        left: &AtomicBool,
        check: impl FnMut(&mut output::OutputState, &str) -> Option<Result<T, RunnerError>>,
    ) -> Result<T, RunnerError> {
        let output = Arc::clone(
            &self
                .lock()?
                .sessions
                .get(id)
                .ok_or_else(|| unknown(id))?
                .output,
        );
        output.until(id, left, check)
    }

    /// End session `id`'s process and answer once its exit is seen and the
    /// threads acting for it have let the runner go.
    ///
    /// # Errors
    /// - `RunnerError::Refused` if session/output locking, process signalling or durable-writer completion fails, the session is unknown, or the caller leaves or runner stops before its exit.
    pub fn end(&self, id: &str, left: &AtomicBool) -> Result<Ended, RunnerError> {
        {
            let mut table = self.lock()?;
            let session = table.sessions.get_mut(id).ok_or_else(|| unknown(id))?;
            if session.ended.is_none() {
                session.ending = true;
                if let Some(live) = &session.live {
                    live.end()?;
                }
            }
        }
        let ended = self.until(id, left, |session, _| session.ended().map(Ok))?;
        self.join_workers([id])?;
        self.writer.barrier()?;
        Ok(ended)
    }

    /// What the runner holds: every session, or the one named.
    ///
    /// # Errors
    /// - `RunnerError::Refused` if session/output state cannot be locked or the named session is unknown.
    pub fn status(&self, only: Option<&str>) -> Result<StatusView, RunnerError> {
        let table = self.lock()?;
        let sessions = match only {
            Some(id) => vec![
                table
                    .sessions
                    .get(id)
                    .ok_or_else(|| unknown(id))?
                    .view(id)?,
            ],
            None => table
                .sessions
                .iter()
                .map(|(id, session)| session.view(id))
                .collect::<Result<Vec<_>, _>>()?,
        };
        Ok(StatusView {
            runner: RUNNER.to_owned(),
            protocol: crate::protocol::PROTOCOL_VERSION,
            sessions,
        })
    }

    /// End every running session and answer once each exit is seen and the
    /// threads acting for them have let the runner go; the runner then
    /// starts nothing more.
    ///
    /// # Errors
    /// - `RunnerError::Refused` if table locking or waiting, or durable-writer completion fails.
    /// - `RunnerError::State` if final session persistence fails.
    pub fn stop_all(&self) -> Result<(), RunnerError> {
        let mut table = self.lock()?;
        table.stopping = true;
        for (id, session) in &mut table.sessions {
            if let Err(error) = session.output.stop() {
                crate::error::said(&format!("session {id}: shutdown_wake_failed: {error}"));
            }
            if session.ended.is_none() {
                session.ending = true;
                if let Some(live) = &session.live {
                    if let Err(error) = live.end() {
                        crate::error::said(&format!(
                            "session {id}: shutdown_signal_failed: {error}"
                        ));
                    }
                }
            }
        }
        while !table.starting.is_empty()
            || table
                .sessions
                .values()
                .any(|session| session.ended.is_none())
        {
            table = self.wait(table)?;
        }
        let persisted = self.persist(&table);
        let ended: Vec<String> = table.sessions.keys().cloned().collect();
        drop(table);
        let flushed = self.writer.barrier();
        self.wake();
        let joined = self.join_workers(ended.iter().map(String::as_str));
        persisted.and(flushed).and(joined)
    }
}
