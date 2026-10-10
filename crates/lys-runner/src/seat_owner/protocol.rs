//! The typed owner command and its answers (AGENTS-004 R1), and where an
//! owner lives on disk.
//!
//! An owner is a runner of its own, serving one supervised session on its
//! own socket under the runner's state directory, so every ordinary act
//! (input, read, operate, status, stop) reaches the harness through the
//! runner protocol unchanged. What this module adds is the command a client
//! uses to bind to that owner: it names the seat, session, conversation and
//! generation it expects, and the process it is, which the owner proves at
//! the kernel before answering. A client that names another binding, a
//! stale generation or a process it is not is refused by name and the owner
//! goes on serving.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::record::{Cursors, Custody, Lease};
use super::rules::is_hex_id;
use crate::error::RunnerError;
use crate::peer::Leader;

/// The owner protocol's version, carried in every answer.
pub const OWNER_PROTOCOL: u32 = 1;

/// The build this binary announces as an owner.
pub const BUILD: &str = env!("CARGO_PKG_VERSION");

/// The build recorded for an owner that announced none.
pub const UNVERSIONED: &str = "unversioned";

fn unversioned() -> String {
    UNVERSIONED.to_owned()
}

/// The directory under the runner's state holding one directory per owner.
pub const OWNERS_DIR: &str = "owners";
/// The owner's socket, in its directory.
pub const SOCKET: &str = "owner.sock";
/// The owner's launch plan, in its directory, written before it starts.
pub const PLAN: &str = "launch.json";
/// The owner's own words, in its directory.
pub const LOG: &str = "owner.log";
/// The owner's runner state (its sessions record), in its directory.
pub const RUNNER_STATE: &str = "runner";

/// The longest session name an owner directory is made for.
pub const MAX_SESSION_BYTES: usize = 128;

/// What a client must name to bind: the seat AGENTS-002 started, its
/// session, its conversation and the generation of its owner lease.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnerBinding {
    /// The seat's public identity.
    pub seat: String,
    /// The session the owner serves.
    pub session: String,
    /// The harness conversation the session carries.
    pub conversation: String,
    /// The owner lease generation the client expects to find.
    pub generation: u64,
}

impl OwnerBinding {
    /// Refuses a binding that cannot name an owner directory or a lease.
    ///
    /// # Errors
    ///
    /// `seat_owner_binding_invalid`.
    pub fn validate(&self) -> Result<(), RunnerError> {
        for (what, name) in [
            ("seat", &self.seat),
            ("session", &self.session),
            ("conversation", &self.conversation),
        ] {
            if name.is_empty()
                || name.len() > MAX_SESSION_BYTES
                || name.trim() != name
                || name.contains('/')
                || name.starts_with('.')
                || name.chars().any(char::is_control)
            {
                return Err(RunnerError::refused(
                    "seat_owner_binding_invalid",
                    format!(
                        "the {what} is empty, padded, oversized, a path or holds a control character"
                    ),
                ));
            }
        }
        if self.generation == 0 {
            return Err(RunnerError::refused(
                "seat_owner_binding_invalid",
                "the generation is zero",
            ));
        }
        Ok(())
    }

    /// Whether `other` names the same seat, session and conversation, at
    /// any generation.
    #[must_use]
    pub fn same_seat(&self, other: &Self) -> bool {
        self.seat == other.seat
            && self.session == other.session
            && self.conversation == other.conversation
    }
}

/// Which client is binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClientKind {
    /// The runner that started the owner, or its replacement.
    Runner,
    /// The identity server admitting the seat's acts.
    IdentityServer,
    /// An upgrade's successor owner taking custody (R3).
    Successor,
}

/// The command a client sends an owner, inside the runner protocol's signed
/// request as [`crate::protocol::Act::Owner`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case", deny_unknown_fields)]
pub enum OwnerCommand {
    /// Bind as `client`, being the process `client_start`, to the owner
    /// `binding` names. The owner proves the process at the kernel.
    Hello {
        /// What the client expects to find.
        binding: OwnerBinding,
        /// Which client this is.
        client: ClientKind,
        /// The client's own process-start identity.
        client_start: Leader,
    },
    /// The owner's view of itself.
    Status,
    /// Move the session's durable receipt, feed and hook cursors forward,
    /// recorded as `intent` so a lost reply is resolved by the same id.
    Cursors {
        /// The move's intent id, 32 hexadecimal characters.
        intent: String,
        /// The cursors after the accepted work; none moves back.
        cursors: Cursors,
    },
    /// Stop the harness deliberately, under fresh authority (AGENTS-002 R3),
    /// recorded as `intent` before it is acted on.
    Stop {
        /// The stop's intent id, 32 hexadecimal characters.
        intent: String,
        /// Who pulled the cord, as the identity server verified them.
        by: String,
        /// Why, in their words.
        reason: String,
    },
    /// Prepare a handover to `successor` under `intent` (R3): custody goes
    /// to handing over; the lease does not move yet.
    Prepare {
        /// The handover's intent id.
        intent: String,
        /// The successor owner's process-start identity.
        successor: Leader,
    },
    /// Transfer the lease to the prepared successor at `generation` (R3),
    /// only after the successor confirmed custody and durability.
    Transfer {
        /// The handover's intent id, the same one that was prepared.
        intent: String,
        /// The successor's generation; it exceeds the owner's.
        generation: u64,
    },
    /// Give a prepared handover up under the same intent (R3): custody
    /// returns to owned and the owner keeps serving.
    Release {
        /// The handover's intent id.
        intent: String,
    },
}

impl OwnerCommand {
    /// Refuses a command whose members cannot be acted on.
    ///
    /// # Errors
    ///
    /// `seat_owner_binding_invalid` or `seat_owner_intent_invalid`.
    pub fn validate(&self) -> Result<(), RunnerError> {
        match self {
            Self::Hello {
                binding,
                client_start,
                ..
            } => {
                binding.validate()?;
                if client_start.pid <= 1 || client_start.start.0.is_empty() {
                    return Err(RunnerError::refused(
                        "seat_owner_binding_invalid",
                        "the client names no process-start identity",
                    ));
                }
                Ok(())
            }
            Self::Status | Self::Cursors { .. } => Ok(()),
            Self::Stop {
                intent, by, reason, ..
            } => {
                check_intent(intent)?;
                if by.is_empty() || reason.is_empty() {
                    return Err(RunnerError::refused(
                        "seat_owner_intent_invalid",
                        "a stop names who and why",
                    ));
                }
                Ok(())
            }
            Self::Prepare {
                intent, successor, ..
            } => {
                check_intent(intent)?;
                if successor.pid <= 1 || successor.start.0.is_empty() {
                    return Err(RunnerError::refused(
                        "seat_owner_intent_invalid",
                        "a handover names its successor's process-start identity",
                    ));
                }
                Ok(())
            }
            Self::Transfer { intent, generation } => {
                check_intent(intent)?;
                if *generation == 0 {
                    return Err(RunnerError::refused(
                        "seat_owner_intent_invalid",
                        "a transfer names a generation",
                    ));
                }
                Ok(())
            }
            Self::Release { intent } => check_intent(intent),
        }
    }

    /// Whether the command changes the owner's record, as opposed to
    /// reading it.
    #[must_use]
    pub fn writes(&self) -> bool {
        !matches!(self, Self::Status | Self::Hello { .. })
    }
}

fn check_intent(intent: &str) -> Result<(), RunnerError> {
    if is_hex_id(intent) {
        Ok(())
    } else {
        Err(RunnerError::refused(
            "seat_owner_intent_invalid",
            "an intent id is 32 lowercase hexadecimal characters",
        ))
    }
}

/// What an owner says about itself.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnerView {
    /// The owner protocol's version.
    pub protocol: u32,
    /// The binding the owner serves.
    pub binding: OwnerBinding,
    /// The owner process itself.
    pub owner: Leader,
    /// The owner's runner id, which signed requests to it must name.
    pub runner: String,
    /// The owner's socket.
    pub socket: PathBuf,
    /// The harness the owner holds, once spawned.
    pub harness: Option<Leader>,
    /// The lease as recorded.
    pub lease: Lease,
    /// The custody stage as recorded.
    pub custody: Custody,
    /// The durable cursors as recorded.
    pub cursors: Cursors,
}

/// The owner's answer, inside the runner protocol's reply as
/// [`crate::protocol::Answer::Owner`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "answer", rename_all = "snake_case", deny_unknown_fields)]
pub enum OwnerAnswer {
    /// The client is bound; here is the owner.
    Bound {
        /// The owner's view.
        view: OwnerView,
    },
    /// The owner's view, for a status read.
    Status {
        /// The owner's view.
        view: OwnerView,
    },
    /// The cursors moved.
    Cursors {
        /// The cursors as recorded.
        cursors: Cursors,
    },
    /// The stop is recorded and the harness is being ended.
    Stopping {
        /// The stop's intent id.
        intent: String,
    },
    /// The handover is prepared; the owner still holds the lease.
    Prepared {
        /// The handover's intent id.
        intent: String,
        /// The generation the successor will take.
        generation: u64,
    },
    /// The lease moved to the successor; this owner serves no new writes.
    Transferred {
        /// The handover's intent id.
        intent: String,
        /// The successor's generation.
        generation: u64,
    },
    /// The handover was given up; the owner serves as before.
    Released {
        /// The handover's intent id.
        intent: String,
    },
}

/// Where the owner of `session` lives under the runner's `state`.
#[must_use]
pub fn owner_dir(state: &Path, session: &str) -> PathBuf {
    state.join(OWNERS_DIR).join(session)
}

/// Where an owner, started or found, is reached.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnerEndpoint {
    /// The owner's directory.
    pub dir: PathBuf,
    /// The owner's socket.
    pub socket: PathBuf,
    /// The owner's runner id, as it announced it.
    pub runner: String,
    /// The owner process.
    pub owner: Leader,
    /// The build of the owner binary serving it, as it announced itself:
    /// a seat started before an upgrade keeps its original owner and
    /// says so (AGENTS-004 amendment 1).
    #[serde(default = "unversioned")]
    pub build: String,
}

impl OwnerEndpoint {
    /// The endpoint for an owner in `dir` that announced `runner` from the
    /// process `owner`.
    #[must_use]
    pub fn new(dir: PathBuf, runner: String, owner: Leader) -> Self {
        let socket = dir.join(SOCKET);
        Self {
            dir,
            socket,
            runner,
            owner,
            build: BUILD.to_owned(),
        }
    }
}

/// The line an owner writes on its standard output once it listens: the
/// runner reads it from the pipe it holds, and nothing else, as the ready
/// signal. Anything else on that pipe before it is the owner's refusal.
#[must_use]
pub fn ready_line(runner: &str, socket: &Path, owner: &Leader, build: &str) -> String {
    format!(
        "owner ready runner={runner} socket={} pid={} start={} build={}",
        socket.display(),
        owner.pid,
        owner.start.0,
        build.trim()
    )
}

/// Reads a ready line back.
///
/// # Errors
///
/// `seat_owner_start_failed` when the line is not a ready line.
pub fn parse_ready_line(line: &str) -> Result<(String, PathBuf, Leader), RunnerError> {
    let rest = line
        .trim_end()
        .strip_prefix("owner ready ")
        .ok_or_else(|| {
            RunnerError::refused(
                "seat_owner_start_failed",
                format!("the owner said {:?} before it was ready", line.trim_end()),
            )
        })?;
    let mut runner = None;
    let mut socket = None;
    let mut pid = None;
    let mut start = None;
    let mut build = UNVERSIONED.to_owned();
    for member in rest.split(' ') {
        match member.split_once('=') {
            Some(("runner", value)) => runner = Some(value.to_owned()),
            Some(("socket", value)) => socket = Some(PathBuf::from(value)),
            Some(("pid", value)) => pid = value.parse::<u32>().ok(),
            Some(("start", value)) => start = Some(value.to_owned()),
            Some(("build", value)) if !value.is_empty() => build = value.to_owned(),
            _ => {}
        }
    }
    match (runner, socket, pid, start) {
        (Some(runner), Some(socket), Some(pid), Some(start)) if pid > 1 && !start.is_empty() => {
            Ok((
                runner,
                socket,
                Leader {
                    pid,
                    start: crate::peer::StartIdentity(start),
                },
                build,
            ))
        }
        _ => Err(RunnerError::refused(
            "seat_owner_start_failed",
            "the owner's ready line names no runner, socket, pid or start",
        )),
    }
}
