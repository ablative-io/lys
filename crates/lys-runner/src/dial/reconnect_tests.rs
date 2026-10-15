#![cfg(test)]
//! Failed connection attempts back off; a connected feed has no retry clock.

use std::sync::atomic::AtomicBool;
use std::time::Duration;

#[test]
fn reconnect_failed_attempts_use_capped_exponential_backoff() -> Result<(), crate::RunnerError> {
    let mut attempts = 0;
    let mut delays = Vec::new();
    let disconnected = AtomicBool::new(false);
    super::ready(
        || {
            attempts += 1;
            if attempts <= 20 {
                Err(crate::RunnerError::Unreachable {
                    reason: "connection closed".to_owned(),
                })
            } else {
                Ok(())
            }
        },
        &disconnected,
        |delay| {
            delays.push(delay);
            Ok(())
        },
    )?;
    assert_eq!(delays.len(), 20);
    assert!(
        delays[2] >= Duration::from_millis(500),
        "third reconnect delay stayed at 250 ms"
    );
    assert!(delays.iter().all(|delay| *delay <= Duration::from_secs(30)));
    assert!(
        delays[19] >= Duration::from_secs(15),
        "backoff did not reach its cap"
    );
    Ok(())
}

#[test]
fn connected_feed_has_no_backoff_wait() -> Result<(), crate::RunnerError> {
    let disconnected = AtomicBool::new(false);
    let mut waits = 0;
    assert_eq!(
        super::ready(
            || Ok(42),
            &disconnected,
            |_| {
                waits += 1;
                Ok(())
            }
        )?,
        42
    );
    assert_eq!(waits, 0);
    Ok(())
}

#[test]
fn reconnect_jitter_varies_within_the_cap() {
    assert_eq!(super::delay(0, 0), Duration::from_millis(125));
    assert_eq!(super::delay(0, 125), Duration::from_millis(250));
    assert_eq!(super::delay(100, 0), Duration::from_secs(15));
    assert_eq!(super::delay(100, 15_000), Duration::from_secs(30));
    assert_ne!(super::delay(3, 0), super::delay(3, 37));
}

#[test]
fn reconnect_stop_signal_cancels_a_pending_backoff() -> Result<(), Box<dyn std::error::Error>> {
    let wait = std::sync::Arc::new(super::RetryWait::default());
    let waiting = std::sync::Arc::clone(&wait);
    let (entered, ready) = std::sync::mpsc::channel();
    let worker = std::thread::spawn(move || {
        waiting.wait_with(Duration::from_secs(30), || {
            assert!(
                entered.send(()).is_ok(),
                "the stop fixture left before registration"
            );
        })
    });
    ready.recv()?;
    wait.stop()?;
    let error = worker
        .join()
        .map_err(|_| "the backoff waiter panicked")?
        .err()
        .ok_or("the stop signal did not end the backoff")?;
    assert_eq!(error.name(), "runner_dial_stopped");
    Ok(())
}
