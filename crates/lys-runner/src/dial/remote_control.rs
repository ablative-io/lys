//! A signed control stream carries reconnect independently of the delivery lanes.

use super::super::reconnect::{self, RetryWait};
use super::super::{Dial, EPOCH_ROUTE, Signer, failed};
use crate::RunnerError;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

pub(super) fn report(signer: &Signer, retry: &RetryWait, epoch: &str) -> Result<(), RunnerError> {
    let body = serde_json::to_vec(&retry.status()?).map_err(|error| failed(error.to_string()))?;
    signer.post(
        &format!("/runner/dial/{}/connection", signer.machine),
        epoch,
        &body,
    )?;
    Ok(())
}

pub(super) struct Remote {
    control: Option<Arc<Signer>>,
    reporter: Option<Arc<Signer>>,
    retry: Arc<RetryWait>,
    worker: Option<std::thread::JoinHandle<Result<(), RunnerError>>>,
}

impl Remote {
    pub(super) fn open(
        dial: &Dial,
        stopping: &super::Stopping,
        probe: &Signer,
    ) -> Result<(Self, String), RunnerError> {
        let (epoch, supported) = stopping.ready(
            || {
                let response = probe.channel.send("GET", EPOCH_ROUTE, &[], b"")?;
                let supported = response.header("x-lys-runner-control") == Some("v1");
                let epoch =
                    String::from_utf8(response.body).map_err(|error| failed(error.to_string()))?;
                Ok((epoch.trim().to_owned(), supported))
            },
            &AtomicBool::new(false),
        )?;
        if !supported {
            crate::error::said(
                "runner_control_unsupported: this Lys build offers no remote Reconnect control; lys runner reconnect remains available on this computer",
            );
            return Ok((
                Self {
                    control: None,
                    reporter: None,
                    retry: Arc::clone(&stopping.retry),
                    worker: None,
                },
                epoch,
            ));
        }
        let control = Arc::new(dial.signer()?);
        let reporter = Arc::new(dial.signer()?);
        let listening_epoch = epoch.clone();
        let (held, reports, retry) = (
            Arc::clone(&control),
            Arc::clone(&reporter),
            Arc::clone(&stopping.retry),
        );
        let worker = std::thread::Builder::new()
            .name("runner-remote-reconnect".to_owned())
            .spawn(move || {
                let outcome = listen(&held, &reports, &retry, listening_epoch);
                if let Err(error) = &outcome {
                    crate::error::said(&format!("runner_remote_control_ended: {error}"));
                }
                outcome
            })
            .map_err(|error| failed(format!("runner_control_start_failed: {error}")))?;
        Ok((
            Self {
                control: Some(control),
                reporter: Some(reporter),
                retry: Arc::clone(&stopping.retry),
                worker: Some(worker),
            },
            epoch,
        ))
    }

    pub(super) fn reporter(&self) -> Option<Arc<Signer>> {
        self.reporter.clone()
    }

    pub(super) fn finish(mut self) -> Result<(), RunnerError> {
        self.close()
    }

    fn close(&mut self) -> Result<(), RunnerError> {
        let Some(worker) = self.worker.take() else {
            return Ok(());
        };
        let mut errors = Vec::new();
        if let Err(error) = self.retry.stop() {
            errors.push(error.to_string());
        }
        if let Some(control) = &self.control {
            if let Err(error) = control.channel.close() {
                errors.push(error.to_string());
            }
        }
        if let Some(reporter) = &self.reporter {
            if let Err(error) = reporter.channel.close() {
                errors.push(error.to_string());
            }
        }
        match worker.join() {
            Ok(Err(error)) if error.name() != "runner_dial_stopped" => {
                errors.push(error.to_string());
            }
            Ok(_) => {}
            Err(payload) => {
                drop(payload);
                errors.push("runner_control_join_failed: the control worker panicked".to_owned());
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(RunnerError::refused(
                "runner_control_close_failed",
                errors.join("; "),
            ))
        }
    }
}

impl Drop for Remote {
    fn drop(&mut self) {
        if let Err(error) = self.close() {
            crate::error::said(&error.to_string());
        }
    }
}

enum Control {
    Reconnect(u64),
    GoingAway(u64),
}

fn said_report(reporter: &Signer, retry: &RetryWait, epoch: &str) {
    if let Err(error) = report(reporter, retry, epoch) {
        crate::error::said(&format!("runner_connection_report_failed: {error}"));
    }
}

fn epoch(signer: &Signer, reporter: &Signer, retry: &RetryWait) -> Result<String, RunnerError> {
    reconnect::ready_reporting(
        || signer.epoch(),
        &AtomicBool::new(false),
        |delay| retry.wait(delay),
        |status| retry.report(status),
    )
    .inspect(|current| said_report(reporter, retry, current))
}

fn listen(
    signer: &Signer,
    reporter: &Signer,
    retry: &RetryWait,
    mut current: String,
) -> Result<(), RunnerError> {
    let mut after = 0;
    loop {
        let answer = reconnect::ready_reporting(
            || {
                let body = serde_json::to_vec(
                    &serde_json::json!({"after":after,"connection":retry.status()?}),
                )
                .map_err(|error| failed(error.to_string()))?;
                let response = signer.post(
                    &format!("/runner/dial/{}/control", signer.machine),
                    &current,
                    &body,
                )?;
                if let Some(expected) = response.header("x-lys-going-away-until") {
                    return expected
                        .parse()
                        .map(Control::GoingAway)
                        .map_err(|error: std::num::ParseIntError| failed(error.to_string()));
                }
                let value: serde_json::Value =
                    serde_json::from_slice(&response.body).map_err(|error| {
                        failed(format!(
                            "runner_control_invalid: {:?} at {}:{}",
                            error.classify(),
                            error.line(),
                            error.column()
                        ))
                    })?;
                value["generation"]
                    .as_u64()
                    .map(Control::Reconnect)
                    .ok_or_else(|| failed("runner_control_invalid: no reconnect generation"))
            },
            &AtomicBool::new(false),
            |delay| retry.wait(delay),
            |status| {
                retry.report(status)?;
                said_report(reporter, retry, &current);
                Ok(())
            },
        );
        match answer {
            Ok(Control::Reconnect(generation)) => {
                after = generation;
                retry.reconnect()?;
            }
            Ok(Control::GoingAway(expected)) => {
                current = retry.resume_at(expected, || epoch(signer, reporter, retry))?;
            }
            Err(RunnerError::DialStale { .. }) => {
                current = epoch(signer, reporter, retry)?;
                after = 0;
            }
            Err(error) => return Err(error),
        }
    }
}
