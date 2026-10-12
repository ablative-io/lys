//! Seat liveness belongs to the runner. Retired monitor settings are refused.
//! Terminal stop waits follow the runner's output and process exit signals.

use std::sync::Arc;
use std::time::Duration;

use crate::error::ServerError;
use crate::routes::AppState;
use crate::runner_acts::Digested;
use crate::runner_sessions::{Driven, kind, record_end};
use crate::seats_acts::act;
use lys_runner::operations::{Operation, OperationRequest, OperationState};
use lys_runner::{Act, Answer};

/// The bounded grace period after accepting the terminal's exit operation.
pub(crate) const STOP_WAIT: Duration = Duration::from_secs(30);

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

#[cfg(test)]
#[path = "seats_monitor_tests.rs"]
mod tests;
