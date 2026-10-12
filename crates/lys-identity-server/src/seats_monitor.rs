//! Seat liveness belongs to the runner. Retired monitor settings are refused.
//! Terminal stop waits follow the runner's output and process exit signals.

use std::sync::Arc;
use std::time::Duration;

#[cfg(test)]
use reqwest::header::HeaderName;
#[cfg(test)]
use serde_json::Value;

use crate::error::ServerError;
#[cfg(test)]
use crate::error_seat::SeatError;
use crate::routes::AppState;
use crate::runner_acts::Digested;
use crate::runner_sessions::{Driven, kind, record_end};
use crate::seats_acts::act;
use lys_runner::operations::{Operation, OperationRequest, OperationState};
use lys_runner::{Act, Answer};

/// The bounded grace period after accepting the terminal's exit operation.
pub(crate) const STOP_WAIT: Duration = Duration::from_secs(30);

#[cfg(test)]
const ASKED_FOR: Duration = Duration::from_secs(5);

/// Refuse retired setting names without reading or reporting their values.
pub(crate) fn check(name: &str) -> Result<String, ServerError> {
    for key in [
        "LYS_MONITOR_URL",
        "LYS_MONITOR_SECRET",
        "LYS_MONITOR_SECRET_HEADER",
    ] {
        if std::env::var_os(key).is_some() {
            return Err(ServerError::ConfigInvalid {
                reason: format!(
                    "{key} is retired: delete the line setting {key}; seat {name}'s liveness comes from the runner"
                ),
            });
        }
    }
    Ok(format!("seat {name}'s liveness is read from its runner"))
}

/// Deliver an exit operation and retain the grace budget and its result.
pub(crate) async fn stop_pty(
    state: &Arc<AppState>,
    driven: &Driven,
    caller: &str,
    operation: &str,
) -> Result<(bool, Vec<String>), ServerError> {
    let exit = Operation {
        operation: operation.to_owned(),
        session: driven.session.clone(),
        request: OperationRequest::Notice {
            text: "/exit".to_owned(),
        },
    };
    let (answer, _) = act(
        state,
        (driven, caller, "seat_stop"),
        Some(Digested::of("/exit")),
        Act::Operate {
            operation: exit.clone(),
        },
    )
    .await?;
    let outcome = crate::seats_send::outcome(answer, &exit)?;
    let mut notes = vec![format!(
        "PTY exit operation {operation}: {:?}; stop_wait={}s",
        outcome.state,
        STOP_WAIT.as_secs()
    )];
    if outcome.state == OperationState::Refused {
        return Err(ServerError::Runner {
            refusal: "operation_refused".to_owned(),
            words: outcome.words,
        });
    }
    let ended = wait_for_exit(state, driven).await?;
    notes.push(if ended {
        "the runner observed the process exit after /exit".to_owned()
    } else {
        format!(
            "stop_wait={}s expired; ending the process with Act::End",
            STOP_WAIT.as_secs()
        )
    });
    Ok((ended, notes))
}

/// Wait for the runner's exit evidence, bounded by the terminal stop budget.
async fn wait_for_exit(state: &Arc<AppState>, driven: &Driven) -> Result<bool, ServerError> {
    match tokio::time::timeout(STOP_WAIT, exited(state, driven)).await {
        Ok(answer) => answer,
        Err(_) => Ok(false),
    }
}

async fn exited(state: &Arc<AppState>, driven: &Driven) -> Result<bool, ServerError> {
    let mut cursor = None;
    loop {
        let answer = crate::runner_client::ask(
            state,
            &driven.machine,
            driven.runner.clone(),
            Act::Read {
                session: driven.session.clone(),
                cursor,
                lines: None,
                bytes: cursor.is_none().then_some(0),
                follow: cursor.is_some(),
            },
        )
        .await?;
        let Answer::Output { output } = answer else {
            return Err(ServerError::Runner {
                refusal: "runner_reply_malformed".to_owned(),
                words: format!("a seat exit read was answered {}", kind(&answer)),
            });
        };
        if output.session != driven.session {
            return Err(ServerError::Runner {
                refusal: "runner_reply_malformed".to_owned(),
                words: "the exit read names another session".to_owned(),
            });
        }
        if let Some(ended) = &output.ended {
            record_end(state, driven, ended)?;
            return Ok(true);
        }
        cursor = Some(output.cursor);
    }
}

/// Ask the monitor at `base` whether `name` is online, with the collector's
/// secret in its header when one is given.
#[cfg(test)]
pub(crate) async fn check_at(
    base: &str,
    secret: Option<(&HeaderName, &str)>,
    name: &str,
) -> Result<String, ServerError> {
    let address = format!("{}/api/seats", base.trim_end_matches('/'));
    let client = reqwest::Client::builder()
        .timeout(ASKED_FOR)
        .build()
        .map_err(|error| ServerError::ConfigInvalid {
            reason: format!("the client that asks the monitor could not be built: {error}"),
        })?;
    let mut request = client.get(&address);
    if let Some((header, secret)) = secret {
        request = request.header(header, secret);
    }
    let answered = match request.send().await {
        Ok(answered) => answered,
        Err(error) => {
            return Ok(format!(
                "the monitor at {base} could not be reached, so it was not known whether seat {name} is online there: {error}"
            ));
        }
    };
    let status = answered.status();
    if !status.is_success() {
        return Ok(format!(
            "the monitor at {base} answered {status} to its seats listing, so it was not known whether seat {name} is online there"
        ));
    }
    let body = match answered.json::<Value>().await {
        Ok(body) => body,
        Err(error) => {
            return Ok(format!(
                "the monitor at {base} answered a seats listing that does not read: {error}"
            ));
        }
    };
    match online_in(&body, name) {
        Ok(Some(true)) => Err(SeatError::OnlineInMonitor {
            name: name.to_owned(),
            monitor: base.to_owned(),
        }
        .into()),
        Ok(Some(false)) => Ok(format!("the monitor at {base} lists seat {name} offline")),
        Ok(None) => Ok(format!(
            "the monitor at {base} does not know whether seat {name} is online, or does not list it"
        )),
        Err(reason) => Ok(format!(
            "the monitor at {base} answered a seats listing that does not read: {reason}"
        )),
    }
}

/// Whether the monitor's seats listing `body` names `name` online: none when
/// it does not list the name or does not know.
#[cfg(test)]
pub(crate) fn online_in(body: &Value, name: &str) -> Result<Option<bool>, String> {
    let seats = body
        .get("seats")
        .and_then(Value::as_array)
        .ok_or("it names no seats list")?;
    for seat in seats {
        if seat.get("name").and_then(Value::as_str) == Some(name) {
            return match seat.get("online") {
                Some(Value::Bool(online)) => Ok(Some(*online)),
                Some(Value::Null) | None => Ok(None),
                Some(other) => Err(format!("seat {name}'s online is {other}, not a boolean")),
            };
        }
    }
    Ok(None)
}

#[cfg(test)]
#[path = "seats_monitor_tests.rs"]
mod tests;
