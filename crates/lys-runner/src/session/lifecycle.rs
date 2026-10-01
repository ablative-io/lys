//! A session's process: started in its terminal, its output kept, its exit
//! seen, and at a usage limit moved to its next account. One thread reads
//! the terminal and one waits on the process; neither runs on a clock.
//!
//! A tracked session's harness is checked before it runs: the executable
//! the launch names is found and asked its version, and one that reports a
//! version its adapter was not measured against is refused
//! `tracking_contract_unsupported`. Its leader's start identity is read at
//! spawn, for the peer proof. Its stream is followed by one more thread,
//! woken by the filesystem's change notices on the stream's directory and
//! never on a timer; it reads from the saved offset to the last whole line,
//! so a restart never reads the stream again from its start, a partial last
//! line is left for the next read, and a truncated or replaced file is a new
//! generation, said by name.

use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, mpsc};

use portable_pty::Child;
use serde_json::Value;

use super::{Live, Session, Sessions, Table, now_ms, unknown};
use crate::error::RunnerError;
use crate::peer::Leader;
use crate::protocol::{Ended, EndedHow, Launch};
use crate::tracking::{Accounts, Harness, Reading, Tracking, version_in};
use crate::tracking_store::{Body, Commit, Coverage, SourceState};

pub use crate::peer::Collected;

#[cfg(test)]
#[path = "../../tests/lifecycle/cases.rs"]
mod io_tests;

/// What wakes a session's stream follower.
#[derive(Debug)]
pub(crate) enum Wake {
    /// The stream's directory changed.
    Changed,
    /// Change notices were lost; the stream is read from its cursor again.
    Lost(String),
    /// The session ended, or its stream was bound to another file.
    Stop,
}

impl Sessions {
    /// Run the session's process, on its rotation's account when it has one,
    /// and start the threads that read its output and see its exit.
    pub(super) fn run(
        self: &Arc<Self>,
        id: &str,
        session: &mut Session,
        resumed: bool,
    ) -> Result<u32, RunnerError> {
        let launch = session.launch.as_ref().ok_or_else(|| unknown(id))?;
        let mut environment = launch.environment.clone();
        let mut arguments = launch.arguments.clone();
        if let Some(rotation) = &session.rotation {
            environment.insert(
                rotation.variable().to_owned(),
                rotation.account().to_owned(),
            );
            if let (true, Some(resume)) = (resumed, rotation.resume_arguments()) {
                arguments = resume.to_vec();
            }
        }
        let spawned = crate::pty::spawn(&crate::pty::Spawn {
            program: &launch.program,
            arguments: &arguments,
            directory: &launch.directory,
            environment: &environment,
            columns: session.columns,
            rows: session.rows,
        })?;
        session.generation += 1;
        session.pid = Some(spawned.pid);
        session.live = Some(Live {
            writer: crate::input::Input::new(spawned.writer),
            master: spawned.master,
            pid: spawned.pid,
        });
        session.guard.leader = match crate::peer::start_identity(spawned.pid) {
            Ok(start) => Some(Leader {
                pid: spawned.pid,
                start,
            }),
            Err(error) => {
                crate::error::said(&format!(
                    "session {id}: its leader's start identity was not read, so no peer of it is proved: {error}"
                ));
                None
            }
        };
        session.leader_start = session.guard.leader.clone();
        let generation = session.generation;
        let (reader, child) = (spawned.reader, spawned.child);
        let pumped = Arc::clone(self);
        let owned = id.to_owned();
        let pump = std::thread::spawn(move || pumped.pump(&owned, generation, reader));
        let watched = Arc::clone(self);
        let owned = id.to_owned();
        std::thread::spawn(move || watched.watch(&owned, generation, child, pump));
        Ok(spawned.pid)
    }

    /// Keep the output of generation `generation` of session `id` until its
    /// terminal closes.
    fn pump(&self, id: &str, generation: u64, mut reader: Box<dyn Read + Send>) {
        let mut buffer = [0_u8; 8192];
        loop {
            let read = match reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(read) => read,
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(error) => {
                    crate::error::said(&format!("session {id}: the terminal closed: {error}"));
                    break;
                }
            };
            let mut table = self.lock();
            if let Some(session) = table
                .sessions
                .get_mut(id)
                .filter(|s| s.generation == generation)
            {
                session.scrollback.push(&buffer[..read]);
                trip_on_words(session, read);
            }
            drop(table);
            self.wake();
        }
    }

    /// See the exit of generation `generation` of session `id`: move it to
    /// its next account at a usage limit, or record its end. The end is
    /// recorded once the exit is seen and every byte its terminal gave is
    /// kept, so a read that answers an end has read everything before it.
    fn watch(
        self: &Arc<Self>,
        id: &str,
        generation: u64,
        mut child: Box<dyn Child + Send + Sync>,
        pump: std::thread::JoinHandle<()>,
    ) {
        let waited = child.wait();
        let at = now_ms();
        let cleanup = self
            .lock()
            .sessions
            .get(id)
            .filter(|session| session.generation == generation)
            .filter(|session| {
                session.ending
                    || session
                        .rotation
                        .as_ref()
                        .is_some_and(crate::rotation::RotationState::tripped)
            })
            .and_then(|session| session.guard.leader.clone());
        if let Some(leader) = cleanup {
            match crate::pty::end_left_group(&leader) {
                Ok(crate::pty::Left::Gone | crate::pty::Left::Ended { reason: None }) => {}
                Ok(left) => {
                    crate::error::said(&format!(
                        "session {id}: group_cleanup_incomplete: {left:?}"
                    ));
                }
                Err(error) => {
                    crate::error::said(&format!("session {id}: group_cleanup_failed: {error}"));
                }
            }
        }
        if pump.join().is_err() {
            crate::error::said(&format!(
                "session {id}: the thread keeping its output ended abnormally"
            ));
        }
        self.read_source(id, None);
        let mut table = self.lock();
        let Some(session) = table
            .sessions
            .get_mut(id)
            .filter(|s| s.generation == generation)
        else {
            return;
        };
        session.live = None;
        let (status, signal) = match &waited {
            Ok(exit) => match exit.signal() {
                Some(signal) => (None, Some(signal.to_owned())),
                None => (Some(exit.exit_code()), None),
            },
            Err(error) => {
                crate::error::said(&format!(
                    "session {id}: the process's exit could not be read: {error}"
                ));
                (None, None)
            }
        };
        let limit = !session.ending
            && session
                .rotation
                .as_ref()
                .is_some_and(|rotation| rotation.limit_at_exit(status));
        let mut how = EndedHow::Exited;
        if limit {
            let next = session
                .rotation
                .as_mut()
                .and_then(|rotation| rotation.advance(at));
            if let Some(moved) = next {
                crate::error::said(&format!(
                    "session {id} reached its usage limit and moves to account {moved}"
                ));
                match self.run(id, session, true) {
                    Ok(_) => {
                        self.persist_logged(&table);
                        drop(table);
                        self.wake();
                        return;
                    }
                    Err(error) => crate::error::said(&format!(
                        "session {id} could not move to its next account: {error}"
                    )),
                }
            } else {
                crate::error::said(&format!(
                    "session {id} reached its usage limit on the last of its accounts"
                ));
                how = EndedHow::AccountsExhausted;
            }
        }
        let ended = Ended {
            how,
            at,
            status,
            signal,
            reason: None,
        };
        session.ended = Some(ended.clone());
        if let Some(follower) = session.follower.take() {
            stop_follower(id, &follower);
        }
        if table
            .sessions
            .get(id)
            .is_some_and(|session| session.guard.tracking.is_some())
        {
            crate::collector::flushed(&mut table, self.runner(), id, "session_end");
        }
        crate::operations::ended(&mut table, id, &ended);
        self.persist_logged(&table);
        drop(table);
        self.wake();
    }

    /// Follow session `id`'s bound stream on a thread of its own, stopping
    /// any follower it had.
    pub(crate) fn follow(self: &Arc<Self>, table: &mut Table, id: &str) {
        let Some(path) = table
            .feed
            .source(id)
            .map(|source| PathBuf::from(&source.path))
        else {
            return;
        };
        let Some(session) = table.sessions.get_mut(id) else {
            return;
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
        let dir = path.parent().unwrap_or_else(|| Path::new("/")).to_owned();
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
                let coverage = Coverage::of("coverage_incomplete", &source, None, words);
                append(table, id, vec![Body::Coverage(coverage)], None);
                return;
            }
        };
        session.follower = Some(wake);
        let (sessions, owned) = (Arc::clone(self), id.to_owned());
        std::thread::spawn(move || {
            sessions.read_source(&owned, None);
            for next in woken {
                match next {
                    Wake::Changed => sessions.read_source(&owned, None),
                    Wake::Lost(reason) => sessions.read_source(&owned, Some(&reason)),
                    Wake::Stop => break,
                }
            }
            drop(watcher);
        });
    }

    /// Read session `id`'s stream from its saved cursor to its last whole
    /// line, and keep what it yields with the new cursor as one unit.
    pub(crate) fn read_source(&self, id: &str, lost: Option<&str>) {
        loop {
            let table = self.lock();
            let Some(session) = table.sessions.get(id) else {
                return;
            };
            let Some(tracking) = session.guard.tracking.clone() else {
                return;
            };
            let Some(mut source) = table.feed.source(id).cloned() else {
                return;
            };
            let evidence = accounts(session, &tracking);
            let current = evidence.current.map(str::to_owned);
            let moves = evidence.moves.to_vec();
            drop(table);
            let reading = Reading {
                runner: self.state.runner(),
                session: id,
                tracking: &tracking,
                accounts: Accounts {
                    current: current.as_deref(),
                    moves: &moves,
                    declared: tracking.account.as_deref(),
                },
                now: now_ms(),
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
            let more = read_lines(&reading, &mut source, &mut bodies);
            if bodies.is_empty() && source == before {
                return;
            }
            let mut table = self.lock();
            if table.feed.source(id) != Some(&before) {
                drop(table);
                continue;
            }
            let commit = Commit {
                source: Some(source),
                attempt: None,
            };
            if let Err(error) = table.feed.append(id, now_ms(), bodies, commit) {
                crate::error::said(&format!(
                    "session {id}: coverage_incomplete: what its stream yielded was not kept, and is read again from the saved cursor: {error}"
                ));
            }
            drop(table);
            self.wake();
            if !more {
                return;
            }
        }
    }
}

/// Stop the follower `follower` of session `id`.
fn stop_follower(id: &str, follower: &mpsc::Sender<Wake>) {
    if follower.send(Wake::Stop).is_err() {
        crate::error::said(&format!(
            "session {id}: its stream follower had already ended"
        ));
    }
}

/// Keep `bodies` for session `id` as one unit, with `source` when given.
pub(crate) fn append(table: &mut Table, id: &str, bodies: Vec<Body>, source: Option<SourceState>) {
    let commit = Commit {
        source,
        attempt: None,
    };
    if let Err(error) = table.feed.append(id, now_ms(), bodies, commit) {
        crate::error::said(&format!("session {id}: coverage_incomplete: {error}"));
    }
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

/// Read `source` from its offset to its last whole line into `bodies`.
fn read_lines(reading: &Reading<'_>, source: &mut SourceState, bodies: &mut Vec<Body>) -> bool {
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
    if let Err(error) = file.seek(SeekFrom::Start(source.offset)) {
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
        let read = reader.by_ref().take(1_048_577).read_until(b'\n', &mut line);
        if let Err(error) = read {
            bodies.push(Body::Coverage(Coverage::of(
                "source_refused",
                source,
                Some(at),
                error.to_string(),
            )));
            return false;
        }
        if line.len() > 1_048_576 {
            bodies.push(Body::Coverage(Coverage::of(
                "record_too_large",
                source,
                Some(at),
                "a transcript record exceeds 1048576 bytes".to_owned(),
            )));
            return false;
        }
        if line.last() != Some(&b'\n') {
            return false;
        }
        match serde_json::from_slice::<Value>(&line) {
            Ok(record) => bodies.extend(match reading.tracking.harness {
                Harness::ClaudeCode => reading.claude(source, at, &record),
                Harness::Codex => reading.codex(source, at, &record),
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
        if at - beginning >= 1_048_576 {
            return true;
        }
    }
}

/// The directory a session is bound to: `directory` resolved, or as given
/// when it cannot be.
pub(crate) fn bound_directory(directory: &str) -> String {
    let given = if directory.is_empty() { "." } else { directory };
    std::fs::canonicalize(given).map_or_else(
        |error| {
            crate::error::said(&format!(
                "{given} does not resolve, and is kept as given: {error}"
            ));
            given.to_owned()
        },
        |path| path.display().to_string(),
    )
}

/// The executable `launch` runs, found as its environment's `PATH` finds
/// it, and the version it reports, refused `tracking_contract_unsupported`
/// unless it is the version `tracking` declares.
pub(crate) fn launched(
    launch: &Launch,
    tracking: &Tracking,
) -> Result<(String, String), RunnerError> {
    let unsupported = |words: String| RunnerError::refused("tracking_contract_unsupported", words);
    let program = Path::new(&launch.program);
    let found = if launch.program.contains('/') {
        Some(program.to_owned())
    } else {
        let path = launch
            .environment
            .get("PATH")
            .cloned()
            .or_else(|| std::env::var("PATH").ok())
            .unwrap_or_default();
        std::env::split_paths(&path)
            .map(|dir| dir.join(program))
            .find(|candidate| candidate.is_file())
    };
    let executable = found
        .and_then(|found| std::fs::canonicalize(found).ok())
        .ok_or_else(|| {
            unsupported(format!(
                "{} is not found to ask its version",
                launch.program
            ))
        })?;
    let output = std::process::Command::new(&executable)
        .arg("--version")
        .output()
        .map_err(|error| {
            unsupported(format!(
                "{} did not say its version: {error}",
                executable.display()
            ))
        })?;
    let said = String::from_utf8_lossy(&output.stdout);
    let version = version_in(&said)
        .ok_or_else(|| unsupported(format!("{} names no version", executable.display())))?;
    crate::tracking::measured(&tracking.adapter, &version)?;
    if version != tracking.version {
        return Err(unsupported(format!(
            "{} is version {version}, and the profile declares {}",
            executable.display(),
            tracking.version
        )));
    }
    Ok((executable.display().to_string(), version))
}

/// Say, in the feed, the executable and version launched for session `id`.
pub(crate) fn tracking_started(table: &mut Table, id: &str, executable: &str, version: &str) {
    let tracking = table
        .sessions
        .get(id)
        .and_then(|session| session.guard.tracking.as_ref());
    let adapter = tracking.map(|tracking| tracking.adapter.clone());
    let coverage = Coverage {
        state: "tracking_started".to_owned(),
        source: None,
        generation: 0,
        offset: None,
        words: format!("launched {executable}, which says it is version {version}"),
        executable: Some(executable.to_owned()),
        harness_version: Some(version.to_owned()),
        adapter,
    };
    append(table, id, vec![Body::Coverage(coverage)], None);
}

/// Mark the session's usage-limit words seen in the last `read` bytes, and
/// end its process so its exit moves it to the next account.
fn trip_on_words(session: &mut Session, read: usize) {
    if session
        .guard
        .tracking
        .as_ref()
        .is_some_and(|tracking| tracking.harness == Harness::ClaudeCode)
    {
        return;
    }
    let Some(rotation) = session.rotation.as_mut() else {
        return;
    };
    if session.ending || rotation.tripped() {
        return;
    }
    let reach = read + rotation.longest_word().saturating_sub(1);
    let from = session
        .scrollback
        .end()
        .saturating_sub(reach as u64)
        .max(session.scrollback.oldest());
    let Ok(bytes) = session.scrollback.from(from) else {
        return;
    };
    if rotation.words_in(&String::from_utf8_lossy(&bytes)) {
        rotation.trip();
        if let Some(live) = &session.live {
            live.end("at its usage limit");
        }
    }
}
