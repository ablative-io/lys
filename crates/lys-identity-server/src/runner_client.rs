//! The server's side of the runner protocol: how it reaches a machine's
//! runner, whichever runner that is.
//!
//! A machine's record names its runner ([`RunnerRecord`]): this install's
//! own, at the socket the configuration names; another tool speaking the
//! protocol on its own Unix socket; or a runner on another machine, which
//! dials this server through its bridge with the machine's own key. Every
//! request is signed with the service key over the greeting of the one
//! connection it is sent on, naming that runner and that connection's
//! challenge, so a runner acts only on this server's word, and only once. Nothing here is specific to any runner: a runner is its
//! socket and the protocol, and the protocol's version is checked on every
//! answer, a wrong one refused `runner_protocol_mismatch` by name.
//!
//! The server spawns nothing: it asks. A caller that leaves closes the
//! request it was waiting on, and the runner stops waiting too; nothing
//! here ends a request on a clock. A runner that dials in is asked only
//! while one of its bridges is dialled in now: a request for one that is
//! not would wait for a bridge that may never come, so it is refused at
//! once by name instead (14 Oct 2026: a machine in use whose runner had
//! stopped dialling held every survey of the runners open for good).

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use lys_core::Ed25519Identity;
use lys_runner::protocol::{Greeting, read_reply, sign_request, unhex};
use lys_runner::{Act, Answer, RunnerError};
use serde::{Deserialize, Serialize};

use crate::error::ServerError;

/// A machine's runner, as the machine's record names it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum RunnerRecord {
    /// This install's own runner, at the socket the configuration names.
    Lys,
    /// Another tool speaking the protocol on its own Unix socket.
    Socket {
        /// The socket's path.
        path: String,
    },
    /// A runner on another machine, reached through its dial bridge.
    Dialled {
        /// The machine's own Ed25519 public key, as 64 hexadecimal
        /// characters: every dial is signed with it.
        key: String,
        /// The id of the one runner the machine's bridge may relay for, as
        /// 32 hexadecimal characters: named here, or pinned by the first
        /// greeting the bridge carries. A greeting naming any other runner
        /// is refused, so a request made for this machine is signed for
        /// its own runner only. Naming the machine's runner again unpins it.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        runner: Option<String>,
    },
}

impl RunnerRecord {
    /// The record, refused by name when a socket is not an absolute path
    /// or a key is not a key.
    ///
    /// # Errors
    /// Refuses invalid socket paths and dial identities.
    pub fn checked(self) -> Result<Self, ServerError> {
        let malformed = |reason: &str| ServerError::RequestMalformed {
            reason: reason.to_owned(),
        };
        let refusal = match &self {
            Self::Socket { path } if !path.starts_with('/') => {
                Some("a runner's socket is an absolute path")
            }
            Self::Dialled { key, .. } if dial_key(key).is_none() => {
                Some("a dialled runner's key is an Ed25519 public key as 64 hexadecimal characters")
            }
            Self::Dialled {
                runner: Some(runner),
                ..
            } if !runner_id(runner) => {
                Some("a dialled runner's id is 32 lowercase hexadecimal characters")
            }
            _ => None,
        };
        if let Some(refusal) = refusal {
            return Err(malformed(refusal));
        }
        Ok(self)
    }
}

/// Whether `text` is a runner's id: 32 lowercase hexadecimal characters.
#[must_use]
pub fn runner_id(text: &str) -> bool {
    text.len() == 32 && unhex(text).is_some()
}

/// The public key `text` spells.
#[must_use]
pub fn dial_key(text: &str) -> Option<[u8; 32]> {
    unhex(text).and_then(|bytes| <[u8; 32]>::try_from(bytes).ok())
}

impl From<RunnerError> for ServerError {
    fn from(error: RunnerError) -> Self {
        let text = error.to_string();
        let words = text
            .split_once(": ")
            .map_or(text.as_str(), |(_, words)| words)
            .to_owned();
        Self::Runner {
            refusal: error.name(),
            words,
        }
    }
}

type Hook = Box<dyn FnOnce() + Send>;

mod leave;
pub use leave::{Leave, LeaveOnDrop};

/// What a caller waiting on a dialled runner is answered: the reply line,
/// or why there is none.
type Replied = Result<String, RunnerError>;

mod hub;
pub use hub::DialHub;
use hub::Queued;

/// How the server reaches runners.
pub struct Runners {
    key: Arc<Ed25519Identity>,
    own: Option<PathBuf>,
    hub: Arc<DialHub>,
    links: crate::runner_links::Links,
    starts: Mutex<HashMap<(String, String), usize>>,
}

impl Runners {
    /// Runners reached with requests signed by `key`, this install's own at
    /// `own` when the configuration names it.
    #[must_use]
    pub fn new(key: Arc<Ed25519Identity>, own: Option<PathBuf>) -> Self {
        Self {
            key,
            own,
            hub: Arc::new(DialHub::default()),
            links: crate::runner_links::Links::default(),
            starts: Mutex::new(HashMap::new()),
        }
    }

    /// Which standing links to runners are held ([`crate::runner_links`]).
    pub fn links(&self) -> &crate::runner_links::Links {
        &self.links
    }

    /// The public half of the key every request to a runner is signed with:
    /// the key a runner acts on, as `runner-server.pub` holds it.
    pub fn public_key(&self) -> [u8; 32] {
        self.key.public_key_bytes()
    }

    /// Where requests for dialled runners wait.
    pub fn hub(&self) -> &DialHub {
        &self.hub
    }

    /// The request line for `act` to the runner that gave `greeting`.
    ///
    /// # Errors
    /// Returns request encoding or signing failures.
    pub fn sign(&self, greeting: &Greeting, act: &Act) -> Result<String, RunnerError> {
        sign_request(&self.key, greeting, act)
    }

    /// Ask `act` of `machine`'s runner, as `record` names it, until it
    /// answers or the caller leaves through `leave`. Blocks: run it off the
    /// async workers.
    ///
    /// # Errors
    /// Returns connection, signing, protocol and caller cancellation failures.
    pub fn ask(
        &self,
        machine: &str,
        record: &RunnerRecord,
        act: &Act,
        leave: &Leave,
    ) -> Result<Answer, RunnerError> {
        let reply = match record {
            RunnerRecord::Lys => {
                let own = self.own.as_ref().ok_or_else(|| RunnerError::Unreachable {
                    reason:
                        "the configuration names no runner_socket for this install's own runner"
                            .to_owned(),
                })?;
                self.exchange_socket(own, act, leave)?
            }
            RunnerRecord::Socket { path } => {
                self.exchange_socket(&PathBuf::from(path), act, leave)?
            }
            RunnerRecord::Dialled { .. } => {
                if !self.hub.bridged(machine)? {
                    return Err(RunnerError::Unreachable {
                        reason: "the machine's runner is not dialled in to Lys, so nothing would take the request"
                            .to_owned(),
                    });
                }
                let Queued {
                    ticket,
                    answer,
                    cancellation,
                    taken,
                } = self.hub.queue(machine, act.clone())?;
                let (hub, machine) = (Arc::clone(&self.hub), machine.to_owned());
                leave.hold(withdraw_on_leave(hub, machine, ticket, cancellation))?;
                if taken
                    .recv_timeout(std::time::Duration::from_secs(2))
                    .is_err()
                {
                    leave.leave()?;
                    return Err(RunnerError::Unreachable {
                        reason:
                            "this computer's runner is not connected; reconnect it before starting"
                                .to_owned(),
                    });
                }
                let reply = answer.recv().map_err(|_left| RunnerError::Unreachable {
                    reason: "the request was withdrawn before the machine's runner answered"
                        .to_owned(),
                })?;
                leave.done()?;
                reply?
            }
        };
        read_reply(&reply)
    }

    /// Hold a grant channel to `record`'s runner, closed when its holder
    /// leaves. A runner that dials in holds no connection the server can
    /// keep, and is refused by name: its grantable rules are then denied
    /// `grant_state_unavailable`.
    ///
    /// # Errors
    /// Refuses missing sockets, dialled runners and connection or signing failures.
    pub fn grant_channel(
        &self,
        record: &RunnerRecord,
        leave: &Leave,
    ) -> Result<lys_runner::GrantChannel, RunnerError> {
        let socket = match record {
            RunnerRecord::Lys => self.own.clone().ok_or_else(|| RunnerError::Unreachable {
                reason: "the configuration names no runner_socket for this install's own runner"
                    .to_owned(),
            })?,
            RunnerRecord::Socket { path } => PathBuf::from(path),
            RunnerRecord::Dialled { .. } => {
                return Err(RunnerError::refused(
                    "grant_channel_undialled",
                    "a runner that dials in holds no grant channel",
                ));
            }
        };
        let connection = lys_runner::connect(&socket)?;
        let closer = connection.closer()?;
        leave.hold(Box::new(move || closer.close()))?;
        connection.grant_channel(&self.key)
    }

    /// Ask `act` of the runner on `socket`, signed over the greeting of the
    /// connection it is sent on. The closer is held before the greeting is
    /// read, so a caller that leaves ends the wait whatever the runner does.
    fn exchange_socket(
        &self,
        socket: &std::path::Path,
        act: &Act,
        leave: &Leave,
    ) -> Result<String, RunnerError> {
        let mut connection = lys_runner::connect(socket)?;
        let closer = connection.closer()?;
        leave.hold(Box::new(move || closer.close()))?;
        let reply = connection
            .greeting()
            .and_then(|greeting| self.sign(&greeting, act))
            .and_then(|line| connection.exchange(&line));
        leave.done()?;
        reply
    }
}

mod pending_starts;
/// Ask the chosen computer for its own programme evidence.
pub(crate) async fn programs(
    state: &Arc<crate::routes::AppState>,
    machine: &str,
) -> Result<lys_runner::dial::programs::Programs, ServerError> {
    let runner = crate::runner_sessions::machine_runner(state, machine)?.ok_or_else(|| {
        ServerError::Runner {
            refusal: "runner_unreachable".to_owned(),
            words: "this computer names no connected runner".to_owned(),
        }
    })?;
    if matches!(runner, RunnerRecord::Dialled { .. }) && !state.runners.hub().bridged(machine)? {
        return Err(ServerError::Runner {
            refusal: "runner_unreachable".to_owned(),
            words: "this computer's runner is not connected to Lys".to_owned(),
        });
    }
    match ask(state, machine, runner, Act::Programs).await? {
        Answer::Programs { found } => Ok(found),
        other => Err(ServerError::Runner {
            refusal: "runner_reply_malformed".to_owned(),
            words: format!(
                "programme discovery answered {}",
                crate::runner_sessions::kind(&other)
            ),
        }),
    }
}
pub(crate) mod observations;
mod session;
pub use session::ask;
pub(crate) use session::outcome;
use session::withdraw_on_leave;

#[cfg(test)]
#[path = "runner_client_dial_tests.rs"]
mod dial_tests;
#[cfg(test)]
#[path = "runner_client_poison_tests.rs"]
mod poison_tests;
