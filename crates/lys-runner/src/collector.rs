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
use crate::tracking::Reading;
use crate::tracking_store::{Body, Boundary, Coverage, SourceState};

mod hooks;
mod notify;
pub(crate) mod status;
mod stop;

#[cfg(test)]
#[path = "../tests/collector_tracking.rs"]
mod tracking_tests;

#[cfg(test)]
#[path = "../tests/collector_native/cases.rs"]
mod native_tests;

#[cfg(test)]
#[path = "../tests/collector_wire.rs"]
mod wire_tests;

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
#[path = "../tests/status_proxied/cases.rs"]
mod status_proxied_tests;

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
        self.signalled(id)?;
        let result = match collected {
            Collected::Hook { event, input } => self.hook(id, event, input),
            Collected::StatusLine { input } => return self.status_line(id, input),
            Collected::Notify { notification } => self.notified(id, notification),
        };
        self.writer.barrier()?;
        if matches!(collected, Collected::Hook { .. }) && result.is_ok() {
            // An accepted hook moves the owner's durable hook cursor once,
            // under an intent derived from the cursor it reaches, so a lost
            // reply never charges it twice (AGENTS-004 R2).
            self.advance_hook_cursor(id)?;
        }
        result
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
            generation: held.as_ref().map_or(0, |held| held.generation + 1),
            offset,
            bound: bound.to_owned(),
            native_conversation: held
                .filter(|held| held.bound == bound)
                .and_then(|held| held.native_conversation),
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
}

/// Keep boundary `name` for session `id`, with the response its stream
/// held pending, now shown whole.
pub(crate) fn flushed(
    table: &mut Table,
    runner: &str,
    id: &str,
    name: &str,
) -> Result<(), RunnerError> {
    flushed_at(table, runner, id, name, None, None)
}

fn flushed_at(
    table: &mut Table,
    runner: &str,
    id: &str,
    name: &str,
    turn: Option<String>,
    native_conversation: Option<&str>,
) -> Result<(), RunnerError> {
    let mut source = table.feed.source(id).cloned();
    let mut native_changed = false;
    if let Some(conversation) = native_conversation {
        let source = source.as_mut().ok_or_else(|| {
            RunnerError::refused(
                "transcript_unbound",
                "the native conversation has no bound source to keep its identity",
            )
        })?;
        source.check_native_conversation(conversation)?;
        native_changed = source.native_conversation.as_deref() != Some(conversation);
        source.native_conversation = Some(conversation.to_owned());
    }
    let mut bodies = Vec::new();
    if let Some(source) = &mut source {
        if let Some(body) = status::pending(table, id, source)? {
            bodies.push(body);
        }
        if let Some(session) = table.sessions.get(id) {
            if session.guard.proxy_source(source).is_none()
                && let Some(tracking) = &session.guard.tracking
            {
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
    let commit = if bodies.is_empty() && !native_changed {
        None
    } else {
        source
    };
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
    use std::io::BufRead;
    let file = std::fs::File::open(&path).map_err(|error| rollout_unreadable(&path, &error))?;
    let mut first = String::new();
    std::io::BufReader::new(file)
        .read_line(&mut first)
        .map_err(|error| rollout_unreadable(&path, &error))?;
    if !first.ends_with('\n') {
        return Err(rollout_unreadable(&path, &"the first line is incomplete"));
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
