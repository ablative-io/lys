//! Bounded replay memory and short handoff leases for dialled machines.

use super::{Act, Replied, RunnerError, ServerError};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, Instant};
use tokio::sync::Notify;

const HANDOFF: Duration = Duration::from_secs(2);
const NONCES: usize = 65_536;
const EPOCH_SECONDS: u64 = 300;

/// A request waiting for a machine's bridge to take it.
pub(super) struct Pending {
    ticket: String,
    act: Act,
    reply: mpsc::Sender<Replied>,
    taken: mpsc::Sender<()>,
}

pub(super) struct Queued {
    pub(super) ticket: String,
    pub(super) answer: mpsc::Receiver<Replied>,
    pub(super) cancellation: mpsc::Sender<Replied>,
    pub(super) taken: mpsc::Receiver<()>,
}

/// One machine's side of the hub.
#[derive(Default)]
pub(super) struct Machine {
    pub(super) queue: VecDeque<Pending>,
    notify: Arc<Notify>,
    pub(super) delivered: HashMap<String, mpsc::Sender<Replied>>,
    pub(super) nonces: HashSet<String>,
    nonce_epoch: String,
    last_contact: Option<Instant>,
    /// How many of the machine's bridges are dialled in now, each waiting
    /// for its next request.
    pub(super) bridges: usize,
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
    started: Instant,
    pub(super) machines: Mutex<HashMap<String, Machine>>,
}

impl Default for DialHub {
    fn default() -> Self {
        Self {
            epoch: lys_runner::protocol::hex(&rand::random::<[u8; 32]>()),
            started: Instant::now(),
            machines: Mutex::default(),
        }
    }
}

impl DialHub {
    pub(super) fn with<T>(
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
    pub fn epoch(&self) -> String {
        format!(
            "{}-{:x}",
            self.epoch,
            self.started.elapsed().as_secs() / EPOCH_SECONDS
        )
    }

    /// Whether `nonce` is new for `machine` in this epoch, holding it.
    ///
    /// # Errors
    /// Returns a refusal when the dial hub lock is unavailable.
    pub fn fresh(&self, machine: &str, nonce: &str) -> Result<bool, ServerError> {
        self.fresh_in(machine, nonce, &self.epoch())
    }

    /// Admit a nonce only within the still-current signed epoch.
    ///
    /// # Errors
    /// Names retired epochs, exhausted replay capacity, or unavailable hub state.
    pub fn fresh_in(&self, machine: &str, nonce: &str, epoch: &str) -> Result<bool, ServerError> {
        self.with(machine, |held| {
            let current = self.epoch();
            if epoch != current {
                return Err(crate::error_machine::MachineError::DialStale {
                    reason: "the signed dial epoch has retired; read the current epoch".to_owned(),
                }.into());
            }
            if held.nonce_epoch != current {
                held.nonces.clear();
                held.nonce_epoch = current;
            }
            if held.nonces.contains(nonce) { return Ok(false); }
            if held.nonces.len() >= NONCES {
                return Err(ServerError::Runner {
                    refusal: "runner_dial_capacity".to_owned(),
                    words: "this computer exhausted this epoch's dial capacity; reconnect in the next epoch".to_owned(),
                });
            }
            Ok(held.nonces.insert(nonce.to_owned()))
        })?
    }

    /// Queue `act` for `machine`'s bridge, answering the ticket and what
    /// its reply arrives on.
    pub(super) fn queue(&self, machine: &str, act: Act) -> Result<Queued, RunnerError> {
        let (reply, answer) = mpsc::channel();
        let cancellation = reply.clone();
        let (taken, received) = mpsc::channel();
        let ticket = lys_runner::protocol::nonce();
        self.with(machine, |held| {
            if !present(held) {
                return Err(RunnerError::Unreachable {
                    reason: "this computer's runner is not connected; reconnect it before starting"
                        .to_owned(),
                });
            }
            held.queue.push_back(Pending {
                ticket: ticket.clone(),
                act,
                reply,
                taken,
            });
            held.notify.notify_one();
            Ok(())
        })??;
        Ok(Queued {
            ticket,
            answer,
            cancellation,
            taken: received,
        })
    }

    /// Withdraw `ticket` for `machine`, whether still waiting or taken: its
    /// caller left, so its reply has no one to reach.
    pub(super) fn withdraw(&self, machine: &str, ticket: &str) -> Result<(), RunnerError> {
        self.with(machine, |held| {
            held.queue.retain(|pending| pending.ticket != ticket);
            held.delivered.remove(ticket);
        })
    }

    /// Whether `machine`'s runner is dialled in now: a bridge of its waits
    /// for its next request, or holds one it has not yet answered. A runner
    /// that is not is never asked, since nothing would take the request.
    ///
    /// # Errors
    /// Returns a refusal when the dial hub lock is unavailable.
    pub fn bridged(&self, machine: &str) -> Result<bool, RunnerError> {
        self.with(machine, |held| present(held))
    }

    /// The next act for `machine` and its ticket, once there is one.
    ///
    /// # Errors
    /// Returns dial hub failures while registering or taking the next request.
    pub async fn next(&self, machine: &str) -> Result<(String, Act), ServerError> {
        self.with(machine, |held| {
            held.bridges += 1;
            held.last_contact = Some(Instant::now());
        })?;
        let _waiting = Bridged { hub: self, machine };
        loop {
            let (taken, notify) = self.with(machine, |held| {
                let taken = held.queue.pop_front().and_then(|pending| {
                    held.last_contact = Some(Instant::now());
                    if pending.taken.send(()).is_err() {
                        return None;
                    }
                    held.delivered.insert(pending.ticket.clone(), pending.reply);
                    Some((pending.ticket, pending.act))
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
    ///
    /// # Errors
    /// Refuses unavailable hub state, an unknown ticket or a departed caller.
    pub fn reply(&self, machine: &str, ticket: &str, reply: Replied) -> Result<(), ServerError> {
        let waiting = self.with(machine, |held| {
            held.last_contact = Some(Instant::now());
            held.delivered.remove(ticket)
        })?;
        let waiting = waiting.ok_or_else(|| {
            crate::error_machine::MachineError::dial_refused(format!(
                "no request of machine `{machine}` waits on ticket `{ticket}`"
            ))
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

fn present(held: &Machine) -> bool {
    held.bridges > 0 || held.last_contact.is_some_and(|at| at.elapsed() <= HANDOFF)
}
