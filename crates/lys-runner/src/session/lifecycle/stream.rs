//! A tracked session's harness stream: followed on a thread of its own,
//! woken by change notices and never a timer, read from its saved offset to
//! its last whole line and kept with its coverage.

use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;
use std::sync::{Arc, mpsc};

use serde_json::Value;

use super::super::{Session, Sessions, Table, now_ms};
use super::{Wake, transcript_parent, window_limit};
use crate::error::RunnerError;
use crate::tracking::{Accounts, Harness, Reading, Tracking};
use crate::tracking_proxy::{PROXY_ADAPTER, ProxyReading};
use crate::tracking_store::{Body, Commit, Coverage, SourceState};

impl Sessions {
    /// Bind session `id` to its run's usage file under the proxy's state and
    /// follow it. Nothing here refuses the run: a proxy state this runner
    /// was not told of, or a usage directory that cannot be made or watched,
    /// is said in the session's coverage.
    pub(crate) fn follow_proxy(self: &Arc<Self>, table: &mut Table, id: &str) {
        let Some(proxy) = table
            .sessions
            .get(id)
            .and_then(|session| session.guard.proxy.clone())
        else {
            return;
        };
        let held = table.feed.source(id).cloned();
        let unfollowed = |table: &mut Table, words: String| {
            crate::error::said(&format!("session {id}: coverage_incomplete: {words}"));
            let source = held.clone().unwrap_or_default();
            let coverage = Coverage {
                adapter: Some(PROXY_ADAPTER.to_owned()),
                ..Coverage::of("coverage_incomplete", &source, None, words)
            };
            if let Err(error) = append(table, id, vec![Body::Coverage(coverage)], None) {
                crate::error::said(&format!("session {id}: coverage_record_failed: {error}"));
            }
        };
        let Some(dir) = self.proxy_usage.get() else {
            unfollowed(
                table,
                "this runner was not told where the proxy keeps its state, so the run's model calls are not counted"
                    .to_owned(),
            );
            return;
        };
        if let Err(error) = std::fs::create_dir_all(dir) {
            unfollowed(table, format!("{} cannot be made: {error}", dir.display()));
            return;
        }
        let source = SourceState {
            path: proxy.file(dir).display().to_string(),
            generation: held.as_ref().map_or(0, |held| held.generation + 1),
            bound: proxy.run.clone(),
            ..SourceState::default()
        };
        let words = format!(
            "following {} from its start: the proxy writes one line there for each finished call of run {}",
            source.path, proxy.run
        );
        let bound = Coverage {
            adapter: Some(PROXY_ADAPTER.to_owned()),
            ..Coverage::of("source_bound", &source, Some(0), words)
        };
        if append(table, id, vec![Body::Coverage(bound)], Some(source)).is_err() {
            // `append` has said why and counted the gap.
            return;
        }
        if let Err(error) = self.follow(table, id) {
            crate::error::said(&format!(
                "session {id}: its usage file is not followed: {error}"
            ));
        }
    }

    /// Follow session `id`'s bound stream on a thread of its own, stopping
    /// any follower it had.
    pub(crate) fn follow(self: &Arc<Self>, table: &mut Table, id: &str) -> Result<(), RunnerError> {
        let Some(path) = table
            .feed
            .source(id)
            .map(|source| PathBuf::from(&source.path))
        else {
            return Ok(());
        };
        let Some(session) = table.sessions.get_mut(id) else {
            return Ok(());
        };
        if let Some(old) = session.follower.take() {
            stop_follower(id, &old);
        }
        let (wake, woken) = mpsc::channel();
        let notices = wake.clone();
        let notifier = notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
            let next = match event {
                Ok(event) if event.need_rescan() => {
                    Wake::Lost("the notifier asked for a rescan".to_owned())
                }
                Ok(_) => Wake::Changed,
                Err(error) => Wake::Lost(error.to_string()),
            };
            if let Err(gone) = notices.send(next) {
                crate::error::said(&format!("a stream follower had ended: {gone}"));
            }
        });
        let dir = match transcript_parent(&path) {
            Ok(dir) => dir,
            Err(error) => {
                crate::error::said(&format!("session {id}: {error}"));
                let source = table.feed.source(id).cloned().unwrap_or_default();
                let coverage = Coverage::of("source_refused", &source, None, error.to_string());
                if let Err(error) = append(table, id, vec![Body::Coverage(coverage)], None) {
                    crate::error::said(&format!("session {id}: coverage_record_failed: {error}"));
                }
                return Err(error);
            }
        };
        let watching = notifier.and_then(|mut each| {
            notify::Watcher::watch(&mut each, &dir, notify::RecursiveMode::NonRecursive)
                .map(|()| each)
        });
        let watcher = match watching {
            Ok(watcher) => watcher,
            Err(error) => {
                let words = format!("{} cannot be watched: {error}", dir.display());
                crate::error::said(&format!("session {id}: coverage_incomplete: {words}"));
                let source = table.feed.source(id).cloned().unwrap_or_default();
                let coverage = Coverage::of("coverage_incomplete", &source, None, words.clone());
                if let Err(error) = append(table, id, vec![Body::Coverage(coverage)], None) {
                    crate::error::said(&format!("session {id}: coverage_record_failed: {error}"));
                }
                return Err(RunnerError::refused("transcript_watch_failed", words));
            }
        };
        let (sessions, owned) = (Arc::clone(self), id.to_owned());
        let follower = std::thread::Builder::new()
            .name("runner-transcript".to_owned())
            .spawn(move || {
                sessions.read_source(&owned, None);
                if sessions.table.is_poisoned() {
                    return;
                }
                for next in woken {
                    match next {
                        Wake::Changed => sessions.read_source(&owned, None),
                        Wake::Lost(reason) => sessions.read_source(&owned, Some(&reason)),
                        Wake::Stop => break,
                    }
                    if sessions.table.is_poisoned() {
                        break;
                    }
                }
                drop(watcher);
            })
            .map_err(|error| RunnerError::refused("transcript_worker_failed", error.to_string()))?;
        session.follower = Some(wake);
        drop(follower);
        Ok(())
    }

    /// Read session `id`'s stream from its saved cursor to its last whole
    /// line, and keep what it yields with the new cursor as one unit.
    pub(crate) fn read_source(&self, id: &str, lost: Option<&str>) {
        loop {
            let Some(table) = self.lock_logged() else {
                return;
            };
            let Some(session) = table.sessions.get(id) else {
                return;
            };
            let tracking = session.guard.tracking.clone();
            let proxy = session.guard.proxy.clone();
            let Some(mut source) = table.feed.source(id).cloned() else {
                return;
            };
            let (current, moves) = tracking.as_ref().map_or((None, Vec::new()), |tracking| {
                let evidence = accounts(session, tracking);
                (evidence.current.map(str::to_owned), evidence.moves.to_vec())
            });
            drop(table);
            let now = now_ms();
            let harness = tracking.as_ref().map(|tracking| Reading {
                runner: self.state.runner(),
                session: id,
                tracking,
                accounts: Accounts {
                    current: current.as_deref(),
                    moves: &moves,
                    declared: tracking.account.as_deref(),
                },
                now,
            });
            let proxied = proxy.as_ref().map(|tracking| ProxyReading {
                runner: self.state.runner(),
                session: id,
                tracking,
                now,
            });
            // A session has one stream: its harness's when its harness is
            // tracked, else its run's usage file under the proxy's state.
            let lines = match (&harness, &proxied) {
                (Some(reading), _) => Lines::Harness(reading),
                (None, Some(reading)) => Lines::Proxy(reading),
                (None, None) => return,
            };
            let mut bodies = Vec::new();
            if let Some(reason) = &lost {
                bodies.push(Body::Coverage(Coverage::of(
                "coverage_incomplete",
                &source,
                Some(source.offset),
                format!("change notices were lost ({reason}): the stream is read again from its saved cursor"),
            )));
            }
            let before = source.clone();
            let more = read_lines(lines, &mut source, &mut bodies);
            if bodies.is_empty() && source == before {
                return;
            }
            let Some(mut table) = self.lock_logged() else {
                return;
            };
            if table.feed.source(id) != Some(&before) {
                drop(table);
                continue;
            }
            if source.generation != before.generation {
                if let Err(error) = crate::collector::status::flush_status(&mut table, id) {
                    crate::error::said(&format!(
                        "session {id}: source_status_record_failed: {error}"
                    ));
                    return;
                }
                if let Some(committed) = table.feed.source(id) {
                    source.snapshot.clone_from(&committed.snapshot);
                    source
                        .snapshot_account
                        .clone_from(&committed.snapshot_account);
                    source.reported_cost_micros = committed.reported_cost_micros;
                }
            }
            let commit = Commit {
                source: Some(source),
                attempt: None,
            };
            let leader = window_limit(&mut table, id, &bodies);
            let appended = table.feed.append(id, now_ms(), bodies, commit);
            drop(table);
            if let Err(error) = appended.and_then(|_| self.writer.barrier()) {
                crate::error::said(&format!(
                    "session {id}: coverage_incomplete: what its stream yielded was not kept, and is read again from the saved cursor: {error}"
                ));
                return;
            }
            if let Some(leader) = leader {
                if let Err(error) = crate::pty::end(&leader) {
                    crate::error::said(&format!("session {id}: rotation_signal_failed: {error}"));
                }
            }
            self.wake();
            if !more {
                return;
            }
        }
    }
}

/// Stop the follower `follower` of session `id`.
pub(super) fn stop_follower(id: &str, follower: &mpsc::Sender<Wake>) {
    if follower.send(Wake::Stop).is_err() {
        crate::error::said(&format!(
            "session {id}: its stream follower had already ended"
        ));
    }
}

/// Keep `bodies` for session `id` as one unit, with `source` when given.
pub(crate) fn append(
    table: &mut Table,
    id: &str,
    bodies: Vec<Body>,
    source: Option<SourceState>,
) -> Result<(), RunnerError> {
    let commit = Commit {
        source,
        attempt: None,
    };
    if let Err(error) = table.feed.append(id, now_ms(), bodies, commit) {
        let gap = table.gaps.entry(id.to_owned()).or_default();
        gap.lost += 1;
        gap.since.get_or_insert(now_ms());
        gap.words = format!("coverage_incomplete: {error}");
        crate::error::said(&format!("session {id}: coverage_incomplete: {error}"));
        return Err(error);
    }
    Ok(())
}

/// The rotation evidence of `session`.
pub(crate) fn accounts<'a>(session: &'a Session, tracking: &'a Tracking) -> Accounts<'a> {
    Accounts {
        current: session
            .rotation
            .as_ref()
            .map(crate::rotation::RotationState::account),
        moves: session
            .rotation
            .as_ref()
            .map(crate::rotation::RotationState::moves)
            .unwrap_or_default(),
        declared: tracking.account.as_deref(),
    }
}

/// What a stream's lines are read with: a harness's adapter, or the proxy's
/// usage file reader.
#[derive(Clone, Copy)]
enum Lines<'a> {
    Harness(&'a Reading<'a>),
    Proxy(&'a ProxyReading<'a>),
}

/// Read `source` from its offset to its last whole line into `bodies`.
fn read_lines(lines: Lines<'_>, source: &mut SourceState, bodies: &mut Vec<Body>) -> bool {
    let opened = std::fs::File::open(&source.path).and_then(|file| {
        let metadata = file.metadata()?;
        Ok((file, metadata))
    });
    let (mut file, metadata) = match opened {
        Ok(opened) => opened,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return false,
        Err(error) => {
            bodies.push(Body::Coverage(Coverage::of(
                "source_refused",
                source,
                Some(source.offset),
                format!("the stream cannot be read: {error}"),
            )));
            return false;
        }
    };
    let identity = format!("{}:{}", metadata.dev(), metadata.ino());
    let replaced = source
        .identity
        .as_ref()
        .is_some_and(|held| *held != identity);
    if replaced || metadata.len() < source.offset {
        let words = if replaced {
            "the stream's file was replaced"
        } else {
            "the stream's file was truncated"
        };
        source.generation += 1;
        source.offset = 0;
        source.pending = None;
        source.totals = None;
        bodies.push(Body::Coverage(Coverage::of(
            "source_generation",
            source,
            Some(0),
            format!(
                "{words}: generation {} is read from its start",
                source.generation
            ),
        )));
    }
    source.identity = Some(identity);
    if let Err(error) = if source.offset == 0 {
        Ok(0)
    } else {
        file.seek(SeekFrom::Start(source.offset))
    } {
        bodies.push(Body::Coverage(Coverage::of(
            "source_refused",
            source,
            Some(source.offset),
            format!("the stream cannot be read: {error}"),
        )));
        return false;
    }
    let mut reader = BufReader::new(file);
    let beginning = source.offset;
    let mut at = source.offset;
    loop {
        let mut line = Vec::new();
        let read = reader.read_until(b'\n', &mut line);
        if let Err(error) = read {
            bodies.push(Body::Coverage(Coverage::of(
                "source_refused",
                source,
                Some(at),
                error.to_string(),
            )));
            return false;
        }
        if line.last() != Some(&b'\n') {
            return false;
        }
        match serde_json::from_slice::<Value>(&line) {
            Ok(record) => bodies.extend(match lines {
                Lines::Harness(reading) => match reading.tracking.harness {
                    Harness::ClaudeCode => reading.claude(source, at, &record),
                    Harness::Codex => reading.codex(source, at, &record),
                },
                Lines::Proxy(reading) => reading.line(source, at, &record),
            }),
            Err(error) => bodies.push(Body::Coverage(Coverage::of(
                "record_unreadable",
                source,
                Some(at),
                format!("the record at byte {at} does not read: {error}"),
            ))),
        }
        at += line.len() as u64;
        source.offset = at;
        // A slice of one pass, not a bound: the caller commits what was read
        // and reads on from here, so every record is read whole.
        if at - beginning >= 1_048_576 {
            return true;
        }
    }
}
