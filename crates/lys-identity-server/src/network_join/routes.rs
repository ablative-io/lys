//! Personal connection codes are admitted before they are issued.

use std::net::IpAddr;
use std::str::FromStr;
use std::sync::Arc;

use axum::Json;
use axum::extract::rejection::JsonRejection;
use axum::extract::{Path as RoutePath, State};
use axum::http::HeaderMap;
use lys_identity::OperationId;
use zeroize::Zeroize;

use super::{JoinCodeBody, JoinCodeGiven, RunnerJoinBody, RunnerJoined, with_joins};
use crate::error::ServerError;
use crate::error_machine::MachineError;
use crate::network_api::with_network;
use crate::routes::{AppState, signed_in};
use crate::runner_client::RunnerRecord;
use crate::session::now;

fn malformed(rejection: &JsonRejection) -> ServerError {
    ServerError::RequestMalformed {
        reason: rejection.body_text(),
    }
}

/// Where a runner on another computer dials this Lys: this service's public
/// address as the configuration names it, the callback's origin and folder.
/// Refused by name where another computer cannot reach it: an address on
/// this computer alone, or plain http, which a dialling runner speaks only
/// to its own computer.
pub(crate) fn dial_address(redirect: &str) -> Result<String, ServerError> {
    let refused = |reason: String| ServerError::from(MachineError::JoinUnreachable { reason });
    let url = reqwest::Url::parse(redirect).map_err(|error| {
        refused(format!(
            "this Lys's own address, {redirect}, does not read as an address: {error}"
        ))
    })?;
    let origin = url.origin().ascii_serialization();
    let host = url.host_str().unwrap_or_default();
    let bare = host.trim_start_matches('[').trim_end_matches(']');
    if bare.is_empty()
        || bare.eq_ignore_ascii_case("localhost")
        || bare
            .parse::<IpAddr>()
            .is_ok_and(|address| address.is_loopback())
    {
        return Err(refused(format!(
            "this Lys is served only at {origin}, an address on the computer it runs on, so another computer cannot reach it. Serve Lys at an address the other computer can reach, over https, and ask again"
        )));
    }
    if url.scheme() != "https" {
        return Err(refused(format!(
            "this Lys is served at {origin} over plain http, and another computer connects its runner to Lys only over https. Serve Lys over https and ask again"
        )));
    }
    let folder = url
        .path()
        .rsplit_once('/')
        .map_or("", |(folder, _callback)| folder);
    Ok(format!("{origin}{folder}"))
}

/// `text` as one word of a shell command: as it is when every character is
/// plain, otherwise in single quotes.
fn word(text: &str) -> String {
    let plain =
        |character: char| character.is_ascii_alphanumeric() || "-_./:@%+=,".contains(character);
    if !text.is_empty() && text.chars().all(plain) {
        text.to_owned()
    } else {
        format!("'{}'", text.replace('\'', r"'\''"))
    }
}

pub(super) async fn issue(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    RoutePath(id): RoutePath<String>,
    given: Result<Json<JoinCodeBody>, JsonRejection>,
) -> Result<Json<JoinCodeGiven>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    let Json(body) = given.map_err(|rejection| malformed(&rejection))?;
    let operation = OperationId::from_str(&body.operation)?.to_string();
    let by = crate::network_api::computer_owner(&state, &actor, &id)?;
    let server = dial_address(state.oidc.redirect_url())?;
    let code = with_joins(&state, |joins| joins.issue(&operation, &id, &by, now()))?;
    let command = format!(
        "lys runner join --server {} --machine {}",
        word(&server),
        word(&id)
    );
    Ok(Json(JoinCodeGiven {
        machine: id,
        server,
        command,
        code,
    }))
}

pub(super) async fn join(
    State(state): State<Arc<AppState>>,
    given: Result<Json<RunnerJoinBody>, JsonRejection>,
) -> Result<Json<RunnerJoined>, ServerError> {
    let Json(mut body) = given.map_err(|rejection| malformed(&rejection))?;
    let record = RunnerRecord::Dialled {
        key: body.key.clone(),
        runner: None,
    }
    .checked();
    let redeemed = record.and_then(|record| {
        with_joins(&state, |joins| {
            joins.redeem(&body.machine, &body.code, &body.key, now())
        })
        .map(|by| (record, by))
    });
    body.code.zeroize();
    let (record, (by, identity)) = redeemed?;
    with_network(&state, |store| {
        store.name_runner(&body.machine, Some(record))
    })?;
    (state.say)(&format!(
        "computer {} joined with key {} as {identity}, under a connection code given by {by}",
        body.machine, body.key
    ));
    Ok(Json(RunnerJoined {
        machine: body.machine,
        server_key: lys_runner::protocol::hex(&state.runners.public_key()),
    }))
}
