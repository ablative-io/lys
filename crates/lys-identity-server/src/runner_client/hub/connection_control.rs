//! Signed runner control stays live while delivery connections reconnect.

use super::{DialHub, ServerError};
use serde_json::{Value, json};
use std::sync::Arc;
use tokio::sync::Notify;

#[derive(Default)]
pub(super) struct Control {
    generation: u64,
    waiting: usize,
    status: Option<Value>,
    changed: Arc<Notify>,
}

impl Control {
    pub(super) fn connected(&self) -> bool {
        self.waiting > 0
    }
}

struct Waiting<'a>(&'a DialHub, &'a str);
impl Drop for Waiting<'_> {
    fn drop(&mut self) {
        if let Err(error) = self.0.with(self.1, |held| {
            held.control.waiting = held.control.waiting.saturating_sub(1);
            held.notify.notify_waiters();
            if held.control.waiting == 0 && held.bridges == 0 {
                for (_, reply) in held.delivered.drain() {
                    if let Err(error) = reply.send(Err(lys_runner::RunnerError::Unreachable {
                        reason: "the runner's connection closed".to_owned(),
                    })) {
                        tracing::debug!("runner_close_receiver_left: {error}");
                    }
                }
                held.queue.clear();
            }
        }) {
            tracing::error!("runner_control_release_failed: {error}");
        }
    }
}

impl DialHub {
    /// Registration, rather than a time window, precedes a controlled runner's reply.
    pub(crate) async fn replacement_bridge(&self, machine: &str) -> Result<(), ServerError> {
        let notify = self.with(machine, |held| Arc::clone(&held.notify))?;
        loop {
            let changed = notify.notified();
            tokio::pin!(changed);
            changed.as_mut().enable();
            if self.with(machine, |held| {
                held.bridges > 0 || !held.control.connected()
            })? {
                return Ok(());
            }
            changed.await;
        }
    }

    /// Retain a signed report of the runner's connection state.
    ///
    /// # Errors
    /// Refuses unavailable hub state.
    pub fn report_connection(&self, machine: &str, status: Value) -> Result<(), ServerError> {
        self.with(machine, |held| held.control.status = Some(status))?;
        Ok(())
    }

    /// Read the runner's last reported state, without inventing a retry schedule.
    ///
    /// # Errors
    /// Refuses unavailable hub state.
    pub fn connection_status(&self, machine: &str) -> Result<Value, ServerError> {
        Ok(self.with(machine, |held| {
            held.control
                .status
                .clone()
                .unwrap_or_else(|| json!({"state":"unreported", "retry":null}))
        })?)
    }

    /// Signal a connected runner's control stream immediately.
    ///
    /// # Errors
    /// Refuses when no control connection can receive the wake, or hub state is unavailable.
    pub fn reconnect(&self, machine: &str) -> Result<u64, ServerError> {
        self.with(machine, |held| {
            if held.control.waiting == 0 {
                return Err(ServerError::Runner { refusal:"runner_reconnect_unavailable".to_owned(), words:"This runner has no live control connection to Lys. Run lys runner reconnect on that computer to end its wait immediately.".to_owned() });
            }
            held.control.generation = held.control.generation.wrapping_add(1);
            held.control.changed.notify_waiters();
            Ok(held.control.generation)
        })?
    }

    /// Hold one signed control request until a person's reconnect action.
    ///
    /// # Errors
    /// Refuses unavailable hub state.
    pub async fn control_after(&self, machine: &str, after: u64) -> Result<u64, ServerError> {
        let notify = self.with(machine, |held| {
            held.control.waiting += 1;
            Arc::clone(&held.control.changed)
        })?;
        let waiting = Waiting(self, machine);
        loop {
            let changed = notify.notified();
            tokio::pin!(changed);
            changed.as_mut().enable();
            let generation = self.with(machine, |held| held.control.generation)?;
            if generation != after {
                drop(waiting);
                return Ok(generation);
            }
            changed.await;
        }
    }
}

impl super::super::Runners {
    /// Resolve a local runner's control socket from its configured record.
    ///
    /// # Errors
    /// Refuses an unconfigured local socket or a dialled record.
    pub fn connection_socket(
        &self,
        record: super::super::RunnerRecord,
    ) -> Result<std::path::PathBuf, ServerError> {
        match record {
            super::super::RunnerRecord::Socket { path } => Ok(path.into()),
            super::super::RunnerRecord::Lys => {
                self.own.clone().ok_or_else(|| ServerError::Runner {
                    refusal: "runner_socket_unavailable".to_owned(),
                    words: "This install names no local runner socket.".to_owned(),
                })
            }
            super::super::RunnerRecord::Dialled { .. } => Err(ServerError::Runner {
                refusal: "runner_control_invalid".to_owned(),
                words: "A dialled runner uses its signed control connection.".to_owned(),
            }),
        }
    }
}
