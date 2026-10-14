//! Two independently waiting lanes keep a computer reachable during a handoff.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, mpsc};

use super::{Dial, Signer, control_request, dispatch, reconnect};
use crate::error::RunnerError;
use crate::protocol::{Answer, reply_line};

struct Lane {
    signer: Arc<Signer>,
    local: Arc<Mutex<BTreeMap<String, crate::client::Closer>>>,
}

fn stopped() -> RunnerError {
    RunnerError::refused("runner_dial_stopped", "the dial bridge has ended")
}

fn stop(lanes: &[Lane], stopping: &AtomicBool) {
    stopping.store(true, Ordering::Release);
    for lane in lanes {
        if let Err(error) = lane.signer.channel.close() {
            crate::error::said(&error.to_string());
        }
        match lane.local.lock() {
            Ok(mut local) => {
                for (_, closer) in std::mem::take(&mut *local) {
                    closer.close();
                }
            }
            Err(error) => crate::error::said(&format!("runner_dial_stop_failed: {error}")),
        }
    }
}

pub(super) fn run(dial: &Dial) -> Result<(), RunnerError> {
    let lanes = [
        Lane {
            signer: Arc::new(dial.signer()?),
            local: Arc::default(),
        },
        Lane {
            signer: Arc::new(dial.signer()?),
            local: Arc::default(),
        },
    ];
    let stopping = AtomicBool::new(false);
    let disconnected = AtomicBool::new(false);
    let dispatch = Mutex::new(dispatch::Dispatch::new());
    std::thread::scope(|scope| {
        let (send, receive) = mpsc::channel();
        let mut workers = Vec::new();
        let mut spawn_failure = None;
        for lane in &lanes {
            let send = send.clone();
            let stopping = &stopping;
            let disconnected = &disconnected;
            let dispatch = &dispatch;
            match std::thread::Builder::new()
                .name("runner-dial-poll".to_owned())
                .spawn_scoped(scope, move || {
                    let answer = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        poll(dial, lane, stopping, disconnected, dispatch)
                    }))
                    .unwrap_or_else(|_| {
                        Err(RunnerError::refused(
                            "runner_dial_worker_failed",
                            "the poll worker panicked",
                        ))
                    });
                    if send.send(answer).is_err() {
                        crate::error::said(
                            "runner_dial_worker_failed: the bridge left before its poll ended",
                        );
                    }
                }) {
                Ok(worker) => workers.push(worker),
                Err(error) => {
                    spawn_failure = Some(RunnerError::refused(
                        "runner_dial_worker_failed",
                        error.to_string(),
                    ));
                    break;
                }
            }
        }
        drop(send);
        let answer = match spawn_failure {
            Some(error) => Err(error),
            None => receive.recv().map_err(|error| {
                RunnerError::refused("runner_dial_worker_failed", error.to_string())
            })?,
        };
        stop(&lanes, &stopping);
        for worker in workers {
            if worker.join().is_err() {
                return Err(RunnerError::refused(
                    "runner_dial_worker_failed",
                    "the poll worker could not be joined",
                ));
            }
        }
        answer
    })
}

fn poll(
    dial: &Dial,
    lane: &Lane,
    stopping: &AtomicBool,
    disconnected: &AtomicBool,
    dispatch: &Mutex<dispatch::Dispatch>,
) -> Result<(), RunnerError> {
    let signer = &lane.signer;
    let epoch = || {
        if stopping.load(Ordering::Acquire) {
            Err(stopped())
        } else {
            signer.epoch()
        }
    };
    let mut current = reconnect::ready(epoch, disconnected, reconnect::pause)?;
    loop {
        if stopping.load(Ordering::Acquire) {
            return Err(stopped());
        }
        let mut connection = crate::client::connect(&dial.socket)?;
        let registration = Held::new(&lane.local, connection.closer()?)?;
        let greeting = connection.greeting_line()?;
        let (ticket, line) = loop {
            if stopping.load(Ordering::Acquire) {
                return Err(stopped());
            }
            let answer = signer.next(&current, &greeting);
            if stopping.load(Ordering::Acquire) {
                return Err(stopped());
            }
            match answer {
                Err(RunnerError::DialStale { .. }) => {
                    current = reconnect::ready(epoch, disconnected, reconnect::pause)?
                }
                Err(RunnerError::Unreachable { .. }) => {
                    reconnect::disconnected(disconnected);
                    reconnect::pause();
                    current = reconnect::ready(epoch, disconnected, reconnect::pause)?;
                }
                answer => break answer?,
            }
        };
        if disconnected.swap(false, Ordering::AcqRel) {
            crate::error::said("runner_dial_reconnected: the server accepted this runner again");
        }
        let control = control_request(&line);
        let posted = Arc::clone(signer);
        let posting_epoch = current.clone();
        let posting_ticket = ticket.clone();
        let mut dispatch = dispatch.lock().map_err(|error| {
            RunnerError::refused("runner_dial_worker_failed", error.to_string())
        })?;
        if let Err(error) = dispatch.submit(control, move || {
            let reply = connection
                .exchange(&line)
                .unwrap_or_else(|error| reply_line(Answer::refusal(&error)));
            if let Err(error) = posted.reply(&posting_epoch, &posting_ticket, &reply) {
                crate::error::said(&format!("a reply was not delivered: {error}"));
            }
            drop(registration);
        }) {
            signer.reply(&current, &ticket, &reply_line(Answer::refusal(&error)))?;
        }
    }
}

struct Held {
    key: String,
    local: Arc<Mutex<BTreeMap<String, crate::client::Closer>>>,
}

impl Held {
    fn new(
        local: &Arc<Mutex<BTreeMap<String, crate::client::Closer>>>,
        closer: crate::client::Closer,
    ) -> Result<Self, RunnerError> {
        let key = crate::protocol::nonce();
        local
            .lock()
            .map_err(|error| RunnerError::refused("runner_dial_stop_failed", error.to_string()))?
            .insert(key.clone(), closer);
        Ok(Self {
            key,
            local: Arc::clone(local),
        })
    }
}

impl Drop for Held {
    fn drop(&mut self) {
        match self.local.lock() {
            Ok(mut local) => {
                local.remove(&self.key);
            }
            Err(error) => crate::error::said(&format!("runner_dial_stop_failed: {error}")),
        }
    }
}

#[cfg(test)]
mod tests;
