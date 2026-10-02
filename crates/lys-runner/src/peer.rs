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
//! A peer may ask the judge about a tool call, deliver a harness signal,
//! or restart its own held launch. None reaches a server-signed act. What a body
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

#[cfg(any(target_os = "linux", test))]
mod group;

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
    let id = i32::try_from(pid)
        .map_err(|error| unproved(format!("{pid} is not a process id: {error}")))?;
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
    let pid = u32::try_from(pid)
        .map_err(|error| unproved(format!("the peer names no process: {error}")))?;
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
        .map_err(|error| unproved(format!("the peer names no process: {error}")))?;
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

fn process_id(pid: u32) -> Result<rustix::process::Pid, RunnerError> {
    i32::try_from(pid)
        .ok()
        .and_then(rustix::process::Pid::from_raw)
        .filter(|id| id.as_raw_nonzero().get() > 1)
        .ok_or_else(|| unproved(format!("{pid} is not a process id to inspect")))
}

/// Read an identity while distinguishing an exited process from an unreadable one.
pub(crate) fn present_start(pid: u32) -> Result<Option<StartIdentity>, RunnerError> {
    let id = process_id(pid)?;
    match rustix::process::test_kill_process(id) {
        Err(rustix::io::Errno::SRCH) => return Ok(None),
        Err(error) => {
            return Err(RunnerError::refused(
                "process_start_unreadable",
                format!("process {pid}: {error}"),
            ));
        }
        Ok(()) => {}
    }
    match start_identity(pid) {
        Ok(start) => Ok(Some(start)),
        Err(error) => match rustix::process::test_kill_process(id) {
            Err(rustix::io::Errno::SRCH) => Ok(None),
            Ok(()) => Err(RunnerError::refused(
                "process_start_unreadable",
                error.to_string(),
            )),
            Err(probe) => Err(RunnerError::refused(
                "process_start_unreadable",
                format!("{error}; process {pid} existence check: {probe}"),
            )),
        },
    }
}

#[cfg(target_os = "macos")]
fn start_order(start: &StartIdentity) -> Option<(u64, u64)> {
    let (seconds, micros) = start.0.strip_prefix("macos:")?.split_once('.')?;
    let micros: u64 = micros.parse().ok()?;
    (micros < 1_000_000).then_some((seconds.parse().ok()?, micros))
}

#[cfg(target_os = "linux")]
fn start_order(start: &StartIdentity) -> Option<(u64, u64)> {
    Some((start.0.strip_prefix("linux:")?.parse().ok()?, 0))
}

/// Compare numeric kernel start times, never their textual ordering.
pub(crate) fn started_not_before(
    start: &StartIdentity,
    leader: &StartIdentity,
) -> Result<bool, RunnerError> {
    let read = |value: &StartIdentity| {
        start_order(value).ok_or_else(|| {
            RunnerError::refused(
                "process_start_unreadable",
                "start identity does not read as a kernel start time",
            )
        })
    };
    Ok(read(start)? >= read(leader)?)
}

/// List this group's members through the kernel's group filter.
#[cfg(target_os = "macos")]
pub(crate) fn group_members(group: u32) -> Result<Vec<u32>, RunnerError> {
    libproc::processes::pids_by_type(libproc::processes::ProcFilter::ByProgramGroup {
        pgrpid: group,
    })
    .map_err(|error| {
        RunnerError::refused(
            "process_group_unreadable",
            format!("process group {group}: {error}"),
        )
    })
}

/// List this group's members from the kernel's process records.
#[cfg(target_os = "linux")]
pub(crate) fn group_members(group: u32) -> Result<Vec<u32>, RunnerError> {
    process_id(group)?;
    let entries = std::fs::read_dir("/proc").map_err(|error| {
        RunnerError::refused(
            "process_group_unreadable",
            format!("process group {group}: {error}"),
        )
    })?;
    let entries = entries.map(|entry| {
        entry
            .map(|entry| {
                entry
                    .file_name()
                    .to_str()
                    .and_then(|name| name.parse::<u32>().ok())
            })
            .map_err(|error| {
                RunnerError::refused(
                    "process_group_unreadable",
                    format!("process group {group}: {error}"),
                )
            })
    });
    group::members(entries, group, |pid| {
        match rustix::process::getpgid(Some(process_id(pid)?)) {
            Ok(found) => u32::try_from(found.as_raw_nonzero().get())
                .map(Some)
                .map_err(|error| {
                    RunnerError::refused("process_group_unreadable", error.to_string())
                }),
            Err(rustix::io::Errno::SRCH) => Ok(None),
            Err(error) => Err(RunnerError::refused(
                "process_group_unreadable",
                format!("process {pid}: {error}"),
            )),
        }
    })
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
    let before = processes.start(leader.pid)?;
    let after = processes.start(leader.pid)?;
    if before == leader.start && after == leader.start {
        Ok(session.clone())
    } else {
        Err(unproved(format!(
            "process {} is not the leader session {session} was started with: its start identity differs",
            leader.pid
        )))
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
    /// Restart only the session the socket proof names.
    Restart {
        /// The stable operation identity.
        operation: String,
        /// An untrusted session claim; the socket proof chooses the session.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        session: Option<String>,
    },
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

/// A decoded outer request retains its shape through classification and dispatch.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum ParsedRequest {
    /// A harness request authenticated by its socket peer.
    Peer(PeerRequest),
    /// A server request authenticated by its signature.
    Server(crate::protocol::Request),
    /// An unsupported shape still identifies a protocol version.
    Versioned {
        /// The outer protocol version.
        version: u32,
    },
}

#[derive(Debug)]
pub(crate) struct Proof {
    pub(crate) session: String,
    pub(crate) generation: u64,
    pub(crate) leader: Leader,
    pid: u32,
    uid: u32,
    start: StartIdentity,
}

/// A socket's proof, reused only while its peer and launch remain the same.
#[derive(Debug, Default)]
pub struct Connection {
    proof: Option<Proof>,
}

impl Connection {
    pub(crate) fn proved_with(
        &mut self,
        sessions: &Sessions,
        processes: &dyn Processes,
        (pid, uid, own): (u32, u32, u32),
    ) -> Result<&Proof, RunnerError> {
        if uid != own {
            return Err(unproved("the peer and runner have different users"));
        }
        if let Some(proof) = self.proof.as_ref() {
            if proof.pid != pid
                || proof.uid != uid
                || processes.start(pid)? != proof.start
                || processes.start(proof.leader.pid)? != proof.leader.start
                || !sessions.peer_matches(&proof.session, proof.generation, &proof.leader)?
            {
                return Err(unproved(
                    "the connection's proved process or launch changed",
                ));
            }
        } else {
            let start = processes.start(pid)?;
            let mut walked = BTreeSet::new();
            let mut at = pid;
            let (session, generation, leader) = loop {
                if let Some(found) = sessions.peer_leader(at)? {
                    break found;
                }
                if at <= 1 || !walked.insert(at) {
                    return Err(unproved("the peer descends from no running session"));
                }
                at = processes.parent(at)?;
            };
            if processes.start(leader.pid)? != leader.start
                || processes.start(leader.pid)? != leader.start
                || processes.start(pid)? != start
                || !sessions.peer_matches(&session, generation, &leader)?
            {
                return Err(unproved("the peer or its launch changed during proof"));
            }
            self.proof = Some(Proof {
                session,
                generation,
                leader,
                pid,
                uid,
                start,
            });
        }
        self.proof
            .as_ref()
            .ok_or_else(|| unproved("the connection has no process proof"))
    }

    /// Answer a parsed request without repeating its ancestry walk.
    pub fn answer(
        &mut self,
        sessions: &Arc<Sessions>,
        stream: &UnixStream,
        request: PeerRequest,
        left: &AtomicBool,
    ) -> Answer {
        if request.version != PROTOCOL_VERSION {
            return Answer::refusal(&RunnerError::ProtocolMismatch {
                theirs: request.version,
                ours: PROTOCOL_VERSION,
            });
        }
        let proved = peer_of(stream)
            .and_then(|(pid, uid)| self.proved_with(sessions, &System, (pid, uid, own_user())));
        let proof = match proved {
            Ok(proof) => proof,
            Err(error) => {
                return match request.peer {
                    PeerAct::Judge(_) => Answer::Judged {
                        verdict: crate::refusals::Verdict::denied(
                            &error.name(),
                            error.to_string(),
                            "not_attributed",
                        ),
                    },
                    PeerAct::Restart { .. } if error.name() == "peer_unproved" => {
                        Answer::refusal(&RunnerError::refused("not_a_session", error.to_string()))
                    }
                    PeerAct::Restart { .. } | PeerAct::Collect(_) => Answer::refusal(&error),
                };
            }
        };
        match request.peer {
            PeerAct::Restart { operation, .. } => sessions
                .restart_proved(&proof.session, &operation, &proof.leader)
                .map_or_else(
                    |error| Answer::refusal(&error),
                    |outcome| Answer::Operation { outcome },
                ),
            PeerAct::Judge(asked) => Answer::Judged {
                verdict: crate::refusal_log::answer_proved(
                    sessions,
                    &proof.session,
                    proof.generation,
                    &proof.leader,
                    &asked,
                    left,
                ),
            },
            PeerAct::Collect(collected) => {
                sessions.collect(&proof.session, &collected).map_or_else(
                    |error| Answer::refusal(&error),
                    |words| Answer::Collected { words },
                )
            }
        }
    }
}

/// Decode the outer JSON once; typed variants retain duplicate-field validation.
pub fn parse(line: &str) -> Result<ParsedRequest, RunnerError> {
    let request: ParsedRequest =
        serde_json::from_str(line).map_err(|error| RunnerError::Malformed {
            reason: format!("the request does not read: {error}"),
        })?;
    let version = match &request {
        ParsedRequest::Peer(request) => request.version,
        ParsedRequest::Server(request) => request.version,
        ParsedRequest::Versioned { version } => *version,
    };
    if version != PROTOCOL_VERSION {
        return Err(RunnerError::ProtocolMismatch {
            theirs: version,
            ours: PROTOCOL_VERSION,
        });
    }
    if matches!(request, ParsedRequest::Versioned { .. }) {
        return Err(RunnerError::Malformed {
            reason: "the request is neither a peer nor a server request".to_owned(),
        });
    }
    Ok(request)
}

/// Whether a decoded request is a peer's rather than the server's.
pub fn is_peer(request: &ParsedRequest) -> bool {
    matches!(request, ParsedRequest::Peer(_))
}

/// Decode a one-shot peer request and answer it without another JSON parse.
pub fn answer(
    sessions: &Arc<Sessions>,
    stream: &UnixStream,
    line: &str,
    left: &AtomicBool,
) -> Answer {
    match parse(line) {
        Ok(ParsedRequest::Peer(request)) => answer_parsed(sessions, stream, request, left),
        Ok(_) => Answer::refusal(&RunnerError::Malformed {
            reason: "the request is not a peer request".to_owned(),
        }),
        Err(error) => Answer::refusal(&error),
    }
}

/// Answer the typed peer request supplied by socket dispatch.
pub fn answer_parsed(
    sessions: &Arc<Sessions>,
    stream: &UnixStream,
    request: PeerRequest,
    left: &AtomicBool,
) -> Answer {
    Connection::default().answer(sessions, stream, request, left)
}

#[cfg(test)]
pub(crate) fn restart_with(
    sessions: &Arc<Sessions>,
    processes: &dyn Processes,
    identity: (u32, u32, u32),
    operation: &str,
) -> Result<crate::operations::OperationOutcome, RunnerError> {
    let leaders = sessions.leaders()?;
    let session = prove_with(processes, identity, &leaders)
        .map_err(|error| RunnerError::refused("not_a_session", error.to_string()))?;
    let leader = leaders
        .get(&session)
        .ok_or_else(|| RunnerError::refused("not_a_session", "the proved session has no leader"))?;
    sessions.restart_proved(&session, operation, leader)
}

#[cfg(test)]
#[path = "../tests/peer_restart/cases.rs"]
mod restart_tests;

#[cfg(test)]
mod parse_tests {
    use super::{ParsedRequest, is_peer, parse};

    #[test]
    fn one_typed_request_is_shared_by_classification_and_dispatch()
    -> Result<(), crate::error::RunnerError> {
        let parsed = parse(r#"{"version":1,"peer":{"act":"restart","operation":"stable"}}"#)?;
        assert!(is_peer(&parsed));
        let ParsedRequest::Peer(request) = parsed else {
            panic!("peer request shape changed");
        };
        assert!(
            matches!(request.peer, super::PeerAct::Restart { operation, .. } if operation == "stable")
        );
        Ok(())
    }

    #[test]
    fn duplicate_fields_unknown_fields_and_unknown_versions_are_refused() {
        for line in [
            r#"{"version":1,"version":1,"peer":{"act":"restart","operation":"stable"}}"#,
            r#"{"version":1,"peer":{"act":"restart","operation":"stable","unknown":true}}"#,
        ] {
            assert!(parse(line).is_err(), "{line}");
        }
        assert!(parse(r#"{"version":999}"#).is_err_and(|error| matches!(
            error,
            crate::error::RunnerError::ProtocolMismatch { .. }
        )));
    }
}

#[cfg(test)]
#[path = "peer_connection_tests.rs"]
mod connection_tests;
