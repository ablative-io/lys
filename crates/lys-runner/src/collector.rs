//! The collector: what a proved session's harness signalled through its
//! hooks, its status line and its notices, kept in the proved session's name.
//!
//! A hook's prompt text and a notice's message text are never kept.

use std::path::Path;
use std::sync::Arc;

use serde_json::Value;

use crate::error::RunnerError;
use crate::peer::Collected;
use crate::session::{Sessions, Table, accounts, append, now_ms};
use crate::tracking::{Harness, Reading};
use crate::tracking_store::{Body, Boundary, Coverage, SourceState};

#[cfg(test)]
#[path = "../tests/collector_binding/cases.rs"]
mod binding_tests;

fn text<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn plain(id: &str) -> bool {
    !id.is_empty()
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

fn boundary(name: &str, turn: Option<String>) -> Body {
    Body::Boundary(Boundary {
        boundary: name.to_owned(),
        turn,
    })
}

impl Sessions {
    /// Keep what session `id`'s harness signalled, answering in words what
    /// was kept.
    pub fn collect(
        self: &Arc<Self>,
        id: &str,
        collected: &Collected,
    ) -> Result<String, RunnerError> {
        match collected {
            Collected::Hook { event, input } => self.hook(id, event, input),
            Collected::StatusLine { input } => self.status_line(id, input),
            Collected::Notify { notification } => self.notified(id, notification),
        }
    }

    fn hook(self: &Arc<Self>, id: &str, event: &str, input: &Value) -> Result<String, RunnerError> {
        match event {
            "SessionStart" => self.bind_claude(id, input),
            "UserPromptSubmit" => {
                let mut table = self.lock();
                if let Some(session) = table.sessions.get_mut(id) {
                    session.guard.idle = false;
                }
                append(&mut table, id, vec![boundary("turn_start", None)], None);
                Ok("a turn began".to_owned())
            }
            "Stop" if input.get("stop_hook_active").and_then(Value::as_bool) == Some(true) => Ok(
                "stop_hook_active: the harness is already continuing from a stop hook, so this is no boundary".to_owned(),
            ),
            "Stop" | "SessionEnd" => {
                self.read_source(id, None);
                let name = if event == "Stop" { "turn_end" } else { "session_end" };
                let mut table = self.lock();
                flushed(&mut table, self.runner(), id, name);
                if event == "Stop" {
                    if let Some(session) = table.sessions.get_mut(id) {
                        session.guard.idle = true;
                    }
                    crate::operations::deliver(&mut table, id);
                }
                drop(table);
                self.wake();
                Ok(format!("{name} kept"))
            }
            "PreCompact" => {
                let mut table = self.lock();
                append(&mut table, id, vec![boundary("compacting", None)], None);
                crate::operations::compacting(&mut table, id);
                drop(table);
                self.wake();
                Ok("compacting kept".to_owned())
            }
            "PostToolUse" | "PostToolUseFailure" => Ok(
                "an outcome observation: it is no usage and no policy refusal, and nothing is kept".to_owned(),
            ),
            other => Err(RunnerError::refused(
                "hook_unknown",
                format!("the collector keeps no hook {other}"),
            )),
        }
    }

    /// Bind session `id` to the transcript a `SessionStart` names, once it
    /// is the file its isolated configuration home keeps for that session.
    fn bind_claude(self: &Arc<Self>, id: &str, input: &Value) -> Result<String, RunnerError> {
        let unbound = |words: String| RunnerError::refused("transcript_unbound", words);
        let claude = text(input, "session_id").filter(|claude| plain(claude));
        let given = text(input, "transcript_path");
        let (Some(claude), Some(given)) = (claude, given) else {
            return Err(unbound(
                "the hook names no session id and transcript".to_owned(),
            ));
        };
        let mut table = self.lock();
        let session = table
            .sessions
            .get(id)
            .ok_or_else(|| crate::session::unknown(id))?;
        let tracking = session
            .guard
            .tracking
            .as_ref()
            .filter(|tracking| tracking.harness == Harness::ClaudeCode)
            .ok_or_else(|| unbound(format!("session {id} is not a tracked Claude Code session")))?;
        let launched = session
            .launch
            .as_ref()
            .map(|launch| launch.directory.clone());
        let expected = [Some(session.guard.cwd.clone()), launched]
            .into_iter()
            .flatten()
            .map(|dir| {
                Path::new(&tracking.config_home)
                    .join("projects")
                    .join(slug(&dir))
                    .join(format!("{claude}.jsonl"))
            })
            .any(|path| path == Path::new(given));
        if !expected {
            let words = format!(
                "{given} is not the transcript the session's configuration home keeps for {claude}"
            );
            let refused = SourceState {
                path: given.to_owned(),
                ..SourceState::default()
            };
            let coverage = Coverage::of("source_refused", &refused, None, words.clone());
            append(&mut table, id, vec![Body::Coverage(coverage)], None);
            return Err(unbound(words));
        }
        self.bind(&mut table, id, (given, claude), false)
    }

    /// Bind session `id` to the stream at `path` as the harness's `bound`
    /// session or thread, from its start or its present end, and follow it.
    fn bind(
        self: &Arc<Self>,
        table: &mut Table,
        id: &str,
        (path, bound): (&str, &str),
        from_start: bool,
    ) -> Result<String, RunnerError> {
        let held = table.feed.source(id).cloned();
        if held.as_ref().is_some_and(|held| held.path == path) {
            return Ok(format!("{path} is already bound"));
        }
        let offset = if from_start {
            0
        } else {
            match std::fs::metadata(path) {
                Ok(metadata) => metadata.len(),
                Err(error) => {
                    let words = format!("{path} cannot be read: {error}");
                    let refused = SourceState {
                        path: path.to_owned(),
                        ..SourceState::default()
                    };
                    append(
                        table,
                        id,
                        vec![Body::Coverage(Coverage::of(
                            "source_refused",
                            &refused,
                            None,
                            words.clone(),
                        ))],
                        None,
                    );
                    return Err(RunnerError::refused("transcript_unreadable", words));
                }
            }
        };
        let source = SourceState {
            path: path.to_owned(),
            generation: held.map_or(0, |held| held.generation + 1),
            offset,
            bound: bound.to_owned(),
            ..SourceState::default()
        };
        let words = format!(
            "following {path} from byte {offset}: what it held before the session bound it is not this session's"
        );
        let bodies = vec![
            Body::Coverage(Coverage::of(
                "source_bound",
                &source,
                Some(offset),
                words.clone(),
            )),
            boundary("session_start", None),
        ];
        append(table, id, bodies, Some(source));
        self.follow(table, id);
        Ok(words)
    }

    fn status_line(&self, id: &str, input: &Value) -> Result<String, RunnerError> {
        let mut guard = self.lock();
        let table = &mut *guard;
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
        let reading = Reading {
            runner: self.runner(),
            session: id,
            tracking,
            accounts: accounts(session, tracking),
            now: now_ms(),
        };
        let record = format!("status-line:{id}:{}", table.feed.end());
        let Some(body) = reading.status(&mut source, input, record) else {
            return Ok("the snapshot repeats the last one kept".to_owned());
        };
        append(table, id, vec![body], Some(source));
        drop(guard);
        self.wake();
        Ok("a context snapshot kept; it adds no spend".to_owned())
    }

    /// Keep a Codex after-turn notification: bind its thread's rollout the
    /// first time, read it, then keep the turn's end.
    fn notified(self: &Arc<Self>, id: &str, notification: &Value) -> Result<String, RunnerError> {
        if text(notification, "type") != Some("agent-turn-complete") {
            return Ok("not a turn's end: nothing is kept".to_owned());
        }
        let thread = text(notification, "thread-id")
            .filter(|thread| plain(thread))
            .ok_or_else(|| {
                RunnerError::refused("notify_malformed", "the notification names no thread")
            })?;
        let turn = text(notification, "turn-id").map(str::to_owned);
        let mut table = self.lock();
        if table
            .feed
            .source(id)
            .is_none_or(|source| source.bound != thread)
        {
            let home = table
                .sessions
                .get(id)
                .and_then(|session| session.guard.tracking.as_ref())
                .filter(|tracking| tracking.harness == Harness::Codex)
                .map(|tracking| tracking.config_home.clone())
                .ok_or_else(|| {
                    RunnerError::refused(
                        "transcript_unbound",
                        format!("session {id} is not a tracked Codex session"),
                    )
                })?;
            let path = rollout(Path::new(&home), thread)?.display().to_string();
            self.bind(&mut table, id, (&path, thread), true)?;
        }
        drop(table);
        self.read_source(id, None);
        let mut table = self.lock();
        append(&mut table, id, vec![boundary("turn_end", turn)], None);
        if let Some(session) = table.sessions.get_mut(id) {
            session.guard.idle = true;
        }
        crate::operations::deliver(&mut table, id);
        drop(table);
        self.wake();
        Ok("the turn's end kept".to_owned())
    }
}

/// Keep boundary `name` for session `id`, with the response its stream
/// held pending, now shown whole.
fn flushed(table: &mut Table, runner: &str, id: &str, name: &str) {
    let source = table.feed.source(id).cloned();
    let pending = table.sessions.get(id).and_then(|session| {
        let tracking = session.guard.tracking.as_ref()?;
        let mut source = source.clone()?;
        let reading = Reading {
            runner,
            session: id,
            tracking,
            accounts: accounts(session, tracking),
            now: now_ms(),
        };
        let body = reading.flush(&mut source)?;
        Some((body, source))
    });
    match pending {
        Some((body, source)) => append(table, id, vec![body, boundary(name, None)], Some(source)),
        None => append(table, id, vec![boundary(name, None)], None),
    }
}

/// The directory name Claude Code keeps a working directory's sessions
/// under: every character not an ASCII letter or digit made `-`.
pub fn slug(cwd: &str) -> String {
    cwd.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect()
}

/// The rollout Codex keeps for `thread` under `home`'s `sessions/`, once
/// its first record, `session_meta`, names that thread; refused
/// `rollout_thread_mismatch` when it names another, and
/// `transcript_unbound` when there is none.
pub fn rollout(home: &Path, thread: &str) -> Result<std::path::PathBuf, RunnerError> {
    let ending = format!("-{thread}.jsonl");
    let mut dirs = vec![home.join("sessions")];
    let mut found = None;
    while let Some(dir) = dirs.pop() {
        let entries = std::fs::read_dir(&dir).map_err(|error| {
            RunnerError::refused("rollout_unreadable", format!("{}: {error}", dir.display()))
        })?;
        for entry in entries {
            let entry = entry.map_err(|error| {
                RunnerError::refused("rollout_unreadable", format!("{}: {error}", dir.display()))
            })?;
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            if path.is_dir() {
                dirs.push(path);
            } else if name.starts_with("rollout-") && name.ends_with(&ending) {
                found = Some(path);
            }
        }
    }
    let path = found.ok_or_else(|| {
        RunnerError::refused(
            "transcript_unbound",
            format!("no rollout of thread {thread} is under {}", home.display()),
        )
    })?;
    let mut first = String::new();
    std::fs::File::open(&path)
        .and_then(|file| {
            std::io::BufRead::read_line(&mut std::io::BufReader::new(file), &mut first)
        })
        .map_err(|error| RunnerError::refused("transcript_unbound", error.to_string()))?;
    let meta: Value = serde_json::from_str(first.trim_end()).map_err(|error| {
        RunnerError::refused(
            "rollout_unreadable",
            format!("the first line is not JSON: {error}"),
        )
    })?;
    let named = meta
        .get("payload")
        .and_then(|payload| payload.get("id"))
        .and_then(Value::as_str);
    if text(&meta, "type") == Some("session_meta") && named == Some(thread) {
        Ok(path)
    } else {
        Err(RunnerError::refused(
            "rollout_thread_mismatch",
            format!(
                "{}'s session_meta names thread {}, not {thread}",
                path.display(),
                named.unwrap_or("none")
            ),
        ))
    }
}
