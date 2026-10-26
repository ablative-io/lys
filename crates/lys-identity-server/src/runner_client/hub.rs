//! Bounded replay memory and connection-owned presence for dialled machines.

use super::{Act, Replied, RunnerError, ServerError};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, Mutex, mpsc};
use std::time::Instant;
use tokio::sync::Notify;

#[path = "hub/connection_control.rs"]
mod connection_control;

const NONCES: usize = 65_536;
const FINISHED: usize = 4096;
const EPOCH_SECONDS: u64 = 300;

/// A request waiting for a machine's bridge to take it.
pub(super) struct Pending {
    ticket: String,
    act: Act,
    reply: mpsc::Sender<Replied>,
    taken: mpsc::Sender<()>,
}

pub(super) struct Queued<'hub> {
    pub(super) ticket: String,
    pub(super) answer: ReplyReceiver<'hub>,
    pub(super) cancellation: mpsc::Sender<Replied>,
    pub(super) taken: mpsc::Receiver<()>,
}

pub(super) struct ReplyReceiver<'hub> {
    hub: &'hub DialHub,
    machine: String,
    ticket: String,
    answer: mpsc::Receiver<Replied>,
}

impl ReplyReceiver<'_> {
    fn received(&self, answer: &Replied) {
        if let Ok(line) = answer {
            if !matches!(
                lys_runner::protocol::read_reply(line),
                Ok(_) | Err(RunnerError::Refused { .. })
            ) {
                return;
            }
        }
        if let Err(error) = self.hub.acknowledge_completion(&self.machine, &self.ticket) {
            tracing::error!("runner completion acknowledgement refused: {error}");
        }
    }

    pub(super) fn recv(&self) -> Result<Replied, mpsc::RecvError> {
        let answer = self.answer.recv()?;
        self.received(&answer);
        Ok(answer)
    }

    #[cfg(test)]
    pub(super) fn try_recv(&self) -> Result<Replied, mpsc::TryRecvError> {
        let answer = self.answer.try_recv()?;
        self.received(&answer);
        Ok(answer)
    }
}

/// One machine's side of the hub.
#[derive(Default)]
pub(super) struct Machine {
    pub(super) queue: VecDeque<Pending>,
    notify: Arc<Notify>,
    pub(super) delivered: HashMap<String, mpsc::Sender<Replied>>,
    pub(super) nonces: HashSet<String>,
    finished: HashMap<String, &'static str>,
    finished_order: VecDeque<String>,
    completions: HashMap<String, String>,
    control: connection_control::Control,
    nonce_epoch: String,
    /// How many of the machine's bridges are dialled in now, each waiting
    /// for its next request.
    pub(super) bridges: usize,
}

impl Machine {
    fn finish(&mut self, ticket: &str, refusal: &'static str) {
        if self.finished.insert(ticket.to_owned(), refusal).is_none() {
            self.finished_order.push_back(ticket.to_owned());
        }
        while self.finished_order.len() > FINISHED {
            if let Some(ticket) = self.finished_order.pop_front() {
                self.finished.remove(&ticket);
            }
        }
    }
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
    pub(super) fn queue(&self, machine: &str, act: Act) -> Result<Queued<'_>, RunnerError> {
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
            if held.completions.len() + held.queue.len() + held.delivered.len() >= FINISHED {
                return Err(RunnerError::refused(
                    "runner_completion_capacity",
                    "read and acknowledge completed replies before queuing more requests",
                ));
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
            answer: ReplyReceiver {
                hub: self,
                machine: machine.to_owned(),
                ticket: ticket.clone(),
                answer,
            },
            ticket,
            cancellation,
            taken: received,
        })
    }

    /// Withdraw `ticket` for `machine`, whether still waiting or taken: its
    /// caller left, so its reply has no one to reach.
    pub(super) fn withdraw(&self, machine: &str, ticket: &str) -> Result<(), RunnerError> {
        self.with(machine, |held| {
            let queued = held.queue.len();
            held.queue.retain(|pending| pending.ticket != ticket);
            if held.delivered.remove(ticket).is_some() || held.queue.len() != queued {
                held.finish(ticket, "runner_reply_expired");
            }
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
            held.notify.notify_waiters();
        })?;
        let _waiting = Bridged { hub: self, machine };
        let notify = self.with(machine, |held| Arc::clone(&held.notify))?;
        loop {
            // Register both waiting bridges before inspecting the queue, so a wake cannot be lost.
            let notified = notify.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            let taken = self.with(machine, |held| {
                while let Some(pending) = held.queue.pop_front() {
                    if pending.taken.send(()).is_err() {
                        held.finish(&pending.ticket, "runner_reply_expired");
                        continue;
                    }
                    held.delivered.insert(pending.ticket.clone(), pending.reply);
                    return Some((pending.ticket, pending.act));
                }
                None
            })?;
            if let Some(taken) = taken {
                return Ok(taken);
            }
            notified.await;
        }
    }

    /// Read the first completion recorded for a ticket without consuming it.
    ///
    /// # Errors
    /// Returns unavailable hub state by name.
    pub fn completion(&self, machine: &str, ticket: &str) -> Result<Option<String>, ServerError> {
        Ok(self.with(machine, |held| held.completions.get(ticket).cloned())?)
    }

    /// List retained tickets without exposing their reply payloads.
    ///
    /// # Errors
    /// Returns unavailable hub state by name.
    pub fn completion_tickets(&self, machine: &str) -> Result<Vec<String>, ServerError> {
        let mut tickets = self.with(machine, |held| {
            held.completions.keys().cloned().collect::<Vec<_>>()
        })?;
        tickets.sort();
        Ok(tickets)
    }

    /// Release a completion after its caller has explicitly acknowledged readback.
    ///
    /// # Errors
    /// Returns unavailable hub state by name.
    pub fn acknowledge_completion(&self, machine: &str, ticket: &str) -> Result<bool, ServerError> {
        Ok(self.with(machine, |held| {
            let received = held.completions.remove(ticket).is_some();
            if received {
                held.finish(ticket, "runner_reply_duplicate");
            }
            received
        })?)
    }

    /// Hand `reply` to the caller waiting on `ticket`.
    ///
    /// # Errors
    /// Refuses unavailable hub state, an unknown ticket or a departed caller.
    pub fn reply(&self, machine: &str, ticket: &str, reply: Replied) -> Result<(), ServerError> {
        self.with(machine, |held| {
            if held.finished.get(ticket) == Some(&"runner_reply_duplicate") && !held.completions.contains_key(ticket) {
                return Err(finished("runner_reply_duplicate"));
            }
            if !held.completions.contains_key(ticket) {
                if held.completions.len() >= FINISHED {
                    return Err(ServerError::Runner { refusal: "runner_completion_capacity".to_owned(), words: "the completion store is full; the bridge must keep this reply until a caller acknowledges an earlier completion".to_owned() });
                }
                let retained = match &reply {
                    Ok(line) => line.clone(),
                    Err(error) => lys_runner::protocol::reply_line(lys_runner::Answer::refusal(error)),
                };
                held.completions.insert(ticket.to_owned(), retained);
            }
            let Some(waiting) = held.delivered.remove(ticket) else {
                let refusal = held
                    .finished
                    .get(ticket)
                    .copied()
                    .unwrap_or("runner_reply_unknown");
                return Err(finished(refusal));
            };
            if waiting.send(reply).is_err() {
                held.finish(ticket, "runner_reply_expired");
                return Err(finished("runner_reply_expired"));
            }
            held.finish(ticket, "runner_reply_duplicate");
            Ok(())
        })?
    }
}

fn finished(refusal: &str) -> ServerError {
    let words = match refusal {
        "runner_reply_expired" => {
            "the caller left before the runner answered; do not send this reply again"
        }
        "runner_reply_duplicate" => {
            "this request was already answered; do not send this reply again"
        }
        _ => "this server holds no such request; do not send this reply again",
    };
    ServerError::Runner {
        refusal: refusal.to_owned(),
        words: words.to_owned(),
    }
}

fn present(held: &Machine) -> bool {
    held.bridges > 0 || held.control.connected()
}

#[cfg(test)]
#[path = "hub/late_completion_tests.rs"]
mod late_completion_tests;
