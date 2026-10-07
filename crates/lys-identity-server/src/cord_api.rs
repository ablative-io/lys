//! The master off switch: one act by the administrator that stops every
//! session Lys started, on every computer, and holds every start back until
//! the administrator lets agents start again.
//!
//! - `POST /runtime/stop-everything` `{"operation", "reason", "kill"?}`:
//!   pull the cord (`cord_pull.rs` does the work)
//! - `POST /runtime/stop-everything/release` `{"operation"}`: let agents
//!   start again
//! - `GET /runtime/stop-everything`: how it stands, for anyone signed in, so
//!   a page can say why Start is refused
//!
//! While the cord is pulled every start is refused `everything_stopped`,
//! in words that say who stopped everything, when and why. The cord does
//! not suspend agents or withdraw certificates: that is each agent's own
//! emergency stop.

use std::str::FromStr;
use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::State;
use axum::extract::rejection::JsonRejection;
use axum::http::{HeaderMap, header};
use axum::routing::{get, post};
use axum::{Json, Router};
use lys_identity::OperationId;
use lys_runner::console_stop;
use serde::{Deserialize, Serialize};

use crate::cord_store::{CordStore, Kept, Pull, PullResult, Release};
use crate::error::ServerError;
use crate::error_cord::CordError;
use crate::routes::{AppState, signed_in, with_directory};
use crate::session::now;

/// A pull of the cord.
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = StopEverythingBody)]
pub(crate) struct PullBody {
    operation: String,
    reason: String,
    /// Kill anything that does not stop on a hang-up.
    #[serde(default)]
    kill: bool,
}

/// A release of the cord.
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = StopEverythingReleaseBody)]
pub(crate) struct ReleaseBody {
    operation: String,
}

/// How the cord stands.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[schema(as = StopEverythingView)]
pub struct CordView {
    /// The pull in force: who stopped everything, when and why; null while
    /// agents may start.
    pub pulled: Option<Pull>,
    /// What the latest pull did; null when none was ever pulled or the
    /// latest has not finished.
    pub last: Option<PullResult>,
    /// The latest release; null when none was ever made.
    pub released: Option<Release>,
    /// Whether the caller may pull the cord and release it: the
    /// administrator.
    pub may_pull: bool,
}

/// The cord's routes.
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/runtime/stop-everything", get(stands).post(pull))
        .route("/runtime/stop-everything/release", post(release))
        .route(console_stop::PATH, post(console))
}

/// The cord's routes' schemas.
pub(crate) fn types(api: &mut lys_openapi::Api) -> Vec<crate::openapi_types::Entry> {
    let view = api.schema::<CordView>();
    vec![
        (
            crate::openapi_table::GET,
            "/runtime/stop-everything",
            None,
            Some(view.clone()),
        ),
        (
            crate::openapi_table::POST,
            "/runtime/stop-everything",
            Some(api.schema::<PullBody>()),
            Some(api.schema::<PullResult>()),
        ),
        (
            crate::openapi_table::POST,
            console_stop::PATH,
            Some(api.schema::<console_stop::Request>()),
            Some(api.schema::<PullResult>()),
        ),
        (
            crate::openapi_table::POST,
            "/runtime/stop-everything/release",
            Some(api.schema::<ReleaseBody>()),
            Some(view),
        ),
    ]
}

fn console_refused(reason: impl Into<String>) -> ServerError {
    CordError::ConsoleSignatureRefused {
        reason: reason.into(),
    }
    .into()
}

/// The console credential is only present here; its signature still needs the bounded body.
pub(crate) fn console_signature(headers: &HeaderMap) -> Result<&str, ServerError> {
    if headers.contains_key(header::COOKIE) {
        return Err(console_refused(
            "a console signature cannot carry a session cookie",
        ));
    }
    let mut signatures = headers.get_all(console_stop::SIGNATURE_HEADER).iter();
    let signature = signatures.next().ok_or(ServerError::NotSignedIn)?;
    if signatures.next().is_some() {
        return Err(console_refused(
            "a console request carries exactly one signature",
        ));
    }
    let signature = signature.to_str().map_err(|error| {
        console_refused(format!(
            "the console signature is not hexadecimal text: {error}"
        ))
    })?;
    if signature.len() != 128 {
        return Err(console_refused(
            "the console signature must encode exactly 64 bytes",
        ));
    }
    Ok(signature)
}

async fn console(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    bytes: Bytes,
) -> Result<Json<PullResult>, ServerError> {
    let signature = lys_runner::protocol::unhex(console_signature(&headers)?)
        .ok_or_else(|| console_refused("the console signature is not hexadecimal text"))?;
    if lys_core::Ed25519Identity::verify(
        &state.runners.public_key(),
        &console_stop::signed_bytes(&bytes),
        &signature,
    )
    .is_err()
    {
        return Err(console_refused(
            "the signature does not verify this body under this service's key",
        ));
    }
    let given: console_stop::Request =
        serde_json::from_slice(&bytes).map_err(|error| ServerError::RequestMalformed {
            reason: error.to_string(),
        })?;
    let operation = OperationId::from_str(&given.operation)?.to_string();
    if given.by.trim().is_empty() || given.reason.trim().is_empty() {
        return Err(ServerError::RequestMalformed {
            reason:
                "the console claim and the reason must both name who stopped everything and why"
                    .to_owned(),
        });
    }
    crate::cord_pull::console(
        &state,
        Pull {
            operation,
            by: "console".to_owned(),
            by_name: Some(given.by),
            reason: given.reason,
            kill: given.kill,
            at: now(),
        },
    )
    .await
    .map(Json)
}

/// Run `act` on the cord, one caller at a time.
pub(crate) fn with_cord<T>(
    state: &AppState,
    act: impl FnOnce(&mut CordStore) -> Result<T, ServerError>,
) -> Result<T, ServerError> {
    let mut store = state.cord.lock().map_err(|error| CordError::Unavailable {
        reason: format!("the cord lock is poisoned: {error}"),
    })?;
    act(&mut store)
}

/// Refuse a start while the cord is pulled, by name, saying who stopped
/// everything, when and why.
pub(crate) fn refuse_start(state: &AppState) -> Result<(), ServerError> {
    let Some(pull) = with_cord(state, CordStore::standing)? else {
        return Ok(());
    };
    let who = pull.by_name.as_deref().unwrap_or("an administrator");
    let when = i64::try_from(pull.at)
        .ok()
        .and_then(|at| jiff::Timestamp::from_second(at).ok())
        .map_or_else(
            || format!("{} seconds after 1970", pull.at),
            |at| at.to_string(),
        );
    Err(CordError::EverythingStopped {
        words: format!(
            "{who} stopped every agent at {when}, because: {}. No agent can be started until an administrator lets agents start again",
            pull.reason
        ),
    }
    .into())
}

/// The caller, admitted as the administrator, by id and by name.
fn administrator(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<(String, Option<String>), ServerError> {
    let actor = signed_in(state, headers)?;
    crate::routes::administrator(state, &actor)?;
    with_directory(state, |directory| {
        let projection = directory.projection()?;
        let caller = crate::grants::caller(state, headers, projection)?;
        let name = projection
            .record(caller)
            .map(|record| record.profile().display_name().to_owned());
        Ok((caller.to_string(), name))
    })
}

fn body<T>(given: Result<Json<T>, JsonRejection>) -> Result<T, ServerError> {
    given
        .map(|Json(body)| body)
        .map_err(|refused| ServerError::RequestMalformed {
            reason: refused.body_text(),
        })
}

/// How the cord stands, for the caller.
fn view(state: &AppState, may_pull: bool) -> Result<CordView, ServerError> {
    let (pulled, (latest, released)) =
        with_cord(state, |store| Ok((store.standing()?, store.latest()?)))?;
    Ok(CordView {
        pulled,
        last: latest.and_then(|kept: Kept| kept.result),
        released,
        may_pull,
    })
}

async fn stands(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<CordView>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    let may_pull = crate::routes::is_administrator(&state, &actor)?;
    view(&state, may_pull).map(Json)
}

async fn pull(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    given: Result<Json<PullBody>, JsonRejection>,
) -> Result<Json<PullResult>, ServerError> {
    let given = body(given)?;
    let operation = OperationId::from_str(&given.operation)?.to_string();
    let reason = given.reason.trim().to_owned();
    if reason.is_empty() {
        return Err(ServerError::RequestMalformed {
            reason: "reason is empty: say why everything is stopped".to_owned(),
        });
    }
    let (by, by_name) = administrator(&state, &headers)?;
    let pull = Pull {
        operation,
        by,
        by_name,
        reason,
        kill: given.kill,
        at: now(),
    };
    crate::cord_pull::pull(&state, &headers, pull)
        .await
        .map(Json)
}

async fn release(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    given: Result<Json<ReleaseBody>, JsonRejection>,
) -> Result<Json<CordView>, ServerError> {
    let given = body(given)?;
    let operation = OperationId::from_str(&given.operation)?.to_string();
    let (by, by_name) = administrator(&state, &headers)?;
    with_cord(&state, |store| {
        store.release((operation, by, by_name, now()))
    })?;
    view(&state, true).map(Json)
}
