//! A broken server connection is replaced without replaying the failed request.

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
    mut wait: impl FnMut(),
) -> Result<T, RunnerError> {
    loop {
        match take() {
            Err(RunnerError::Unreachable { .. }) => {
                disconnected(state);
                wait();
            }
            answer => return answer,
        }
    }
}

pub(super) fn pause() {
    std::thread::sleep(std::time::Duration::from_millis(250));
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
            || waits += 1,
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
            || waits += 1,
        );
        assert!(refused.is_err());
        assert_eq!(waits, 1);
        Ok(())
    }
}
