#![cfg(test)]
//! Held requests cannot consume the bridge's reserved control worker.

use std::error::Error;
use std::sync::{Arc, Condvar, Mutex, mpsc};

use super::Dispatch;

#[test]
fn ordinary_dial_dispatch_is_bounded_and_control_keeps_a_slot() -> Result<(), Box<dyn Error>> {
    let mut dispatch = Dispatch::new();
    let released = Arc::new((Mutex::new(false), Condvar::new()));
    let (started, entered) = mpsc::channel();
    let (finished, ended) = mpsc::channel();
    for _ in 0..31 {
        let release = Arc::clone(&released);
        let started = started.clone();
        let finished = finished.clone();
        dispatch.submit(false, move || {
            let result = (|| -> Result<(), String> {
                started.send(()).map_err(|error| error.to_string())?;
                let (lock, changed) = &*release;
                let mut ready = lock.lock().map_err(|error| error.to_string())?;
                while !*ready {
                    ready = changed.wait(ready).map_err(|error| error.to_string())?;
                }
                Ok(())
            })();
            if let Err(error) = finished.send(result) {
                crate::error::said(&format!("dial_test_completion_failed: {error}"));
            }
        })?;
    }
    for _ in 0..31 {
        entered.recv()?;
    }
    let overflow = dispatch.submit(false, || {});
    let (control_done, controlled) = mpsc::channel();
    let control = dispatch.submit(true, move || {
        if let Err(error) = control_done.send(()) {
            crate::error::said(&format!("dial_test_control_failed: {error}"));
        }
    });
    *released.0.lock().map_err(|error| error.to_string())? = true;
    released.1.notify_all();
    for _ in 0..31 {
        ended.recv()?.map_err(std::io::Error::other)?;
    }
    control?;
    controlled.recv()?;
    assert_eq!(
        overflow.err().map(|error| error.name()).as_deref(),
        Some("runner_dial_full")
    );
    Ok(())
}
