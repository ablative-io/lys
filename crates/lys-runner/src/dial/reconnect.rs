//! A broken server feed reconnects without replaying a dispatched act.

use crate::error::RunnerError;
use std::sync::atomic::{AtomicBool, Ordering};

pub(super) fn disconnected(state: &AtomicBool) {
    if !state.swap(true, Ordering::AcqRel) {
        crate::error::said(
            "runner_dial_disconnected: the server connection was lost; reconnecting",
        );
    }
}

#[derive(Clone, serde::Serialize)]
pub(super) struct RetryStatus {
    pub(super) refused_at: u64,
    pub(super) next_dial: u64,
    pub(super) attempt: u32,
    pub(super) reason: String,
}

impl RetryStatus {
    pub(super) fn words(&self) -> String {
        format!(
            "Lys refused at {}, next dial at {}, attempt {}: {}",
            self.refused_at, self.next_dial, self.attempt, self.reason
        )
    }
}

fn now() -> Result<u64, RunnerError> {
    let elapsed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|error| RunnerError::refused("runner_dial_clock_failed", error.to_string()))?;
    u64::try_from(elapsed.as_millis())
        .map_err(|error| RunnerError::refused("runner_dial_clock_failed", error.to_string()))
}

#[cfg(test)]
pub(super) fn ready<T>(
    take: impl FnMut() -> Result<T, RunnerError>,
    state: &AtomicBool,
    wait: impl FnMut(std::time::Duration) -> Result<(), RunnerError>,
) -> Result<T, RunnerError> {
    ready_reporting(take, state, wait, |status| {
        if let Some(status) = status {
            crate::error::said(&status.words());
        }
        Ok(())
    })
}

pub(super) fn ready_reporting<T>(
    mut take: impl FnMut() -> Result<T, RunnerError>,
    state: &AtomicBool,
    mut wait: impl FnMut(std::time::Duration) -> Result<(), RunnerError>,
    mut report: impl FnMut(Option<&RetryStatus>) -> Result<(), RunnerError>,
) -> Result<T, RunnerError> {
    let mut failures = 0_u32;
    loop {
        match take() {
            Err(error @ RunnerError::Unreachable { .. }) => {
                disconnected(state);
                let delay = delay(failures, rand::random());
                failures = failures.saturating_add(1);
                let refused_at = now()?;
                let milliseconds = u64::try_from(delay.as_millis()).map_err(|error| {
                    RunnerError::refused("runner_dial_clock_failed", error.to_string())
                })?;
                report(Some(&RetryStatus {
                    refused_at,
                    next_dial: refused_at.saturating_add(milliseconds),
                    attempt: failures,
                    reason: error.to_string(),
                }))?;
                wait(delay)?;
            }
            Ok(answer) => {
                report(None)?;
                return Ok(answer);
            }
            Err(error) => return Err(error),
        }
    }
}

fn delay(attempt: u32, jitter: u64) -> std::time::Duration {
    let ceiling = (250_u64 * (1_u64 << attempt.min(7))).min(30_000);
    let minimum = ceiling / 2;
    std::time::Duration::from_millis(minimum + jitter % (ceiling - minimum + 1))
}

/// A stop signal interrupts a failed connection's backoff immediately.
#[derive(Default)]
pub(super) struct RetryWait {
    state: std::sync::Mutex<WaitState>,
    changed: std::sync::Condvar,
}

#[derive(Default)]
struct WaitState {
    stopped: bool,
    connected: bool,
    generation: u64,
    status: Option<RetryStatus>,
}

impl RetryWait {
    pub(super) fn wait(&self, delay: std::time::Duration) -> Result<(), RunnerError> {
        self.wait_with(delay, || {})
    }

    fn wait_with(
        &self,
        delay: std::time::Duration,
        entered: impl FnOnce(),
    ) -> Result<(), RunnerError> {
        self.wait_observed(delay, entered, |_, _| {})
    }

    fn wait_observed(
        &self,
        delay: std::time::Duration,
        entered: impl FnOnce(),
        mut checked: impl FnMut(u64, bool),
    ) -> Result<(), RunnerError> {
        let held = self.state.lock().map_err(|error| {
            RunnerError::refused("runner_dial_reconnect_wait_failed", error.to_string())
        })?;
        let generation = held.generation;
        entered();
        let (held, _) = self
            .changed
            .wait_timeout_while(held, delay, |state| {
                let waiting = !state.stopped && state.generation == generation;
                checked(state.generation, waiting);
                waiting
            })
            .map_err(|error| {
                RunnerError::refused("runner_dial_reconnect_wait_failed", error.to_string())
            })?;
        if held.stopped {
            return Err(RunnerError::refused(
                "runner_dial_stopped",
                "the dial bridge has ended",
            ));
        }
        Ok(())
    }

    pub(super) fn reconnect(&self) -> Result<u64, RunnerError> {
        let mut held = self.state.lock().map_err(|error| {
            RunnerError::refused("runner_dial_reconnect_wait_failed", error.to_string())
        })?;
        held.generation = held.generation.wrapping_add(1);
        if let Some(status) = &mut held.status {
            status.next_dial = now()?;
        }
        let generation = held.generation;
        self.changed.notify_all();
        Ok(generation)
    }

    pub(super) fn report(&self, status: Option<&RetryStatus>) -> Result<(), RunnerError> {
        if let Some(status) = status {
            crate::error::said(&status.words());
        }
        let mut held = self.state.lock().map_err(|error| {
            RunnerError::refused("runner_dial_reconnect_wait_failed", error.to_string())
        })?;
        held.status = status.cloned();
        held.connected = status.is_none();
        Ok(())
    }

    pub(super) fn status(&self) -> Result<serde_json::Value, RunnerError> {
        let held = self.state.lock().map_err(|error| {
            RunnerError::refused("runner_dial_reconnect_wait_failed", error.to_string())
        })?;
        let state = if held.stopped {
            "stopped"
        } else if held.status.is_some() {
            "refused"
        } else if held.connected {
            "connected"
        } else {
            "connecting"
        };
        Ok(
            serde_json::json!({"state":state, "words":held.status.as_ref().map(RetryStatus::words), "retry":held.status}),
        )
    }

    pub(super) fn stop(&self) -> Result<(), RunnerError> {
        self.state
            .lock()
            .map_err(|error| {
                RunnerError::refused("runner_dial_reconnect_wait_failed", error.to_string())
            })?
            .stopped = true;
        self.changed.notify_all();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_lost_server_connection_retries_but_an_admission_refusal_does_not()
    -> Result<(), crate::RunnerError> {
        let mut reads = 0;
        let mut waits = 0;
        let disconnected = super::AtomicBool::new(false);
        let value = super::ready(
            || {
                reads += 1;
                if reads == 1 {
                    Err(crate::RunnerError::Unreachable {
                        reason: "closed".to_owned(),
                    })
                } else {
                    Ok(42)
                }
            },
            &disconnected,
            |_| {
                waits += 1;
                Ok(())
            },
        )?;
        assert_eq!(value, 42);
        assert_eq!(reads, 2);
        assert_eq!(waits, 1);
        assert!(disconnected.load(super::Ordering::Acquire));
        let refused = super::ready(
            || {
                Err::<(), _>(crate::RunnerError::Dial {
                    reason: "admission refused".to_owned(),
                })
            },
            &disconnected,
            |_| {
                waits += 1;
                Ok(())
            },
        );
        assert!(refused.is_err());
        assert_eq!(waits, 1);
        Ok(())
    }
}

#[cfg(test)]
#[path = "reconnect_tests.rs"]
mod reconnect_tests;

#[cfg(test)]
#[path = "reconnect_control_tests.rs"]
mod control_tests;
