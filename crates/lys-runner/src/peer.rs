//! Who is on the other end of a connection that is not the server's: the
//! judge and collector clients a session's harness runs.
//!
//! A peer is never believed about itself. Its process id and user are read
//! from the connected socket (`LOCAL_PEERPID` and `getpeereid` on macOS,
//! `SO_PEERCRED` on Linux), never from its request. Its user must be the
//! runner's. Its ancestry is then walked, parent by parent, to the leader of
//! a session this runner started, and the leader's start identity, recorded
//! at spawn, is read before and after the walk: a leader whose identity
//! differs from the one recorded, or changed during the walk, is a reused
//! process id, not the session. A peer whose ancestry reaches no leader, or
//! breaks where a parent cannot be read, is not proved, and so not
//! attributed to any session. The socket stays connected through the proof
//! and the answer.
//!
//! macOS process records are read through the safe `libproc` interface and
//! Linux's through `/proc`; this crate adds no unsafe code. A proof is a
//! statement about processes of one user on one machine: it does not stand
//! against that same user tampering with the harness.
//!
//! A peer may ask two acts and no other: the judge's question about a tool
//! call and the collector's delivery of what the harness signalled. Neither
//! carries a signature and neither reaches a server-signed act. What a body
//! says of its session changes nothing: the record is the proved session's
//! or none. A hook's prompt text and a notice's message text are never kept.

use std::collections::{BTreeMap, BTreeSet};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::RunnerError;
use crate::protocol::{Answer, PROTOCOL_VERSION};
use crate::refusals::JudgeAsk;
use crate::session::{Sessions, Table, accounts, append, now_ms};
use crate::tracking::{Harness, Reading};
use crate::tracking_store::{Body, Boundary, Coverage, SourceState};

/// A process's start identity: what tells one process from a later one
/// given the same id.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct StartIdentity(pub String);

/// A session's leader, as the runner recorded it at spawn.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Leader {
    /// Its process id.
    pub pid: u32,
    /// Its start identity when it was spawned.
    pub start: StartIdentity,
}

/// What the proof reads of the machine's processes.
pub trait Processes {
    /// The parent of `pid`.
    fn parent(&self, pid: u32) -> Result<u32, RunnerError>;
    /// The start identity of `pid`.
    fn start(&self, pid: u32) -> Result<StartIdentity, RunnerError>;
}

/// This machine's processes.
#[derive(Debug, Clone, Copy, Default)]
pub struct System;

fn unproved(reason: impl Into<String>) -> RunnerError {
    RunnerError::refused("peer_unproved", reason)
}

#[cfg(target_os = "macos")]
fn bsd_info(pid: u32) -> Result<libproc::bsd_info::BSDInfo, RunnerError> {
    let id = i32::try_from(pid).map_err(|_wide| unproved(format!("{pid} is not a process id")))?;
    libproc::proc_pid::pidinfo::<libproc::bsd_info::BSDInfo>(id, 0)
        .map_err(|error| unproved(format!("process {pid} cannot be read: {error}")))
}

#[cfg(target_os = "linux")]
fn stat_fields(pid: u32) -> Result<Vec<String>, RunnerError> {
    let text = std::fs::read_to_string(format!("/proc/{pid}/stat"))
        .map_err(|error| unproved(format!("process {pid} cannot be read: {error}")))?;
    let (_, after) = text
        .rsplit_once(')')
        .ok_or_else(|| unproved(format!("process {pid}'s record does not read")))?;
    Ok(after.split_whitespace().map(str::to_owned).collect())
}

impl Processes for System {
    #[cfg(target_os = "macos")]
    fn parent(&self, pid: u32) -> Result<u32, RunnerError> {
        Ok(bsd_info(pid)?.pbi_ppid)
    }

    #[cfg(target_os = "macos")]
    fn start(&self, pid: u32) -> Result<StartIdentity, RunnerError> {
        let info = bsd_info(pid)?;
        Ok(StartIdentity(format!(
            "macos:{}.{:06}",
            info.pbi_start_tvsec, info.pbi_start_tvusec
        )))
    }

    #[cfg(target_os = "linux")]
    fn parent(&self, pid: u32) -> Result<u32, RunnerError> {
        stat_fields(pid)?
            .get(1)
            .and_then(|field| field.parse().ok())
            .ok_or_else(|| unproved(format!("process {pid}'s parent does not read")))
    }

    #[cfg(target_os = "linux")]
    fn start(&self, pid: u32) -> Result<StartIdentity, RunnerError> {
        stat_fields(pid)?
            .get(19)
            .map(|field| StartIdentity(format!("linux:{field}")))
            .ok_or_else(|| unproved(format!("process {pid}'s start does not read")))
    }
}

/// The process id and user of the peer connected on `stream`, read from
/// the socket.
#[cfg(target_os = "macos")]
pub fn peer_of(stream: &UnixStream) -> Result<(u32, u32), RunnerError> {
    let pid = nix::sys::socket::getsockopt(stream, nix::sys::socket::sockopt::LocalPeerPid)
        .map_err(|error| unproved(format!("the peer's process cannot be read: {error}")))?;
    let (uid, _group) = nix::unistd::getpeereid(stream)
        .map_err(|error| unproved(format!("the peer's user cannot be read: {error}")))?;
    let pid = u32::try_from(pid).map_err(|_negative| unproved("the peer names no process"))?;
    Ok((pid, uid.as_raw()))
}

/// The process id and user of the peer connected on `stream`, read from
/// the socket.
#[cfg(target_os = "linux")]
pub fn peer_of(stream: &UnixStream) -> Result<(u32, u32), RunnerError> {
    let credentials =
        nix::sys::socket::getsockopt(stream, nix::sys::socket::sockopt::PeerCredentials)
            .map_err(|error| unproved(format!("the peer's credentials cannot be read: {error}")))?;
    let pid = u32::try_from(credentials.pid())
        .map_err(|_negative| unproved("the peer names no process"))?;
    Ok((pid, credentials.uid()))
}

/// The runner's own user.
pub fn own_user() -> u32 {
    rustix::process::getuid().as_raw()
}

/// The start identity of `pid` on this machine, recorded when a session's
/// leader is spawned.
pub fn start_identity(pid: u32) -> Result<StartIdentity, RunnerError> {
    System.start(pid)
}

/// The session whose leader the peer `pid`, of user `uid`, descends from,
/// among `leaders`, as `processes` reads them. Refused `peer_unproved`,
/// with the reason, when the user is not `own`, when the ancestry breaks or
/// reaches no leader, or when the leader's start identity is not the one
/// recorded, before and after the walk alike.
pub fn prove_with(
    processes: &dyn Processes,
    (pid, uid, own): (u32, u32, u32),
    leaders: &BTreeMap<String, Leader>,
) -> Result<String, RunnerError> {
    if uid != own {
        return Err(unproved(format!(
            "the peer runs as user {uid}, and the runner as user {own}"
        )));
    }
    let before: BTreeMap<u32, Result<StartIdentity, String>> = leaders
        .values()
        .map(|leader| {
            (
                leader.pid,
                processes.start(leader.pid).map_err(|error| error.to_string()),
            )
        })
        .collect();
    let mut walked = BTreeSet::new();
    let mut at = pid;
    let (session, leader) = loop {
        if let Some(found) = leaders.iter().find(|(_, leader)| leader.pid == at) {
            break found;
        }
        if at <= 1 || !walked.insert(at) {
            return Err(unproved(format!(
                "process {pid} descends from no session this runner started"
            )));
        }
        at = processes.parent(at)?;
    };
    let after = processes.start(leader.pid)?;
    match before.get(&leader.pid) {
        Some(Ok(seen)) if *seen == leader.start && after == leader.start => Ok(session.clone()),
        Some(Err(reason)) => Err(unproved(format!(
            "session {session}'s leader could not be read: {reason}"
        ))),
        _ => Err(unproved(format!(
            "process {} is not the leader session {session} was started with: its start identity differs",
            leader.pid
        ))),
    }
}

/// The session the peer on `stream` belongs to, among `leaders`, proved on
/// this machine.
pub fn prove(
    stream: &UnixStream,
    leaders: &BTreeMap<String, Leader>,
) -> Result<String, RunnerError> {
    let (pid, uid) = peer_of(stream)?;
    prove_with(&System, (pid, uid, own_user()), leaders)
}

/// What a session's harness hands its collector.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "source", rename_all = "snake_case", deny_unknown_fields)]
pub enum Collected {
    /// A hook fired: its event name and its input.
    Hook {
        /// The hook event.
        event: String,
        /// Its input, as the harness gave it.
        input: Value,
    },
    /// The status line's input.
    StatusLine {
        /// Its input, as the harness gave it.
        input: Value,
    },
    /// The harness's after-turn notification, message text removed.
    Notify {
        /// The notification.
        notification: Value,
    },
}

/// What a peer may ask.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "act", rename_all = "snake_case", deny_unknown_fields)]
pub enum PeerAct {
    /// Judge a tool call before it runs.
    Judge(JudgeAsk),
    /// Keep what the harness signalled.
    Collect(Collected),
}

/// A peer's request line.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PeerRequest {
    /// The protocol version.
    pub version: u32,
    /// The act.
    pub peer: PeerAct,
}

/// Whether request `line` is a peer's rather than the server's.
pub fn is_peer(line: &str) -> bool {
    serde_json::from_str::<Value>(line).is_ok_and(|value| value.get("peer").is_some())
}

/// The answer to the peer request `line`, from the peer on `stream`.
pub fn answer(sessions: &Arc<Sessions>, stream: &UnixStream, line: &str, left: &AtomicBool) -> Answer {
    let request = match serde_json::from_str::<PeerRequest>(line) {
        Ok(request) if request.version == PROTOCOL_VERSION => request,
        Ok(request) => {
            return Answer::refusal(&RunnerError::ProtocolMismatch {
                theirs: request.version,
                ours: PROTOCOL_VERSION,
            });
        }
        Err(error) => {
            return Answer::refusal(&RunnerError::Malformed {
                reason: format!("the peer's request does not read: {error}"),
            });
        }
    };
    match request.peer {
        PeerAct::Judge(asked) => Answer::Judged {
            verdict: crate::refusal_log::answer(sessions, stream, &asked, left),
        },
        PeerAct::Collect(collected) => prove(stream, &sessions.leaders())
            .and_then(|session| sessions.collect(&session, &collected))
            .map_or_else(|error| Answer::refusal(&error), |words| Answer::Collected { words }),
    }
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
    pub fn collect(self: &Arc<Self>, id: &str, collected: &Collected) -> Result<String, RunnerError> {
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
            return Err(unbound("the hook names no session id and transcript".to_owned()));
        };
        let mut table = self.lock();
        let session = table.sessions.get(id).ok_or_else(|| crate::session::unknown(id))?;
        let tracking = session
            .guard
            .tracking
            .as_ref()
            .filter(|tracking| tracking.harness == Harness::ClaudeCode)
            .ok_or_else(|| unbound(format!("session {id} is not a tracked Claude Code session")))?;
        let launched = session.launch.as_ref().map(|launch| launch.directory.clone());
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
        Ok(self.bind(&mut table, id, (given, claude), false))
    }

    /// Bind session `id` to the stream at `path` as the harness's `bound`
    /// session or thread, from its start or its present end, and follow it.
    fn bind(
        self: &Arc<Self>,
        table: &mut Table,
        id: &str,
        (path, bound): (&str, &str),
        from_start: bool,
    ) -> String {
        let held = table.feed.source(id).cloned();
        if held.as_ref().is_some_and(|held| held.path == path) {
            return format!("{path} is already bound");
        }
        let offset = if from_start {
            0
        } else {
            std::fs::metadata(path).map_or(0, |metadata| metadata.len())
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
            Body::Coverage(Coverage::of("source_bound", &source, Some(offset), words.clone())),
            boundary("session_start", None),
        ];
        append(table, id, bodies, Some(source));
        self.follow(table, id);
        words
    }

    fn status_line(&self, id: &str, input: &Value) -> Result<String, RunnerError> {
        let mut guard = self.lock();
        let table = &mut *guard;
        let session = table.sessions.get(id).ok_or_else(|| crate::session::unknown(id))?;
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
            .ok_or_else(|| RunnerError::refused("notify_malformed", "the notification names no thread"))?;
        let turn = text(notification, "turn-id").map(str::to_owned);
        let mut table = self.lock();
        if table.feed.source(id).is_none_or(|source| source.bound != thread) {
            let home = table
                .sessions
                .get(id)
                .and_then(|session| session.guard.tracking.as_ref())
                .filter(|tracking| tracking.harness == Harness::Codex)
                .map(|tracking| tracking.config_home.clone())
                .ok_or_else(|| RunnerError::refused("transcript_unbound", format!("session {id} is not a tracked Codex session")))?;
            let path = rollout(Path::new(&home), thread)?.display().to_string();
            self.bind(&mut table, id, (&path, thread), true);
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
        let entries = match std::fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(error) => {
                crate::error::said(&format!("{} is not searched for rollouts: {error}", dir.display()));
                continue;
            }
        };
        for entry in entries.flatten() {
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
        .and_then(|file| std::io::BufRead::read_line(&mut std::io::BufReader::new(file), &mut first))
        .map_err(|error| RunnerError::refused("transcript_unbound", error.to_string()))?;
    let meta: Value = serde_json::from_str(first.trim_end()).unwrap_or(Value::Null);
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
