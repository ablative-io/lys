//! Raw PTY transport uses the same operator grant and durable receipt path as text controls.

use std::sync::Arc;

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::routing::post;
use axum::{Json, Router};
use lys_runner::Act;
use serde::Deserialize;
use serde_json::Value;

use crate::error::ServerError;
use crate::routes::AppState;
use crate::runner_acts::Digested;
use crate::runner_api::{Carried, admitted, perform};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Read {
    cursor: Option<u64>,
    follow: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    data: Vec<u8>,
}

pub(crate) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/runtime/sessions/{id}/read-bytes", post(read))
        .route("/runtime/sessions/{id}/input-bytes", post(input))
}

fn body<T>(given: Result<Json<T>, JsonRejection>) -> Result<T, ServerError> {
    given
        .map(|Json(value)| value)
        .map_err(|error| ServerError::RequestMalformed {
            reason: error.body_text(),
        })
}

async fn read(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    given: Result<Json<Read>, JsonRejection>,
) -> Result<Json<Value>, ServerError> {
    let given = body(given)?;
    let (driven, caller) = admitted(&state, &headers, &id, "read_bytes")?;
    perform(
        &state,
        (&driven, caller, "read_bytes"),
        Carried::default(),
        Act::ReadBytes {
            session: id,
            cursor: given.cursor,
            follow: given.follow,
        },
    )
    .await
}

async fn input(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    given: Result<Json<Input>, JsonRejection>,
) -> Result<Json<Value>, ServerError> {
    let given = body(given)?;
    let (driven, caller) = admitted(&state, &headers, &id, "input_bytes")?;
    let carried = Carried {
        text: Some(Digested::bytes(&given.data)),
        keys: Vec::new(),
    };
    perform(
        &state,
        (&driven, caller, "input_bytes"),
        carried,
        Act::InputBytes {
            session: id,
            data: given.data,
        },
    )
    .await
}
