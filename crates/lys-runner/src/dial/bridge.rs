//! Two independently waiting lanes keep a computer reachable during a handoff.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, mpsc};

use super::{Dial, Signer, control_request, dispatch, reconnect};
use crate::error::RunnerError;
use crate::protocol::{Answer, reply_line};

#[path = "reply_outbox.rs"]
mod reply_outbox;
use reply_outbox::{Outbox, Reply};

#[path = "control.rs"]
mod control;

#[path = "next_frame.rs"]
mod next_frame;

#[path = "remote_control.rs"]
mod remote_control;

impl Dial {
    /// Interrupt this runner's current reconnect backoff through its owner-only socket.
    ///
    /// # Errors
    /// Returns unavailable local control or a named reconnect refusal.
    pub fn reconnect_socket(socket: &std::path::Path) -> Result<serde_json::Value, RunnerError> {
        control::ask(socket, b'R')
    }

    /// Read the runner's current connection status, including its next attempted dial.
    ///
    /// # Errors
    /// Returns unavailable local control or an unreadable status response.
    pub fn connection_status(socket: &std::path::Path) -> Result<serde_json::Value, RunnerError> {
        control::ask(socket, b'S')
    }
}

impl Stopping {
    fn ready<T>(
        &self,
        take: impl FnMut() -> Result<T, RunnerError>,
        disconnected: &AtomicBool,
    ) -> Result<T, RunnerError> {
        reconnect::ready_reporting(
            take,
            disconnected,
            |delay| self.retry.wait(delay),
            |status| {
                self.retry.report(status)?;
                if let Some(reporter) = &self.reporter {
                    if let Err(error) = reporter
                        .epoch()
                        .and_then(|epoch| remote_control::report(reporter, &self.retry, &epoch))
                    {
                        crate::error::said(&format!("runner_connection_report_failed: {error}"));
                    }
                }
                Ok(())
            },
        )
    }
}

#[derive(Default)]
struct Stopping {
    state: AtomicBool,
    retry: Arc<reconnect::RetryWait>,
    reporter: Option<Arc<Signer>>,
}

struct Lane {
    outbox: Arc<Outbox>,
    signer: Arc<Signer>,
    initial_epoch: Option<String>,
    local: Arc<Mutex<BTreeMap<String, crate::client::Closer>>>,
}

fn stopped() -> RunnerError {
    RunnerError::refused("runner_dial_stopped", "the dial bridge has ended")
}

fn stop(lanes: &[Lane], stopping: &Stopping) {
    stopping.state.store(true, Ordering::Release);
    if let Err(error) = stopping.retry.stop() {
        crate::error::said(&error.to_string());
    }
    if let Some(reporter) = &stopping.reporter {
        if let Err(error) = reporter.channel.close() {
            crate::error::said(&error.to_string());
        }
    }
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
    let outbox = Arc::new(Outbox::open(&dial.socket)?);
    let mut lanes = lanes(dial, &outbox)?;
    let mut stopping = Stopping::default();
    let control = control::Control::open(&dial.socket, Arc::clone(&stopping.retry))?;
    let (remote, initial_epoch) = remote_control::Remote::open(dial, &stopping, &lanes[0].signer)?;
    lanes[0].initial_epoch = Some(initial_epoch);
    stopping.reporter = remote.reporter();
    let disconnected = Arc::new(AtomicBool::new(false));
    let finished_reply = Arc::new(AtomicBool::new(false));
    let dispatch = Mutex::new(dispatch::Dispatch::new());
    let answered = std::thread::scope(|scope| {
        let (send, receive) = mpsc::channel();
        let mut workers = Vec::new();
        let mut spawn_failure = None;
        for lane in &lanes {
            let send = send.clone();
            let stopping = &stopping;
            let disconnected = &disconnected;
            let dispatch = &dispatch;
            let finished_reply = Arc::clone(&finished_reply);
            match std::thread::Builder::new()
                .name("runner-dial-poll".to_owned())
                .spawn_scoped(scope, move || {
                    let answer = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        poll(
                            dial,
                            lane,
                            stopping,
                            disconnected,
                            dispatch,
                            &send,
                            &finished_reply,
                        )
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
    });
    let controls = control.finish().and_then(|()| remote.finish());
    match (answered, controls) {
        (Err(error), Err(control)) => Err(RunnerError::refused(
            "runner_dial_stop_failed",
            format!("{error}; {control}"),
        )),
        (Err(error), Ok(())) | (Ok(()), Err(error)) => Err(error),
        (Ok(()), Ok(())) => Ok(()),
    }
}

fn lanes(dial: &Dial, outbox: &Arc<Outbox>) -> Result<[Lane; 2], RunnerError> {
    Ok([
        Lane {
            outbox: Arc::clone(outbox),
            signer: Arc::new(dial.signer()?),
            initial_epoch: None,
            local: Arc::default(),
        },
        Lane {
            outbox: Arc::clone(outbox),
            signer: Arc::new(dial.signer()?),
            initial_epoch: None,
            local: Arc::default(),
        },
    ])
}

fn poll(
    dial: &Dial,
    lane: &Lane,
    stopping: &Stopping,
    disconnected: &Arc<AtomicBool>,
    dispatch: &Mutex<dispatch::Dispatch>,
    failures: &mpsc::Sender<Result<(), RunnerError>>,
    finished_reply: &Arc<AtomicBool>,
) -> Result<(), RunnerError> {
    let signer = &lane.signer;
    let epoch = || {
        if stopping.state.load(Ordering::Acquire) {
            Err(stopped())
        } else {
            signer.epoch()
        }
    };
    let mut current = match &lane.initial_epoch {
        Some(accepted) => accepted.clone(),
        None => stopping.ready(epoch, disconnected)?,
    };
    flush_replies(signer, &current, &lane.outbox, finished_reply, disconnected)?;
    loop {
        if stopping.state.load(Ordering::Acquire) {
            return Err(stopped());
        }
        let mut connection = crate::client::connect(&dial.socket)?;
        let registration = Held::new(&lane.local, connection.closer()?)?;
        let greeting = connection.greeting_line()?;
        let (ticket, request_line) = next_request(
            lane,
            &mut current,
            &greeting,
            stopping,
            disconnected,
            finished_reply,
        )?;
        if disconnected.swap(false, Ordering::AcqRel) {
            crate::error::said("runner_dial_reconnected: the server accepted this runner again");
        }
        let control = control_request(&request_line);
        let posted = Arc::clone(signer);
        let posting_epoch = current.clone();
        let posting_ticket = ticket.clone();
        let failures = failures.clone();
        let posting_finished = Arc::clone(finished_reply);
        let posting_disconnected = Arc::clone(disconnected);
        let posting_outbox = Arc::clone(&lane.outbox);
        let mut dispatch = dispatch.lock().map_err(|error| {
            RunnerError::refused("runner_dial_worker_failed", error.to_string())
        })?;
        if let Err(error) = dispatch.submit(control, move || {
            let reply = connection
                .exchange(&request_line)
                .unwrap_or_else(|error| reply_line(Answer::refusal(&error)));
            if let Err(error) = record_reply(
                &posted,
                &posting_epoch,
                &posting_ticket,
                &reply,
                &posting_finished,
                &posting_disconnected,
                &posting_outbox,
            ) && failures.send(Err(error)).is_err()
            {
                crate::error::said(
                    "runner_dial_worker_failed: the bridge left before its reply ended",
                );
            }
            drop(registration);
        }) {
            record_reply(
                signer,
                &current,
                &ticket,
                &reply_line(Answer::refusal(&error)),
                finished_reply,
                disconnected,
                &lane.outbox,
            )?;
        }
    }
}

fn next_request(
    lane: &Lane,
    current: &mut String,
    greeting: &str,
    stopping: &Stopping,
    disconnected: &AtomicBool,
    finished_reply: &AtomicBool,
) -> Result<(String, String), RunnerError> {
    let signer = &lane.signer;
    let epoch = || {
        if stopping.state.load(Ordering::Acquire) {
            Err(stopped())
        } else {
            signer.epoch()
        }
    };
    loop {
        if stopping.state.load(Ordering::Acquire) {
            return Err(stopped());
        }
        let mut refresh = false;
        let answer = stopping.ready(
            || {
                if stopping.state.load(Ordering::Acquire) {
                    return Err(stopped());
                }
                if refresh {
                    *current = stopping.ready(epoch, disconnected)?;
                    flush_replies(signer, current, &lane.outbox, finished_reply, disconnected)?;
                }
                let answer = signer.next_frame(current, greeting);
                refresh = matches!(&answer, Err(RunnerError::Unreachable { .. }));
                answer
            },
            disconnected,
        );
        if stopping.state.load(Ordering::Acquire) {
            return Err(stopped());
        }
        match answer {
            Ok(next_frame::Frame::Request(ticket, request)) => return Ok((ticket, request)),
            Ok(next_frame::Frame::GoingAway(expected)) => {
                crate::error::said(&format!(
                    "runner_going_away: Lys expects to return at {expected}"
                ));
                *current = stopping
                    .retry
                    .resume_at(expected, || stopping.ready(epoch, disconnected))?;
                flush_replies(signer, current, &lane.outbox, finished_reply, disconnected)?;
            }
            Err(RunnerError::DialStale { .. }) => {
                *current = stopping.ready(epoch, disconnected)?;
            }
            Err(error) => return Err(error),
        }
    }
}

fn record_reply(
    signer: &Signer,
    epoch: &str,
    ticket: &str,
    reply: &str,
    finished: &AtomicBool,
    disconnected: &AtomicBool,
    outbox: &Outbox,
) -> Result<(), RunnerError> {
    outbox.save(&Reply {
        epoch: epoch.to_owned(),
        ticket: ticket.to_owned(),
        body: reply.to_owned(),
    })?;
    if transmit_reply(signer, epoch, ticket, reply, finished, disconnected)? {
        outbox.acknowledge(ticket)?;
    }
    Ok(())
}

fn flush_replies(
    signer: &Signer,
    epoch: &str,
    outbox: &Outbox,
    finished: &AtomicBool,
    disconnected: &AtomicBool,
) -> Result<(), RunnerError> {
    for reply in outbox.pending()? {
        if transmit_reply(
            signer,
            epoch,
            &reply.ticket,
            &reply.body,
            finished,
            disconnected,
        )? {
            outbox.acknowledge(&reply.ticket)?;
        } else {
            break;
        }
    }
    Ok(())
}

#[cfg(test)]
fn post_reply(
    signer: &Signer,
    epoch: &str,
    ticket: &str,
    reply: &str,
    finished: &AtomicBool,
    disconnected: &AtomicBool,
) -> Result<(), RunnerError> {
    transmit_reply(signer, epoch, ticket, reply, finished, disconnected).map(drop)
}

fn transmit_reply(
    signer: &Signer,
    epoch: &str,
    ticket: &str,
    reply: &str,
    finished: &AtomicBool,
    disconnected: &AtomicBool,
) -> Result<bool, RunnerError> {
    match signer.reply(epoch, ticket, reply) {
        Err(RunnerError::Unreachable { .. }) => {
            // A completed act is never replayed; only its retained reply can be resent.
            reconnect::disconnected(disconnected);
            Ok(false)
        }
        Err(error @ RunnerError::Refused { .. })
            if matches!(
                error.name().as_str(),
                "runner_reply_expired" | "runner_reply_duplicate" | "runner_reply_unknown"
            ) =>
        {
            // Expired and unknown tickets keep their local receipt until readback is acknowledged.
            if !finished.swap(true, Ordering::AcqRel) {
                crate::error::said(&error.to_string());
            }
            Ok(error.name() == "runner_reply_duplicate")
        }
        Ok(()) => Ok(true),
        Err(error) => Err(error),
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

#[cfg(test)]
mod lifecycle_tests;
