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
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::RunnerError;
use crate::protocol::{Answer, PROTOCOL_VERSION};
use crate::refusals::JudgeAsk;
use crate::session::Sessions;

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
                processes
                    .start(leader.pid)
                    .map_err(|error| error.to_string()),
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
pub fn answer(
    sessions: &Arc<Sessions>,
    stream: &UnixStream,
    line: &str,
    left: &AtomicBool,
) -> Answer {
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
            .map_or_else(
                |error| Answer::refusal(&error),
                |words| Answer::Collected { words },
            ),
    }
}
