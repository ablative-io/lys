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

pub(crate) mod status;

#[cfg(test)]
#[path = "../tests/collector_binding/cases.rs"]
mod binding_tests;

#[cfg(test)]
#[path = "../tests/rollout_dates/cases.rs"]
mod rollout_tests;

#[cfg(test)]
#[path = "../tests/follower_parent/cases.rs"]
mod parent_tests;

#[cfg(test)]
#[path = "../tests/status_coalescing/cases.rs"]
mod status_tests;

#[cfg(test)]
thread_local! {
    static ROLLOUT_DIRECTORIES: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

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
        let result = match collected {
            Collected::Hook { event, input } => self.hook(id, event, input),
            Collected::StatusLine { input } => return self.status_line(id, input),
            Collected::Notify { notification } => self.notified(id, notification),
        };
        self.writer.barrier()?;
        result
    }

    fn hook(self: &Arc<Self>, id: &str, event: &str, input: &Value) -> Result<String, RunnerError> {
        match event {
            "SessionStart" => self.bind_claude(id, input),
            "UserPromptSubmit" => {
                let mut table = self.lock()?;
                if let Some(session) = table.sessions.get_mut(id) {
                    session.guard.idle = false;
                }
                append(&mut table, id, vec![boundary("turn_start", None)], None)?;
                Ok("a turn began".to_owned())
            }
            "Stop" if input.get("stop_hook_active").and_then(Value::as_bool) == Some(true) => Ok(
                "stop_hook_active: the harness is already continuing from a stop hook, so this is no boundary".to_owned(),
            ),
            "Stop" | "SessionEnd" => {
                self.read_source(id, None);
                let name = if event == "Stop" { "turn_end" } else { "session_end" };
                let mut table = self.lock()?;
                flushed(&mut table, self.runner(), id, name)?;
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
                let mut table = self.lock()?;
                append(&mut table, id, vec![boundary("compacting", None)], None)?;
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
        let mut table = self.lock()?;
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
            append(&mut table, id, vec![Body::Coverage(coverage)], None)?;
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
        if let Err(error) = crate::session::transcript_parent(Path::new(path)) {
            let source = SourceState {
                path: path.to_owned(),
                ..SourceState::default()
            };
            append(
                table,
                id,
                vec![Body::Coverage(Coverage::of(
                    "source_refused",
                    &source,
                    None,
                    error.to_string(),
                ))],
                None,
            )?;
            return Err(error);
        }
        let held = table.feed.source(id).cloned();
        if held.as_ref().is_some_and(|held| held.path == path) {
            return Ok(format!("{path} is already bound"));
        }
        status::flush_status(table, id)?;
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
                    )?;
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
        append(table, id, bodies, Some(source))?;
        self.follow(table, id)?;
        Ok(words)
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
        let lookup = {
            let table = self.lock()?;
            let source = table
                .feed
                .source(id)
                .map(|source| (source.generation, source.bound.clone()));
            if source.as_ref().is_some_and(|(_, bound)| bound == thread) {
                None
            } else {
                let session = table
                    .sessions
                    .get(id)
                    .ok_or_else(|| crate::session::unknown(id))?;
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
        let mut table = self.lock()?;
        flushed_at(&mut table, self.runner(), id, "turn_end", turn)?;
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
pub(crate) fn flushed(
    table: &mut Table,
    runner: &str,
    id: &str,
    name: &str,
) -> Result<(), RunnerError> {
    flushed_at(table, runner, id, name, None)
}

fn flushed_at(
    table: &mut Table,
    runner: &str,
    id: &str,
    name: &str,
    turn: Option<String>,
) -> Result<(), RunnerError> {
    let mut source = table.feed.source(id).cloned();
    let mut bodies = Vec::new();
    if let Some(source) = &mut source {
        if let Some(body) = status::pending(table, id, source)? {
            bodies.push(body);
        }
        if let Some(session) = table.sessions.get(id) {
            if let Some(tracking) = &session.guard.tracking {
                let reading = Reading {
                    runner,
                    session: id,
                    tracking,
                    accounts: accounts(session, tracking),
                    now: now_ms(),
                };
                if let Some(body) = reading.flush(source) {
                    bodies.push(body);
                }
            }
        }
    }
    let commit = if bodies.is_empty() { None } else { source };
    bodies.push(boundary(name, turn));
    append(table, id, bodies, commit)?;
    status::clear(table, id);
    Ok(())
}

/// The directory name Claude Code keeps a working directory's sessions
/// under: every character not an ASCII letter or digit made `-`.
pub fn slug(cwd: &str) -> String {
    cwd.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect()
}

/// Locate a rollout created on the current local date.
/// Use [`rollout_since`] when the session may span several dates.
///
/// # Errors
/// Refuses an unreadable rollout, mismatched metadata, or an absent thread.
pub fn rollout(home: &Path, thread: &str) -> Result<std::path::PathBuf, RunnerError> {
    let now = now_ms();
    rollout_since(home, thread, now, now)
}

/// Locate a rollout within the session's launch-to-notification interval.
/// Date folders use the machine's local calendar, as the harness does.
///
/// # Errors
/// Refuses reversed instants, unreadable rollouts, or an absent thread.
pub fn rollout_since(
    home: &Path,
    thread: &str,
    launched: u64,
    notified: u64,
) -> Result<std::path::PathBuf, RunnerError> {
    if !plain(thread) {
        return Err(RunnerError::refused(
            "notify_malformed",
            "the thread id is invalid",
        ));
    }
    if launched > notified {
        return Err(RunnerError::refused(
            "rollout_date_invalid",
            "the launch follows the notification",
        ));
    }
    let refused = |error: &dyn std::fmt::Display| {
        RunnerError::refused("rollout_date_invalid", error.to_string())
    };
    let zone = jiff::tz::TimeZone::try_system().map_err(|error| refused(&error))?;
    let date = |instant: u64| -> Result<jiff::civil::Date, RunnerError> {
        let millis = i64::try_from(instant).map_err(|error| refused(&error))?;
        Ok(jiff::Timestamp::from_millisecond(millis)
            .map_err(|error| refused(&error))?
            .to_zoned(zone.clone())
            .date())
    };
    let mut first = date(launched)?;
    let last = date(notified)?;
    let root = home.join("sessions");
    let metadata = std::fs::metadata(&root).map_err(|error| rollout_unreadable(&root, &error))?;
    if !metadata.is_dir() {
        return Err(rollout_unreadable(&root, &"not a directory"));
    }
    loop {
        let dir = root.join(format!(
            "{:04}/{:02}/{:02}",
            first.year(),
            first.month(),
            first.day()
        ));
        if let Some(path) = rollout_on(&dir, thread)? {
            return rollout_metadata(path, thread);
        }
        if first >= last {
            break;
        }
        first = first.tomorrow().map_err(|error| refused(&error))?;
    }
    Err(RunnerError::refused(
        "transcript_unbound",
        format!(
            "no rollout of thread {thread} is in the session's date interval under {}",
            root.display()
        ),
    ))
}

fn rollout_unreadable(path: &Path, error: &dyn std::fmt::Display) -> RunnerError {
    RunnerError::refused("rollout_unreadable", format!("{}: {error}", path.display()))
}

fn rollout_on(dir: &Path, thread: &str) -> Result<Option<std::path::PathBuf>, RunnerError> {
    #[cfg(test)]
    ROLLOUT_DIRECTORIES.with(|count| count.set(count.get() + 1));
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(rollout_unreadable(dir, &error)),
    };
    let ending = format!("-{thread}.jsonl");
    for entry in entries {
        let entry = entry.map_err(|error| rollout_unreadable(dir, &error))?;
        let name = entry.file_name();
        let bytes = name.as_encoded_bytes();
        if bytes.starts_with(b"rollout-") && bytes.ends_with(ending.as_bytes()) {
            if !entry
                .file_type()
                .map_err(|error| rollout_unreadable(&entry.path(), &error))?
                .is_file()
            {
                return Err(rollout_unreadable(
                    &entry.path(),
                    &"the rollout is not a regular file",
                ));
            }
            return Ok(Some(entry.path()));
        }
    }
    Ok(None)
}

fn rollout_metadata(
    path: std::path::PathBuf,
    thread: &str,
) -> Result<std::path::PathBuf, RunnerError> {
    use std::io::{BufRead, Read};
    let file = std::fs::File::open(&path).map_err(|error| rollout_unreadable(&path, &error))?;
    let mut first = String::new();
    std::io::BufReader::new(file.take(1_048_577))
        .read_line(&mut first)
        .map_err(|error| rollout_unreadable(&path, &error))?;
    if first.len() > 1_048_576 || !first.ends_with('\n') {
        return Err(rollout_unreadable(
            &path,
            &"the first line exceeds its limit or is incomplete",
        ));
    }
    let meta: Value = serde_json::from_str(first.trim_end()).map_err(|error| {
        rollout_unreadable(&path, &format!("the first line is not JSON: {error}"))
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
