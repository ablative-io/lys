//! Owners reconnect machines; signed machines alone report and hold their control stream.
use crate::error::ServerError;
use crate::routes::{AppState, signed_in};
use crate::runner_client::{RunnerRecord, dial_key};
use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{Value, json};
use std::sync::Arc;

pub(super) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/network/machines/{id}/connection", get(status))
        .route("/network/machines/{id}/reconnect", post(reconnect))
        .route("/runner/dial/{machine}/control", post(control))
        .route("/runner/dial/{machine}/connection", post(report))
}

fn owner(
    state: &AppState,
    headers: &HeaderMap,
    machine: &str,
) -> Result<RunnerRecord, ServerError> {
    let actor = signed_in(state, headers)?;
    crate::network_api::computer_owner(state, &actor, machine)?;
    crate::runner_sessions::machine_runner(state, machine)?.ok_or_else(|| {
        ServerError::RunnerAbsent {
            machine: machine.to_owned(),
        }
    })
}

async fn status(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(machine): Path<String>,
) -> Result<Json<Value>, ServerError> {
    let record = owner(&state, &headers, &machine)?;
    if matches!(record, RunnerRecord::Dialled { .. }) {
        return state.runners.hub().connection_status(&machine).map(Json);
    }
    let socket = state.runners.connection_socket(record)?;
    local_control(&socket, &machine, b'S').await.map(Json)
}

async fn reconnect(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(machine): Path<String>,
) -> Result<Json<Value>, ServerError> {
    let record = owner(&state, &headers, &machine)?;
    if matches!(record, RunnerRecord::Dialled { .. }) {
        let generation = state.runners.hub().reconnect(&machine)?;
        return Ok(Json(json!({"reconnecting":true,"generation":generation})));
    }
    let socket = state.runners.connection_socket(record)?;
    local_control(&socket, &machine, b'R').await.map(Json)
}

async fn local_control(
    socket: &std::path::Path,
    machine: &str,
    command: u8,
) -> Result<Value, ServerError> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let failed = |words: String| ServerError::Runner {
        refusal: "runner_reconnect_control_failed".to_owned(),
        words,
    };
    let path = socket.with_extension("reconnect.sock");
    let mut call = crate::waits::Call::begin(
        "runner",
        &path.to_string_lossy(),
        if command == b'S' {
            "connection status"
        } else {
            "reconnect"
        },
        machine,
    )?;
    let work = async {
        let mut stream = tokio::net::UnixStream::connect(&path)
            .await
            .map_err(|error| failed(error.to_string()))?;
        stream
            .write_all(&[command])
            .await
            .map_err(|error| failed(error.to_string()))?;
        let mut bytes = Vec::new();
        stream
            .take(1_048_576)
            .read_to_end(&mut bytes)
            .await
            .map_err(|error| failed(error.to_string()))?;
        let value: Value = serde_json::from_slice(&bytes).map_err(|error| {
            failed(format!(
                "invalid control response: {:?} at {}:{}",
                error.classify(),
                error.line(),
                error.column()
            ))
        })?;
        if let Some(reason) = value.get("refusal").and_then(Value::as_str) {
            return Err(failed(reason.to_owned()));
        }
        Ok(value)
    };
    call.until(work).await.map_err(failed)?
}

fn admitted(
    state: &AppState,
    machine: &str,
    headers: &HeaderMap,
    route: &str,
    body: &[u8],
) -> Result<(), ServerError> {
    let Some(RunnerRecord::Dialled { key, .. }) =
        crate::runner_sessions::machine_runner(state, machine)?
    else {
        return Err(ServerError::Runner {
            refusal: "runner_dial_refused".to_owned(),
            words: "This machine names no dialled runner.".to_owned(),
        });
    };
    let key = dial_key(&key).ok_or_else(|| ServerError::Runner {
        refusal: "runner_dial_refused".to_owned(),
        words: "This machine's recorded dial key is invalid.".to_owned(),
    })?;
    crate::runner_dial::signed_by(state, machine, &key, headers, route, body)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ControlBody {
    after: u64,
    connection: Value,
}

fn checked_status(value: &Value) -> Result<(), ServerError> {
    if !value.get("state").is_some_and(Value::is_string) {
        return Err(ServerError::RequestMalformed {
            reason: "runner_connection_invalid: connection state is absent".to_owned(),
        });
    }
    if let Some(retry) = value.get("retry").filter(|value| !value.is_null()) {
        if !["refused_at", "next_dial", "attempt"]
            .iter()
            .all(|name| retry.get(name).is_some_and(Value::is_u64))
            || !retry.get("reason").is_some_and(Value::is_string)
        {
            return Err(ServerError::RequestMalformed {
                reason: "runner_connection_invalid: retry instants, attempt or reason are invalid"
                    .to_owned(),
            });
        }
    }
    Ok(())
}

async fn control(
    State(state): State<Arc<AppState>>,
    Path(machine): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<Value>, ServerError> {
    admitted(
        &state,
        &machine,
        &headers,
        &format!("/runner/dial/{machine}/control"),
        &body,
    )?;
    let body: ControlBody =
        serde_json::from_slice(&body).map_err(|error| ServerError::RequestMalformed {
            reason: format!(
                "runner_control_invalid: {:?} at {}:{}",
                error.classify(),
                error.line(),
                error.column()
            ),
        })?;
    checked_status(&body.connection)?;
    state
        .runners
        .hub()
        .report_connection(&machine, body.connection)?;
    let generation = state
        .runners
        .hub()
        .control_after(&machine, body.after)
        .await?;
    Ok(Json(json!({"generation":generation})))
}

async fn report(
    State(state): State<Arc<AppState>>,
    Path(machine): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<Value>, ServerError> {
    admitted(
        &state,
        &machine,
        &headers,
        &format!("/runner/dial/{machine}/connection"),
        &body,
    )?;
    let status: Value =
        serde_json::from_slice(&body).map_err(|error| ServerError::RequestMalformed {
            reason: format!(
                "runner_connection_invalid: {:?} at {}:{}",
                error.classify(),
                error.line(),
                error.column()
            ),
        })?;
    checked_status(&status)?;
    state.runners.hub().report_connection(&machine, status)?;
    Ok(Json(json!({"recorded":true})))
}

#[cfg(test)]
#[path = "machine_connection_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "machine_connection_wait_tests.rs"]
mod wait_tests;
