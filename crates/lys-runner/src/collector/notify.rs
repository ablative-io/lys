//! Native thread notifications retain identity beside the accounting source.

use std::path::Path;
use std::sync::Arc;

use serde_json::Value;

use super::{flushed_at, rollout_since, text};
use crate::error::RunnerError;
use crate::session::{Sessions, now_ms};
use crate::tracking::Harness;

impl Sessions {
    /// Keep a Codex after-turn notification: bind its thread's rollout the
    /// first time, read it, then keep the turn's end.
    pub(super) fn notified(
        self: &Arc<Self>,
        id: &str,
        notification: &Value,
    ) -> Result<String, RunnerError> {
        if text(notification, "type") != Some("agent-turn-complete") {
            return Ok("not a turn's end: nothing is kept".to_owned());
        }
        let thread = text(notification, "thread-id").ok_or_else(|| {
            RunnerError::refused("notify_malformed", "the notification names no thread")
        })?;
        crate::harness_control::Transport::Codex
            .validate_resume(thread)
            .map_err(|error| {
                RunnerError::refused(
                    "notify_malformed",
                    format!("the notification's thread id is not a Codex UUID: {error}"),
                )
            })?;
        let turn = text(notification, "turn-id").map(str::to_owned);
        let lookup = {
            let table = self.lock()?;
            let source = table
                .feed
                .source(id)
                .map(|source| (source.generation, source.bound.clone()));
            let session = table
                .sessions
                .get(id)
                .ok_or_else(|| crate::session::unknown(id))?;
            let proxied = table
                .feed
                .source(id)
                .is_some_and(|source| session.guard.proxy_source(source).is_some());
            if let Some(source) = table.feed.source(id) {
                source.check_native_conversation(thread)?;
                if !proxied && !source.bound.is_empty() && source.bound != thread {
                    return Err(RunnerError::refused(
                        "native_conversation_changed",
                        "the notification names a different thread than the bound native source",
                    ));
                }
            }
            if proxied || source.as_ref().is_some_and(|(_, bound)| bound == thread) {
                None
            } else {
                let tracking = session
                    .guard
                    .tracking
                    .as_ref()
                    .filter(|tracking| tracking.harness == Harness::Codex)
                    .ok_or_else(|| {
                        RunnerError::refused(
                            "transcript_unbound",
                            format!("session {id} is not a tracked Codex session"),
                        )
                    })?;
                Some((
                    tracking.config_home.clone(),
                    session.started_at,
                    session.guard.leader.clone(),
                    source,
                ))
            }
        };
        if let Some((home, launched, leader, before)) = lookup {
            let path = rollout_since(Path::new(&home), thread, launched, now_ms())?
                .display()
                .to_string();
            let mut table = self.lock()?;
            let session = table
                .sessions
                .get(id)
                .ok_or_else(|| crate::session::unknown(id))?;
            if session.started_at != launched
                || session.guard.leader != leader
                || session
                    .guard
                    .tracking
                    .as_ref()
                    .is_none_or(|tracking| tracking.config_home != home)
            {
                return Err(RunnerError::refused(
                    "rollout_session_changed",
                    "the launch changed while locating its rollout",
                ));
            }
            let current = table
                .feed
                .source(id)
                .map(|source| (source.generation, source.bound.clone()));
            if current.as_ref().is_none_or(|(_, bound)| bound != thread) {
                if current != before {
                    return Err(RunnerError::refused(
                        "source_changed",
                        "the bound source changed while locating its rollout",
                    ));
                }
                self.bind(&mut table, id, (&path, thread), true)?;
            }
        }
        self.read_source(id, None);
        self.finish_notification(id, turn, thread)
    }

    fn finish_notification(
        &self,
        id: &str,
        turn: Option<String>,
        thread: &str,
    ) -> Result<String, RunnerError> {
        let mut table = self.lock()?;
        flushed_at(
            &mut table,
            self.runner(),
            id,
            "turn_end",
            turn,
            Some(thread),
        )?;
        if table
            .sessions
            .get(id)
            .is_none_or(|session| session.managed.is_none())
        {
            if let Some(session) = table.sessions.get_mut(id) {
                session.guard.idle = true;
            }
            crate::operations::deliver(&mut table, id);
        }
        drop(table);
        self.wake();
        Ok("the turn's end kept".to_owned())
    }
}
