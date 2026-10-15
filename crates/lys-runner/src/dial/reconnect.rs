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

pub(super) fn ready<T>(
    mut take: impl FnMut() -> Result<T, RunnerError>,
    state: &AtomicBool,
    mut wait: impl FnMut(std::time::Duration) -> Result<(), RunnerError>,
) -> Result<T, RunnerError> {
    let mut failures = 0_u32;
    loop {
        match take() {
            Err(error @ RunnerError::Unreachable { .. }) => {
                disconnected(state);
                let delay = delay(failures, rand::random());
                failures = failures.saturating_add(1);
                crate::error::said(&format!(
                    "runner_dial_reconnect_failed: attempt {failures}: {error}; retrying in {} ms",
                    delay.as_millis(),
                ));
                wait(delay)?;
            }
            answer => return answer,
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
    stopped: std::sync::Mutex<bool>,
    changed: std::sync::Condvar,
}

fn unavailable() -> RunnerError {
    RunnerError::refused(
        "runner_dial_reconnect_wait_failed",
        "the reconnect stop lock is poisoned",
    )
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
        let held = self.stopped.lock().map_err(|_| unavailable())?;
        entered();
        let (held, _) = self
            .changed
            .wait_timeout_while(held, delay, |stopped| !*stopped)
            .map_err(|_| unavailable())?;
        if *held {
            return Err(RunnerError::refused(
                "runner_dial_stopped",
                "the dial bridge has ended",
            ));
        }
        Ok(())
    }

    pub(super) fn stop(&self) -> Result<(), RunnerError> {
        *self.stopped.lock().map_err(|_| unavailable())? = true;
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
#[path = "lysfix_srv_reconnect_tests.rs"]
mod lysfix_srv_reconnect_tests;
