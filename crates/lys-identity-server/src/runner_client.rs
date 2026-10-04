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
//! here ends a request on a clock.

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};

use lys_core::Ed25519Identity;
use lys_runner::protocol::{Greeting, read_reply, sign_request, unhex};
use lys_runner::{Act, Answer, RunnerError};
use serde::{Deserialize, Serialize};
use tokio::sync::Notify;

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
pub fn runner_id(text: &str) -> bool {
    text.len() == 32 && unhex(text).is_some()
}

/// The public key `text` spells.
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

/// A caller's leaving: what closes the request it was waiting on.
#[derive(Default)]
pub struct Leave {
    left: AtomicBool,
    hook: Mutex<Option<Hook>>,
}

impl Leave {
    /// Hold `hook` to run when the caller leaves; run at once when it has.
    pub fn hold(&self, hook: Hook) -> Result<(), RunnerError> {
        let mut held = match self.hook.lock() {
            Ok(held) => held,
            Err(error) => {
                hook();
                return Err(RunnerError::Unreachable {
                    reason: format!("caller leave hook unavailable: {error}"),
                });
            }
        };
        if self.left.load(Ordering::SeqCst) {
            drop(held);
            hook();
            return Ok(());
        }
        *held = Some(hook);
        Ok(())
    }

    /// The caller left.
    pub fn leave(&self) -> Result<(), RunnerError> {
        self.left.store(true, Ordering::SeqCst);
        let hook = self
            .hook
            .lock()
            .map_err(|error| RunnerError::Unreachable {
                reason: format!("caller leave hook unavailable: {error}"),
            })?
            .take();
        if let Some(hook) = hook {
            hook();
        }
        Ok(())
    }

    /// The request was answered: nothing is left to close.
    pub fn done(&self) -> Result<(), RunnerError> {
        drop(
            self.hook
                .lock()
                .map_err(|error| RunnerError::Unreachable {
                    reason: format!("caller leave hook unavailable: {error}"),
                })?
                .take(),
        );
        Ok(())
    }
}

/// Leaves when dropped: an HTTP handler holds one while it waits, so a
/// caller that goes drops it.
pub struct LeaveOnDrop(pub Arc<Leave>);

impl Drop for LeaveOnDrop {
    fn drop(&mut self) {
        if let Err(error) = self.0.leave() {
            tracing::error!("runner caller leave refused: {error}");
        }
    }
}

/// What a caller waiting on a dialled runner is answered: the reply line,
/// or why there is none.
type Replied = Result<String, RunnerError>;

/// A request waiting for a machine's bridge to take it.
struct Pending {
    ticket: String,
    act: Act,
    reply: mpsc::Sender<Replied>,
}

struct Queued {
    ticket: String,
    answer: mpsc::Receiver<Replied>,
    cancellation: mpsc::Sender<Replied>,
}

/// One machine's side of the hub.
#[derive(Default)]
struct Machine {
    queue: VecDeque<Pending>,
    notify: Arc<Notify>,
    delivered: HashMap<String, mpsc::Sender<Replied>>,
    nonces: HashSet<String>,
    /// How many of the machine's bridges are dialled in now, each waiting
    /// for its next request.
    bridges: usize,
}

/// A bridge dialled in and waiting: counted while its ask is held, and no
/// longer once it is answered or the bridge leaves.
struct Bridged<'a> {
    hub: &'a DialHub,
    machine: &'a str,
}

impl Drop for Bridged<'_> {
    fn drop(&mut self) {
        let left = self.hub.with(self.machine, |held| {
            held.bridges = held.bridges.saturating_sub(1);
        });
        if let Err(error) = left {
            tracing::error!("runner bridge count unavailable: {error}");
        }
    }
}

/// Where requests for dialled runners wait for their machine's bridge.
///
/// A dial is signed under the hub's epoch, made fresh each time the server
/// starts, and each of its nonces is admitted once within it. A dial
/// captured under an earlier epoch names an epoch this hub never had, so
/// nothing need be remembered across a restart to refuse it.
pub struct DialHub {
    epoch: String,
    machines: Mutex<HashMap<String, Machine>>,
}

impl Default for DialHub {
    fn default() -> Self {
        Self {
            epoch: lys_runner::protocol::hex(&rand::random::<[u8; 32]>()),
            machines: Mutex::default(),
        }
    }
}

impl DialHub {
    fn with<T>(
        &self,
        machine: &str,
        act: impl FnOnce(&mut Machine) -> T,
    ) -> Result<T, RunnerError> {
        let mut machines = self
            .machines
            .lock()
            .map_err(|error| RunnerError::Unreachable {
                reason: format!("runner dial hub unavailable: {error}"),
            })?;
        Ok(act(machines.entry(machine.to_owned()).or_default()))
    }

    /// The epoch every dial to this server is signed under.
    pub fn epoch(&self) -> &str {
        &self.epoch
    }

    /// Whether `nonce` is new for `machine` in this epoch, holding it.
    pub fn fresh(&self, machine: &str, nonce: &str) -> Result<bool, ServerError> {
        Ok(self.with(machine, |held| held.nonces.insert(nonce.to_owned()))?)
    }

    /// Queue `act` for `machine`'s bridge, answering the ticket and what
    /// its reply arrives on.
    fn queue(&self, machine: &str, act: Act) -> Result<Queued, RunnerError> {
        let (reply, answer) = mpsc::channel();
        let cancellation = reply.clone();
        let ticket = lys_runner::protocol::nonce();
        self.with(machine, |held| {
            held.queue.push_back(Pending {
                ticket: ticket.clone(),
                act,
                reply,
            });
            held.notify.notify_one();
        })?;
        Ok(Queued {
            ticket,
            answer,
            cancellation,
        })
    }

    /// Withdraw `ticket` for `machine`, whether still waiting or taken: its
    /// caller left, so its reply has no one to reach.
    fn withdraw(&self, machine: &str, ticket: &str) -> Result<(), RunnerError> {
        self.with(machine, |held| {
            held.queue.retain(|pending| pending.ticket != ticket);
            held.delivered.remove(ticket);
        })
    }

    /// Whether `machine`'s runner is dialled in now: a bridge of its waits
    /// for its next request, or holds one it has not yet answered. A runner
    /// that is not is never asked, since nothing would take the request.
    pub fn bridged(&self, machine: &str) -> Result<bool, RunnerError> {
        self.with(machine, |held| {
            held.bridges > 0 || !held.delivered.is_empty()
        })
    }

    /// The next act for `machine` and its ticket, once there is one.
    pub async fn next(&self, machine: &str) -> Result<(String, Act), ServerError> {
        self.with(machine, |held| held.bridges += 1)?;
        let _waiting = Bridged { hub: self, machine };
        loop {
            let (taken, notify) = self.with(machine, |held| {
                let taken = held.queue.pop_front().map(|pending| {
                    held.delivered.insert(pending.ticket.clone(), pending.reply);
                    (pending.ticket, pending.act)
                });
                (taken, Arc::clone(&held.notify))
            })?;
            if let Some(taken) = taken {
                return Ok(taken);
            }
            notify.notified().await;
        }
    }

    /// Hand `reply` to the caller waiting on `ticket`.
    pub fn reply(&self, machine: &str, ticket: &str, reply: Replied) -> Result<(), ServerError> {
        let waiting = self.with(machine, |held| held.delivered.remove(ticket))?;
        let waiting = waiting.ok_or_else(|| ServerError::DialRefused {
            reason: format!("no request of machine `{machine}` waits on ticket `{ticket}`"),
        })?;
        if waiting.send(reply).is_err() {
            return Err(ServerError::Runner {
                refusal: "caller_left".to_owned(),
                words: "the caller left before the runner answered".to_owned(),
            });
        }
        Ok(())
    }
}

fn withdraw_on_leave(
    hub: Arc<DialHub>,
    machine: String,
    ticket: String,
    cancellation: mpsc::Sender<Replied>,
) -> Hook {
    Box::new(move || {
        if let Err(error) = hub.withdraw(&machine, &ticket) {
            tracing::error!("runner request withdrawal refused: {error}");
            if let Err(undelivered) = cancellation.send(Err(error)) {
                tracing::error!(
                    "runner withdrawal refusal could not reach its caller: {undelivered}"
                );
            }
        }
    })
}

/// How the server reaches runners.
pub struct Runners {
    key: Arc<Ed25519Identity>,
    own: Option<PathBuf>,
    hub: Arc<DialHub>,
}

impl Runners {
    /// Runners reached with requests signed by `key`, this install's own at
    /// `own` when the configuration names it.
    pub fn new(key: Arc<Ed25519Identity>, own: Option<PathBuf>) -> Self {
        Self {
            key,
            own,
            hub: Arc::new(DialHub::default()),
        }
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
    pub fn sign(&self, greeting: &Greeting, act: &Act) -> Result<String, RunnerError> {
        sign_request(&self.key, greeting, act)
    }

    /// Ask `act` of `machine`'s runner, as `record` names it, until it
    /// answers or the caller leaves through `leave`. Blocks: run it off the
    /// async workers.
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
                let Queued {
                    ticket,
                    answer,
                    cancellation,
                } = self.hub.queue(machine, act.clone())?;
                let (hub, machine) = (Arc::clone(&self.hub), machine.to_owned());
                leave.hold(withdraw_on_leave(hub, machine, ticket, cancellation))?;
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

    /// Hold a grant channel to `record`'s runner. A runner that dials in
    /// holds no connection the server can keep, and is refused by name:
    /// its grantable rules are then denied `grant_state_unavailable`.
    pub fn grant_channel(
        &self,
        record: &RunnerRecord,
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
        lys_runner::connect(&socket)?.grant_channel(&self.key)
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

/// Ask `act` of `machine`'s runner off the async workers; a caller that
/// leaves closes the request.
pub async fn ask(
    state: &Arc<crate::routes::AppState>,
    machine: &str,
    record: RunnerRecord,
    act: Act,
) -> Result<Answer, ServerError> {
    let leave = Arc::new(Leave::default());
    let guard = LeaveOnDrop(Arc::clone(&leave));
    let (shared, machine) = (Arc::clone(state), machine.to_owned());
    let asked =
        tokio::task::spawn_blocking(move || shared.runners.ask(&machine, &record, &act, &leave))
            .await
            .map_err(|failed| ServerError::Runner {
                refusal: "runner_unreachable".to_owned(),
                words: format!("the request to the runner ended abnormally: {failed}"),
            })?;
    guard.0.done()?;
    drop(guard);
    Ok(asked?)
}

#[cfg(test)]
#[path = "runner_client_poison_tests.rs"]
mod poison_tests;
