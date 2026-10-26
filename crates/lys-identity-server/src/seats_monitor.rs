//! Seat liveness belongs to the runner. Retired monitor settings are refused.
//! Terminal stop waits follow the runner's output and process exit signals.

use std::sync::Arc;

use crate::error::ServerError;
use crate::routes::AppState;
use crate::runner_acts::Digested;
use crate::runner_sessions::{Driven, kind, record_end};
use crate::seats_acts::act;
use lys_runner::operations::{Operation, OperationRequest, OperationState};
use lys_runner::{Act, Answer, Ended};

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

/// Deliver an exit operation and wait for the runner's exit evidence.
pub(crate) async fn stop_pty(
    state: &Arc<AppState>,
    driven: &Driven,
    caller: &str,
    operation: &str,
) -> Result<(bool, Vec<String>), ServerError> {
    crate::stop_api::asked(
        state,
        driven,
        caller,
        "Stop asked; waiting for the process exit",
    )?;
    stop_operation(
        state,
        driven,
        caller,
        operation,
        OperationRequest::Notice {
            text: "/exit".to_owned(),
        },
    )
    .await
    .map(|(_ended, notes)| (true, notes))
}

pub(crate) async fn stop_session(
    state: &Arc<AppState>,
    driven: &Driven,
    caller: &str,
    operation: &str,
) -> Result<(Ended, Vec<String>), ServerError> {
    crate::stop_api::asked(
        state,
        driven,
        caller,
        "Stop asked; waiting for the process exit",
    )?;
    let answer = crate::runner_client::ask(
        state,
        &driven.machine,
        driven.runner.clone(),
        Act::Liveness {
            session: Some(driven.session.clone()),
        },
    )
    .await?;
    let Answer::Liveness { sessions } = answer else {
        return Err(ServerError::Runner {
            refusal: "runner_reply_malformed".to_owned(),
            words: "the stop's liveness read was not answered with liveness".to_owned(),
        });
    };
    let [live] = sessions.as_slice() else {
        return Err(ServerError::Runner {
            refusal: "runner_reply_malformed".to_owned(),
            words: "the stop's liveness read did not name exactly one session".to_owned(),
        });
    };
    if live.session != driven.session {
        return Err(ServerError::Runner {
            refusal: "runner_reply_malformed".to_owned(),
            words: "the stop's liveness read names another session".to_owned(),
        });
    }
    if live.ended {
        return Ok((reaped(state, driven).await?, Vec::new()));
    }
    let request = if live.harness.managed {
        OperationRequest::Stop
    } else {
        OperationRequest::Notice {
            text: "/exit".to_owned(),
        }
    };
    stop_operation(state, driven, caller, operation, request).await
}

async fn stop_operation(
    state: &Arc<AppState>,
    driven: &Driven,
    caller: &str,
    operation: &str,
    request: OperationRequest,
) -> Result<(Ended, Vec<String>), ServerError> {
    let text = match &request {
        OperationRequest::Notice { text } => Some(Digested::of(text)),
        _ => None,
    };
    let exit = Operation {
        operation: operation.to_owned(),
        session: driven.session.clone(),
        request,
    };
    let (answer, _) = act(
        state,
        (driven, caller, "seat_stop"),
        text,
        Act::Operate {
            operation: exit.clone(),
        },
    )
    .await?;
    let outcome = crate::seats_send::outcome(answer, &exit)?;
    let mut notes = vec![format!("stop operation {operation}: {:?}", outcome.state)];
    if outcome.state == OperationState::Refused {
        return Err(ServerError::Runner {
            refusal: "operation_refused".to_owned(),
            words: outcome.words,
        });
    }
    let ended = reaped(state, driven).await?;
    notes.push("the runner reaped the process after the stop".to_owned());
    Ok((ended, notes))
}

pub(crate) async fn exited(state: &Arc<AppState>, driven: &Driven) -> Result<bool, ServerError> {
    reaped(state, driven).await.map(|_ended| true)
}

async fn reaped(state: &Arc<AppState>, driven: &Driven) -> Result<Ended, ServerError> {
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
        if let Some(ended) = output.ended {
            ended.require_reaped(&driven.session)?;
            record_end(state, driven, &ended)?;
            return Ok(ended);
        }
        cursor = Some(output.cursor);
    }
}

#[cfg(test)]
#[path = "seats_monitor_tests.rs"]
mod tests;
