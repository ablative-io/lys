//! The runner protocol, version 1: what any runner speaks, Lys's own or
//! another tool's.
//!
//! A connection carries exactly one greeting, one request and one reply,
//! each one line of JSON ending in a newline. The caller gives up on a
//! request by closing the connection; nothing on either side ends one on a
//! clock.
//!
//! The runner speaks first: its greeting is
//! `{"version":1,"runner":..,"challenge":..}`, naming the runner's own id,
//! which it keeps for as long as its state directory lives, and a challenge
//! of 32 random bytes made for this connection alone. The request is
//! `{"version":1,"runner":..,"challenge":..,"act":..,"signature":..}`. `act`
//! is the act's JSON as a string, so the bytes signed are the bytes sent,
//! and `signature` is the lowercase hex of the Ed25519 signature, by the
//! server's key, over [`REQUEST_DOMAIN`], the version, the runner, the
//! challenge and the act, each followed by a newline except the last.
//!
//! A runner answers only a request its server's key verifies, that names
//! this runner (else `runner_request_misaddressed`) and the challenge it
//! gave on this connection (else `runner_request_replayed`). So a request
//! is answered at most once, by the one runner it was made for: a request
//! captured on its way is bound to a challenge no later connection is given,
//! in this run of the runner or any later one, and to a runner no other
//! runner is, even one trusting the same server key. Nothing is remembered
//! to refuse a replay: the challenge is what cannot recur.
//!
//! The acts are `start`, `input`, `keys`, `read`, `wait`, `resize`, `end`,
//! `status` and `stop_everything`, among others, tagged by `act`. A reply is `{"version":1,"answer":..}`,
//! the answer tagged by `kind`; a refusal is the kind `refused` with its
//! name and words. A reply in another version is refused
//! `runner_protocol_mismatch` before anything else in it is read.

use std::collections::BTreeMap;

use lys_core::Ed25519Identity;
use serde::{Deserialize, Serialize};

use crate::admitted::{Admitted, JudgedUnder};
use crate::error::RunnerError;
pub use crate::protocol_key::Key;
pub use crate::protocol_request::Request;
pub use crate::protocol_stop::Stopped;
use crate::rotation::{Move, Rotation};
pub use lys_home::harness::lys_mcp::LysMcp;

/// The protocol version this crate speaks.
pub const PROTOCOL_VERSION: u32 = 1;

/// The domain every request signature is made under.
pub const REQUEST_DOMAIN: &str = "lys/runner-request/v1";

/// What a session is started with.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Launch {
    /// The session's id, chosen by the server.
    pub session: String,
    /// The program run.
    pub program: String,
    /// Its arguments.
    pub arguments: Vec<String>,
    /// The directory it runs in: the launch's named folder, else the agent's
    /// own default from its profile. Never empty; a runner refuses an empty
    /// one by name (`launch_without_directory`) rather than starting the run
    /// in its own folder or the login's home.
    pub directory: String,
    /// The variables set in its environment, beside the runner's own. A
    /// credential never rides here: only a handle's id does.
    #[serde(default)]
    pub environment: BTreeMap<String, String>,
    /// Native config files carried in this signed launch.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config: Option<crate::launch_config::Config>,
    /// The terminal's width in columns.
    pub columns: u16,
    /// The terminal's height in rows.
    pub rows: u16,
    /// The accounts the session moves between at a usage limit, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rotation: Option<Rotation>,
    /// The agent's tool-boundary policy and the digest it was admitted
    /// under, when the session is judged at its tool boundary.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub policy: Option<Box<Admitted>>,
}

mod acts;
pub use acts::{Act, Answer, TrackingSpec};

/// How a session ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EndedHow {
    /// Its process exited, and the runner saw it.
    Exited,
    /// The runner was restarted while holding it; the restart found it gone.
    EndedByRunnerRestart,
    /// It reached a usage limit on the last account of its list.
    AccountsExhausted,
    /// It was stopped by a stop of everything, or by the runner's own stop;
    /// `stopped` names who and why.
    Stopped,
}

/// A session's end.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ended {
    /// How it ended.
    pub how: EndedHow,
    /// When, in milliseconds since the Unix epoch: the exit seen, or the
    /// instant a restart found it gone.
    pub at: u64,
    /// The exit status, when an exit was seen; never invented.
    pub status: Option<u32>,
    /// The signal that ended it, when one did.
    pub signal: Option<String>,
    /// Why a restart did not signal a group it could not prove it owned.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Who stopped it and why, when a stop of everything ended it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stopped: Option<crate::protocol_stop::Stopped>,
}

/// Output read from a session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Output {
    /// The session.
    pub session: String,
    /// The cursor the text begins at.
    pub from: u64,
    /// The cursor to read on from.
    pub cursor: u64,
    /// The oldest cursor still kept.
    pub oldest: u64,
    /// The text, whole characters only; bytes that do not read as text are
    /// shown as the replacement character.
    pub text: String,
    /// Its end, once it has ended.
    pub ended: Option<Ended>,
}

/// One session as the runner holds it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionView {
    /// A known adapter's unqualified version, when it has one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub qualification: Option<String>,
    /// The reviewed and started seat digests, when this is a seat program.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seat_program: Option<serde_json::Value>,
    /// The session.
    pub session: String,
    /// Its process id, while one is known.
    pub pid: Option<u32>,
    /// When it started, in milliseconds since the Unix epoch.
    pub started_at: u64,
    /// Width in columns.
    pub columns: u16,
    /// Height in rows.
    pub rows: u16,
    /// The oldest cursor still kept.
    pub oldest: u64,
    /// The cursor after the last byte of output.
    pub cursor: u64,
    /// The account handle in use, when it rotates; never a value.
    pub account: Option<String>,
    /// Every move between accounts.
    pub moves: Vec<Move>,
    /// Its end, once it has ended.
    pub ended: Option<Ended>,
    /// The policy its tool boundary is judged under, when its launch
    /// carried one; none for a session a restarted runner reports.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub policy: Option<JudgedUnder>,
    /// Current managed boundary and occurrence identifiers, without saved words.
    pub control: Option<crate::harness_control::ControlStatus>,
}

/// What a runner is and holds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatusView {
    /// The runner, in its own words.
    pub runner: String,
    /// The protocol it speaks.
    pub protocol: u32,
    /// The sessions it holds, running and ended.
    pub sessions: Vec<SessionView>,
}

/// What a runner says first on every connection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Greeting {
    /// The protocol version.
    pub version: u32,
    /// The runner's own id.
    pub runner: String,
    /// Made for this connection alone.
    pub challenge: String,
}

impl Greeting {
    /// The greeting of runner `runner`, with a fresh challenge.
    #[must_use]
    pub fn fresh(runner: &str) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            runner: runner.to_owned(),
            challenge: hex(&rand::random::<[u8; 32]>()),
        }
    }

    /// Whether the runner and challenge are lowercase hex, the challenge 32
    /// bytes, so neither can carry the newline that ends a signed field.
    #[must_use]
    pub fn well_formed(&self) -> bool {
        unhex(&self.runner).is_some_and(|bytes| !bytes.is_empty())
            && unhex(&self.challenge).is_some_and(|bytes| bytes.len() == 32)
    }

    /// The greeting's line.
    #[must_use]
    pub fn line(&self) -> String {
        format!(
            "{{\"version\":{},\"runner\":{},\"challenge\":{}}}",
            self.version,
            serde_json::Value::from(self.runner.as_str()),
            serde_json::Value::from(self.challenge.as_str())
        )
    }
}

/// A reply as it is sent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reply {
    /// The protocol version.
    pub version: u32,
    /// The answer.
    pub answer: Answer,
}

#[derive(Deserialize)]
struct Versioned {
    version: u32,
}

include!("protocol/encoding.rs");

/// The greeting `line` carries, refused `runner_protocol_mismatch` when it
/// is in another version before anything else in it is read.
///
/// # Errors
/// - `RunnerError::ReplyMalformed` if greeting JSON or its runner/challenge shape is invalid.
/// - `RunnerError::ProtocolMismatch` if the greeting names another version.
pub fn read_greeting(line: &str) -> Result<Greeting, RunnerError> {
    let versioned: Versioned =
        serde_json::from_str(line).map_err(|error| RunnerError::ReplyMalformed {
            reason: format!("the greeting is not the protocol's JSON: {error}"),
        })?;
    if versioned.version != PROTOCOL_VERSION {
        return Err(RunnerError::ProtocolMismatch {
            theirs: versioned.version,
            ours: PROTOCOL_VERSION,
        });
    }
    let greeting: Greeting =
        serde_json::from_str(line).map_err(|error| RunnerError::ReplyMalformed {
            reason: format!("the greeting does not read: {error}"),
        })?;
    if !greeting.well_formed() {
        return Err(RunnerError::ReplyMalformed {
            reason:
                "the greeting's runner and challenge are not lowercase hex, the challenge 32 bytes"
                    .to_owned(),
        });
    }
    Ok(greeting)
}

/// The request line for `act` to the runner that gave `greeting`, signed by
/// `key` over its runner and challenge.
///
/// # Errors
/// - `RunnerError::ReplyMalformed` if the greeting runner/challenge shape is invalid.
/// - `RunnerError::Malformed` if the act or signed request cannot be encoded.
pub fn sign_request(
    key: &Ed25519Identity,
    greeting: &Greeting,
    act: &Act,
) -> Result<String, RunnerError> {
    if !greeting.well_formed() {
        return Err(RunnerError::ReplyMalformed {
            reason: "the greeting's runner and challenge are not lowercase hex: nothing is signed over it".to_owned(),
        });
    }
    let act = serde_json::to_string(act).map_err(|error| RunnerError::Malformed {
        reason: format!("the act does not write: {error}"),
    })?;
    let signature = key.sign(&signed_bytes(
        PROTOCOL_VERSION,
        &greeting.runner,
        &greeting.challenge,
        &act,
    ));
    let request = Request {
        version: PROTOCOL_VERSION,
        runner: greeting.runner.clone(),
        challenge: greeting.challenge.clone(),
        act,
        signature: hex(&signature),
    };
    serde_json::to_string(&request).map_err(|error| RunnerError::Malformed {
        reason: format!("the request does not write: {error}"),
    })
}

/// The act `line` asks for, once its version is this protocol's, it names
/// the runner and challenge of `greeting`, and its signature verifies
/// against `server`.
///
/// # Errors
/// - `RunnerError::Malformed` if request or act JSON is invalid.
/// - `RunnerError::ProtocolMismatch` if the request names another version.
/// - `RunnerError::Unsigned` if the signature is absent, invalid hex, or does not verify.
/// - `RunnerError::Misaddressed` if the request names another runner.
/// - `RunnerError::Replayed` if the request challenge differs from this connection.
pub fn verify_request(
    line: &str,
    server: &[u8; 32],
    greeting: &Greeting,
) -> Result<Act, RunnerError> {
    let versioned: Versioned =
        serde_json::from_str(line).map_err(|error| RunnerError::Malformed {
            reason: format!("the request is not the protocol's JSON: {error}"),
        })?;
    if versioned.version != PROTOCOL_VERSION {
        return Err(RunnerError::ProtocolMismatch {
            theirs: versioned.version,
            ours: PROTOCOL_VERSION,
        });
    }
    let request: Request = serde_json::from_str(line).map_err(|error| RunnerError::Malformed {
        reason: format!("the request does not read: {error}"),
    })?;
    verify_parsed(&request, server, greeting)
}

/// Verify a decoded server request without reparsing its outer JSON.
///
/// # Errors
/// - `RunnerError::Malformed` if request or act JSON is invalid.
/// - `RunnerError::ProtocolMismatch` if the request names another version.
/// - `RunnerError::Unsigned` if the signature is absent, invalid hex, or does not verify.
/// - `RunnerError::Misaddressed` if the request names another runner.
/// - `RunnerError::Replayed` if the request challenge differs from this connection.
pub fn verify_parsed(
    request: &Request,
    server: &[u8; 32],
    greeting: &Greeting,
) -> Result<Act, RunnerError> {
    if request.version != PROTOCOL_VERSION {
        return Err(RunnerError::ProtocolMismatch {
            theirs: request.version,
            ours: PROTOCOL_VERSION,
        });
    }
    if request.signature.is_empty() {
        return Err(RunnerError::Unsigned {
            reason: "the request carries no signature".to_owned(),
        });
    }
    if request.runner != greeting.runner {
        return Err(RunnerError::Misaddressed {
            reason: format!(
                "the request names runner `{}`, and this is runner `{}`",
                request.runner, greeting.runner
            ),
        });
    }
    if request.challenge != greeting.challenge {
        return Err(RunnerError::Replayed {
            reason: "the request names a challenge this connection was not given".to_owned(),
        });
    }
    let signature = unhex(&request.signature).ok_or_else(|| RunnerError::Unsigned {
        reason: "the signature is not lowercase hex".to_owned(),
    })?;
    let message = signed_bytes(
        request.version,
        &request.runner,
        &request.challenge,
        &request.act,
    );
    Ed25519Identity::verify(server, &message, &signature).map_err(|_refused| {
        RunnerError::Unsigned {
            reason: "the signature is not the server's".to_owned(),
        }
    })?;
    serde_json::from_str(&request.act).map_err(|error| RunnerError::Malformed {
        reason: format!(
            "the act does not read at line {}, column {}",
            error.line(),
            error.column()
        ),
    })
}

/// The reply line for `answer`.
#[must_use]
pub fn reply_line(answer: Answer) -> String {
    let reply = Reply {
        version: PROTOCOL_VERSION,
        answer,
    };
    serde_json::to_string(&reply).unwrap_or_else(|error| {
        format!(
            "{{\"version\":{PROTOCOL_VERSION},\"answer\":{{\"kind\":\"refused\",\"refusal\":\"runner_reply_malformed\",\"words\":\"the reply does not write: {}\"}}}}",
            error.to_string().replace(['"', '\\'], "'")
        )
    })
}

/// The answer `line` carries, refused `runner_protocol_mismatch` when it is
/// in another version, and as its own refusal when the runner refused.
///
/// # Errors
/// - `RunnerError::ReplyMalformed` if reply JSON or its answer shape is invalid.
/// - `RunnerError::ProtocolMismatch` if the reply names another version.
/// - `RunnerError::Refused` if the runner returns a named refusal, preserving its words and oldest cursor.
pub fn read_reply(line: &str) -> Result<Answer, RunnerError> {
    let versioned: Versioned =
        serde_json::from_str(line).map_err(|error| RunnerError::ReplyMalformed {
            reason: format!("the reply is not the protocol's JSON: {error}"),
        })?;
    if versioned.version != PROTOCOL_VERSION {
        return Err(RunnerError::ProtocolMismatch {
            theirs: versioned.version,
            ours: PROTOCOL_VERSION,
        });
    }
    let reply: Reply = serde_json::from_str(line).map_err(|error| RunnerError::ReplyMalformed {
        reason: format!("the reply does not read: {error}"),
    })?;
    match reply.answer {
        Answer::Refused {
            refusal,
            words,
            oldest,
        } => Err(RunnerError::Refused {
            refusal,
            words,
            oldest,
        }),
        answer => Ok(answer),
    }
}
