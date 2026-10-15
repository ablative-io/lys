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
    ///
    /// # Errors
    /// - `RunnerError::Refused` if native process metadata cannot be read or a custom provider refuses the lookup.
    /// - `RunnerError::Unsigned` if supplied by a custom process provider; the provider error is returned unchanged.
    /// - `RunnerError::Malformed` if supplied by a custom process provider; the provider error is returned unchanged.
    /// - `RunnerError::ProtocolMismatch` if supplied by a custom process provider; the provider error is returned unchanged.
    /// - `RunnerError::Replayed` if supplied by a custom process provider; the provider error is returned unchanged.
    /// - `RunnerError::Misaddressed` if supplied by a custom process provider; the provider error is returned unchanged.
    /// - `RunnerError::Unreachable` if supplied by a custom process provider; the provider error is returned unchanged.
    /// - `RunnerError::ReplyMalformed` if supplied by a custom process provider; the provider error is returned unchanged.
    /// - `RunnerError::State` if supplied by a custom process provider; the provider error is returned unchanged.
    /// - `RunnerError::StateHeld` if supplied by a custom process provider; the provider error is returned unchanged.
    /// - `RunnerError::AlreadyRunning` if supplied by a custom process provider; the provider error is returned unchanged.
    /// - `RunnerError::Socket` if supplied by a custom process provider; the provider error is returned unchanged.
    /// - `RunnerError::Key` if supplied by a custom process provider; the provider error is returned unchanged.
    /// - `RunnerError::DialStale` if supplied by a custom process provider; the provider error is returned unchanged.
    /// - `RunnerError::Dial` if supplied by a custom process provider; the provider error is returned unchanged.
    fn parent(&self, pid: u32) -> Result<u32, RunnerError>;
    /// The start identity of `pid`.
    ///
    /// # Errors
    /// - `RunnerError::Refused` if native process metadata cannot be read or a custom provider refuses the lookup.
    /// - `RunnerError::Unsigned` if supplied by a custom process provider; the provider error is returned unchanged.
    /// - `RunnerError::Malformed` if supplied by a custom process provider; the provider error is returned unchanged.
    /// - `RunnerError::ProtocolMismatch` if supplied by a custom process provider; the provider error is returned unchanged.
    /// - `RunnerError::Replayed` if supplied by a custom process provider; the provider error is returned unchanged.
    /// - `RunnerError::Misaddressed` if supplied by a custom process provider; the provider error is returned unchanged.
    /// - `RunnerError::Unreachable` if supplied by a custom process provider; the provider error is returned unchanged.
    /// - `RunnerError::ReplyMalformed` if supplied by a custom process provider; the provider error is returned unchanged.
    /// - `RunnerError::State` if supplied by a custom process provider; the provider error is returned unchanged.
    /// - `RunnerError::StateHeld` if supplied by a custom process provider; the provider error is returned unchanged.
    /// - `RunnerError::AlreadyRunning` if supplied by a custom process provider; the provider error is returned unchanged.
    /// - `RunnerError::Socket` if supplied by a custom process provider; the provider error is returned unchanged.
    /// - `RunnerError::Key` if supplied by a custom process provider; the provider error is returned unchanged.
    /// - `RunnerError::DialStale` if supplied by a custom process provider; the provider error is returned unchanged.
    /// - `RunnerError::Dial` if supplied by a custom process provider; the provider error is returned unchanged.
    fn start(&self, pid: u32) -> Result<StartIdentity, RunnerError>;
}

/// This machine's processes.
#[derive(Debug, Clone, Copy, Default)]
pub struct System;

fn unproved(reason: impl Into<String>) -> RunnerError {
    RunnerError::refused("peer_unproved", reason)
}

include!("peer/processes.rs");

/// The session whose leader the peer `pid`, of user `uid`, descends from,
/// among `leaders`, as `processes` reads them. Refused `peer_unproved`,
/// with the reason, when the user is not `own`, when the ancestry breaks or
/// reaches no leader, or when the leader's start identity is not the one
/// recorded, before and after the walk alike.
///
/// # Errors
/// - `RunnerError::Refused` if users differ, ancestry cycles or reaches no leader, the leader start changes, or the process provider refuses a lookup.
/// - `RunnerError::Unsigned` if supplied by a custom process provider; the provider error is returned unchanged.
/// - `RunnerError::Malformed` if supplied by a custom process provider; the provider error is returned unchanged.
/// - `RunnerError::ProtocolMismatch` if supplied by a custom process provider; the provider error is returned unchanged.
/// - `RunnerError::Replayed` if supplied by a custom process provider; the provider error is returned unchanged.
/// - `RunnerError::Misaddressed` if supplied by a custom process provider; the provider error is returned unchanged.
/// - `RunnerError::Unreachable` if supplied by a custom process provider; the provider error is returned unchanged.
/// - `RunnerError::ReplyMalformed` if supplied by a custom process provider; the provider error is returned unchanged.
/// - `RunnerError::State` if supplied by a custom process provider; the provider error is returned unchanged.
/// - `RunnerError::StateHeld` if supplied by a custom process provider; the provider error is returned unchanged.
/// - `RunnerError::AlreadyRunning` if supplied by a custom process provider; the provider error is returned unchanged.
/// - `RunnerError::Socket` if supplied by a custom process provider; the provider error is returned unchanged.
/// - `RunnerError::Key` if supplied by a custom process provider; the provider error is returned unchanged.
/// - `RunnerError::DialStale` if supplied by a custom process provider; the provider error is returned unchanged.
/// - `RunnerError::Dial` if supplied by a custom process provider; the provider error is returned unchanged.
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
///
/// # Errors
/// - `RunnerError::Refused` with `peer_unproved` if peer credentials or native ancestry cannot be read, users differ, no leader is reached, or its start identity changes.
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

include!("peer/connection.rs");

/// Decode the outer JSON once; typed variants retain duplicate-field validation.
///
/// # Errors
/// - `RunnerError::Malformed` if the outer JSON, typed request or version field does not decode.
/// - `RunnerError::ProtocolMismatch` if the request names another version.
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
#[must_use]
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
