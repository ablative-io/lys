//! What callers do to running sessions: wait on them, type into them,
//! resize and end them, read their status, and stop them all.

use super::*;
use crate::admitted::Admitted;
use crate::error::RunnerError;
use crate::input::Input;
use crate::judge::Policy;
use crate::operations::Operations;
use crate::peer::Leader;
use crate::protocol::{Ended, EndedHow, Key, Launch, SessionView, StatusView};
use crate::refusals::Desk;
use crate::rotation::RotationState;
use crate::state::{Kept, KeptSession, StateFile};
use crate::tracking::Tracking;
use crate::tracking_store::Feed;
use portable_pty::MasterPty;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, Weak, mpsc};
use std::time::{SystemTime, UNIX_EPOCH};

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
    pub fn write(&self, id: &str, bytes: &[u8]) -> Result<(), RunnerError> {
        let writer = {
            let mut table = self.lock()?;
            let session = table.sessions.get_mut(id).ok_or_else(|| unknown(id))?;
            session.live(id)?.writer.clone()
        };
        writer.write(bytes.to_vec())
    }

    /// Type `text`, then Enter when asked.
    pub fn input(&self, id: &str, text: &str, enter: bool) -> Result<(), RunnerError> {
        let mut bytes = text.as_bytes().to_vec();
        if enter {
            bytes.extend_from_slice(Key::Enter.bytes());
        }
        self.write(id, &bytes)
    }

    /// Send `keys`, in order.
    pub fn keys(&self, id: &str, keys: &[Key]) -> Result<(), RunnerError> {
        let bytes: Vec<u8> = keys
            .iter()
            .flat_map(|key| key.bytes().iter().copied())
            .collect();
        self.write(id, &bytes)
    }

    /// Resize session `id`'s terminal.
    pub fn resize(&self, id: &str, columns: u16, rows: u16) -> Result<(), RunnerError> {
        let mut table = self.lock()?;
        let session = table.sessions.get_mut(id).ok_or_else(|| unknown(id))?;
        crate::pty::resize(&*session.live(id)?.master, columns, rows)?;
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

    /// End session `id`'s process and answer once its exit is seen.
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
        self.writer.barrier()?;
        Ok(ended)
    }

    /// What the runner holds: every session, or the one named.
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

    /// End every running session and answer once each exit is seen; the
    /// runner then starts nothing more.
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
        drop(table);
        let flushed = self.writer.barrier();
        self.wake();
        persisted.and(flushed)
    }
}
