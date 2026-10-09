//! The routes a runner on another machine dials in on. The server never
//! dials it: the machine's bridge asks for the next request, relays it to
//! its runner, and posts the runner's reply back.
//!
//! - `GET /runner/dial/epoch`: the epoch every dial is signed under, made
//!   fresh each time the server starts
//! - `POST /runner/dial/{machine}/next`: carries the greeting of the
//!   bridge's connection to its runner, and answers the next request for
//!   the machine, signed over that greeting, its ticket in the
//!   `x-lys-runner-ticket` header, once there is one; the ask ends then, or
//!   when the bridge leaves
//! - `POST /runner/dial/{machine}/replies/{ticket}`: the runner's reply
//! - `POST /runner/dial/{machine}/pass`: the machine the computer's join
//!   made asks for a pass to an app, signed as a dial is, but by the key
//!   its join recorded (`machine_pass.rs`, ACCESS-005)
//!
//! Each POST is admitted only when the machine's record names a dialled
//! runner and the request is signed by the key that record names, over the
//! dial domain, the method, the route, this server's epoch, a nonce used
//! once in it and the body. The first greeting a machine's bridge carries
//! pins the machine's record to that runner, unless the record names one
//! already; a greeting naming another runner is refused, so a request made
//! for the machine is signed for its own runner only. A dial signed under
//! another epoch is refused
//! `runner_dial_stale`, so a dial captured before a restart is never
//! admitted after it; anything else is refused `runner_dial_refused`, by
//! name. A greeting in another protocol version is answered to the caller
//! waiting on the request as `runner_protocol_mismatch`.

use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use lys_core::Ed25519Identity;
use lys_runner::dial::{
    EPOCH_HEADER, EPOCH_ROUTE, NONCE_HEADER, SIGNATURE_HEADER, TICKET_HEADER, dial_signed_bytes,
    next_route, reply_route,
};
use lys_runner::protocol::{read_greeting, unhex};
use serde_json::{Value, json};

use crate::error::ServerError;
use crate::machine_pass::PASS_ROUTE;
use crate::network_api::with_network;
use crate::routes::AppState;
use crate::runner_client::{RunnerRecord, dial_key};
use crate::runner_sessions::machine_runner;

/// The dial routes.
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(EPOCH_ROUTE, get(epoch))
        .route("/runner/dial/{machine}/next", post(next))
        .route("/runner/dial/{machine}/replies/{ticket}", post(reply))
        .route(PASS_ROUTE, post(crate::machine_pass::pass))
}

fn refused(reason: impl Into<String>) -> ServerError {
    crate::error_machine::MachineError::dial_refused(reason)
}

fn header<'a>(headers: &'a HeaderMap, name: &str) -> Result<&'a str, ServerError> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| refused(format!("the dial carries no {name} header")))
}

/// Admit a dial from `machine` on `route` carrying `body`.
fn admitted(
    state: &AppState,
    machine: &str,
    headers: &HeaderMap,
    route: &str,
    body: &[u8],
) -> Result<(), ServerError> {
    let Some(RunnerRecord::Dialled { key, .. }) = machine_runner(state, machine)? else {
        return Err(refused(format!(
            "machine `{machine}` names no dialled runner"
        )));
    };
    let key = dial_key(&key)
        .ok_or_else(|| refused(format!("machine `{machine}`'s key does not read")))?;
    signed_by(state, machine, &key, headers, route, body)
}

/// Admit a request from `machine` on `route` carrying `body` when it is
/// signed by `key` as a dial is: over the dial domain, the method, the
/// route, this server's epoch, a nonce used once in it, and the body. The
/// caller names the key: a dial's is its runner record's, a machine pass's
/// the key its join recorded (`machine_pass`).
pub(crate) fn signed_by(
    state: &AppState,
    machine: &str,
    key: &[u8; 32],
    headers: &HeaderMap,
    route: &str,
    body: &[u8],
) -> Result<(), ServerError> {
    let epoch = header(headers, EPOCH_HEADER)?;
    if epoch != state.runners.hub().epoch() {
        return Err(crate::error_machine::MachineError::DialStale {
            reason: format!("the dial names epoch `{epoch}`, which is not this server's"),
        }
        .into());
    }
    let nonce = header(headers, NONCE_HEADER)?;
    if nonce.len() != 32 || unhex(nonce).is_none() {
        return Err(refused(
            "the dial's nonce is not 16 random bytes in lowercase hex",
        ));
    }
    let signature = unhex(header(headers, SIGNATURE_HEADER)?)
        .ok_or_else(|| refused("the dial's signature is not lowercase hex"))?;
    Ed25519Identity::verify(
        key,
        &dial_signed_bytes("POST", route, epoch, nonce, body),
        &signature,
    )
    .map_err(|forged| {
        refused(format!(
            "the dial is not signed by machine `{machine}`'s key: {forged}"
        ))
    })?;
    if !state.runners.hub().fresh(machine, nonce)? {
        return Err(refused(format!("nonce `{nonce}` was already used")));
    }
    Ok(())
}

async fn next(
    State(state): State<Arc<AppState>>,
    Path(machine): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response, ServerError> {
    admitted(&state, &machine, &headers, &next_route(&machine), &body)?;
    let greeting = std::str::from_utf8(&body)
        .map_err(|error| refused(format!("the greeting is not text: {error}")))
        .map(|text| read_greeting(text.trim_end()));
    if let Ok(Ok(greeting)) = &greeting {
        pinned(&state, &machine, &greeting.runner)?;
    }
    let (ticket, act) = state.runners.hub().next(&machine).await?;
    let signed = match greeting {
        Ok(greeting) => greeting.and_then(|greeting| state.runners.sign(&greeting, &act)),
        Err(refusal) => {
            state.runners.hub().reply(
                &machine,
                &ticket,
                Err(lys_runner::RunnerError::ReplyMalformed {
                    reason: "the machine's bridge sent a greeting that is not text".to_owned(),
                }),
            )?;
            return Err(refusal);
        }
    };
    match signed {
        Ok(line) => Ok(([(TICKET_HEADER, ticket)], line).into_response()),
        Err(error) => {
            let answer = ServerError::Runner {
                refusal: error.name(),
                words: error.to_string(),
            };
            state.runners.hub().reply(&machine, &ticket, Err(error))?;
            Err(answer)
        }
    }
}

/// Hold `machine`'s bridge to the runner `runner`: pinned now when the
/// machine's record names none, refused `runner_dial_refused` when it names
/// another, so the bridge cannot carry another runner's greeting and have a
/// request made for this machine signed for that runner.
fn pinned(state: &AppState, machine: &str, runner: &str) -> Result<(), ServerError> {
    with_network(state, |store| match store.runner(machine).cloned() {
        Some(RunnerRecord::Dialled {
            runner: Some(held), ..
        }) if held == runner => Ok(()),
        Some(RunnerRecord::Dialled {
            runner: Some(held), ..
        }) => Err(refused(format!(
            "the greeting names runner `{runner}`, and machine `{machine}`'s record pins runner `{held}`: name the machine's runner again to pin another"
        ))),
        Some(RunnerRecord::Dialled { key, runner: None }) => {
            store.name_runner(
                machine,
                Some(RunnerRecord::Dialled {
                    key,
                    runner: Some(runner.to_owned()),
                }),
            )?;
            // Its runner has connected for the first time: a screen waiting
            // for it asks again.
            state.changes.signal()
        }
        _ => Err(refused(format!(
            "machine `{machine}` names no dialled runner"
        ))),
    })
}

/// The epoch every dial to this server is signed under.
async fn epoch(State(state): State<Arc<AppState>>) -> String {
    state.runners.hub().epoch().to_owned()
}

async fn reply(
    State(state): State<Arc<AppState>>,
    Path((machine, ticket)): Path<(String, String)>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<Value>, ServerError> {
    admitted(
        &state,
        &machine,
        &headers,
        &reply_route(&machine, &ticket),
        &body,
    )?;
    let line = String::from_utf8(body.to_vec())
        .map_err(|error| refused(format!("the reply is not text: {error}")))?;
    state.runners.hub().reply(&machine, &ticket, Ok(line))?;
    Ok(Json(json!({ "delivered": ticket })))
}
