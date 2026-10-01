//! The bridge's workers are reused; held requests cannot consume its control slot.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};

use crate::error::RunnerError;

#[cfg(test)]
#[path = "../../tests/dial_dispatch/cases.rs"]
mod bound_tests;

type Work = Box<dyn FnOnce() + Send>;
const ORDINARY: usize = 31;

struct Worker {
    sender: mpsc::Sender<Work>,
    busy: Arc<AtomicBool>,
}

pub(super) struct Dispatch {
    ordinary: Vec<Worker>,
    control: Option<Worker>,
}

impl Dispatch {
    pub(super) fn new() -> Self {
        Self {
            ordinary: Vec::new(),
            control: None,
        }
    }

    pub(super) fn submit(
        &mut self,
        control: bool,
        work: impl FnOnce() + Send + 'static,
    ) -> Result<(), RunnerError> {
        let worker = if let Some(worker) = self.ordinary.iter().find(|worker| {
            worker
                .busy
                .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
        }) {
            worker
        } else if self.ordinary.len() < ORDINARY {
            self.ordinary.push(Worker::new()?);
            self.ordinary.last().ok_or_else(|| {
                RunnerError::refused("runner_dial_worker_failed", "the new worker is missing")
            })?
        } else if control {
            if self.control.is_none() {
                self.control = Some(Worker::new()?);
            } else if self.control.as_ref().is_some_and(|worker| {
                worker
                    .busy
                    .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                    .is_err()
            }) {
                return Err(full());
            }
            self.control.as_ref().ok_or_else(|| {
                RunnerError::refused("runner_dial_worker_failed", "the control worker is missing")
            })?
        } else {
            return Err(full());
        };
        worker.sender.send(Box::new(work)).map_err(|error| {
            worker.busy.store(false, Ordering::Release);
            RunnerError::refused("runner_dial_worker_failed", error.to_string())
        })
    }
}

fn full() -> RunnerError {
    RunnerError::refused("runner_dial_full", "all request workers are occupied")
}

impl Worker {
    fn new() -> Result<Self, RunnerError> {
        let (sender, receiver) = mpsc::channel::<Work>();
        let busy = Arc::new(AtomicBool::new(true));
        let occupied = Arc::clone(&busy);
        let worker = std::thread::Builder::new()
            .name("runner-dial-request".to_owned())
            .spawn(move || {
                for work in receiver {
                    if std::panic::catch_unwind(std::panic::AssertUnwindSafe(work)).is_err() {
                        crate::error::said("runner_dial_worker_failed: request handling panicked");
                    }
                    occupied.store(false, Ordering::Release);
                }
            })
            .map_err(|error| {
                RunnerError::refused("runner_dial_worker_failed", error.to_string())
            })?;
        drop(worker);
        Ok(Self { sender, busy })
    }
}
