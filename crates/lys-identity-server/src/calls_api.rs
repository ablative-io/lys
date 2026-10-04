//! An agent's model calls, as the proxy saw them pass.
//!
//! `GET /agents/{id}/calls` lists them newest first from the service's own
//! kept uses: each call the proxy's usage file reported is one use, and the
//! budgets' index holds their positions, so a page is a walk of that many
//! index rows and no file is opened. `GET /agents/{id}/calls/{call}` reads
//! one call whole from the proxy's home on this computer, by the session and
//! entry its use names: one seek and one block read for each body, off the
//! request's own thread. A call made on another computer is kept there, and
//! the answer names that computer.
//!
//! A list shows figures and is admitted as the agent's usage is. A call
//! whole is everything the agent sent and was sent, so it is held to whoever
//! may read the agent's terminal.

use std::sync::Arc;

use axum::extract::rejection::QueryRejection;
use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::routing::get;
use axum::{Json, Router};
use lys_home::record::Home;
use lys_home::record::call::whole::{CallWhole, call_whole};
use lys_runner::tracking::RecordAt;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::budgets_api::with_budgets;
use crate::budgets_state::Usage;
use crate::error::ServerError;
use crate::error_call::CallError;
use crate::routes::AppState;
use crate::runner_sessions::operator;

/// The rows one page holds; the page after it is asked for by its `next`.
const PAGE: usize = 100;

/// Which page of an agent's calls is asked for.
#[derive(Debug, Default, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CallsQuery {
    /// The `next` the preceding page answered; absent for the newest calls.
    pub after: Option<String>,
}

/// One model call an agent made, as its row is shown.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct CallRow {
    /// The proxy's id for the call.
    pub call_id: String,
    /// The Lys session that made it.
    pub session: Option<String>,
    /// When it ended, in milliseconds since the Unix epoch.
    pub at_ms: i64,
    /// The model asked for.
    pub model: Option<String>,
    /// Input tokens, as the provider counted them; null when it reported none.
    pub input_tokens: Option<u64>,
    /// Output tokens.
    pub output_tokens: Option<u64>,
    /// Tokens written to the cache.
    pub cache_creation_tokens: Option<u64>,
    /// Tokens read from the cache.
    pub cache_read_tokens: Option<u64>,
    /// The account the call drew on.
    pub account: Option<String>,
    /// How the call ended, in the record's own word: `complete`,
    /// `cancelled`, `partial`, `unrecorded` or `lost`.
    pub status: Option<String>,
    /// How long the call took, in milliseconds.
    pub duration_ms: Option<u64>,
}

/// One page of an agent's calls, newest first.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct CallsView {
    /// The agent.
    pub agent: String,
    /// The calls of this page.
    pub calls: Vec<CallRow>,
    /// What to ask the next, older page with; null when this is the last.
    pub next: Option<String>,
}

/// One call whole.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct CallView {
    /// The call's record as the proxy keeps it: its ids, model, status,
    /// timing, token figures and both heads. A credential header is named
    /// and never valued.
    #[schema(value_type = Object)]
    pub call: Value,
    /// What was sent, as JSON; null when it cannot be shown.
    #[schema(value_type = Object)]
    pub request: Option<Value>,
    /// Why the request cannot be shown as JSON.
    pub request_unreadable: Option<String>,
    /// What came back, as JSON; an event stream as its events in order.
    #[schema(value_type = Object)]
    pub response: Option<Value>,
    /// Why the response cannot be shown as JSON, or what of it is missing.
    pub response_unreadable: Option<String>,
}

/// The calls routes.
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/agents/{id}/calls", get(list))
        .route("/agents/{id}/calls/{call}", get(whole))
}

fn malformed(reason: impl Into<String>) -> ServerError {
    ServerError::RequestMalformed {
        reason: reason.into(),
    }
}

/// The index key a cursor names: the instant and position of the last row
/// of the page before.
fn before(after: Option<&str>) -> Result<Option<(i64, usize)>, ServerError> {
    after
        .map(|after| {
            after
                .split_once('.')
                .and_then(|(at, position)| Some((at.parse().ok()?, position.parse().ok()?)))
                .ok_or_else(|| malformed("after is not a cursor this route answered"))
        })
        .transpose()
}

fn row(usage: &Usage) -> Option<CallRow> {
    let call = usage.call.as_ref()?;
    Some(CallRow {
        call_id: call.id.clone(),
        session: usage.session.clone(),
        at_ms: usage.at_ms,
        model: call.model.clone(),
        input_tokens: call.input_tokens,
        output_tokens: call.output_tokens,
        cache_creation_tokens: call.cache_creation_tokens,
        cache_read_tokens: call.cache_read_tokens,
        account: usage.account.clone(),
        status: call.record.as_ref().map(|kept| kept.status.clone()),
        duration_ms: call.record.as_ref().and_then(|kept| kept.duration_ms),
    })
}

async fn list(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(agent): Path<String>,
    query: Result<Query<CallsQuery>, QueryRejection>,
) -> Result<Json<CallsView>, ServerError> {
    operator(&state, &headers, &agent, "usage")?;
    let Query(query) = query.map_err(|refused| malformed(refused.body_text()))?;
    let before = before(query.after.as_deref())?;
    with_budgets(&state, |store| {
        let held = store.held();
        let (positions, older) = held.index.calls(&agent, before, PAGE);
        let next = positions
            .last()
            .filter(|_| older)
            .and_then(|position| Some(format!("{}.{position}", held.uses.get(*position)?.at_ms)));
        let calls = positions
            .iter()
            .filter_map(|position| row(held.uses.get(*position)?))
            .collect();
        Ok(Json(CallsView { agent, calls, next }))
    })
}

/// The computer whose runner reported `usage`, from the event it is kept as.
fn machine(usage: &Usage, call: &str) -> String {
    usage
        .event
        .strip_prefix("native:")
        .and_then(|rest| rest.strip_suffix(call))
        .map_or(usage.event.as_str(), |machine| {
            machine.trim_end_matches(':')
        })
        .to_owned()
}

fn unavailable(reason: impl Into<String>) -> ServerError {
    CallError::RecordsUnavailable {
        reason: reason.into(),
    }
    .into()
}

/// Read the call from the proxy's home under `proxy`; a session that home
/// does not hold was recorded on the computer that reported it.
fn read(proxy: &std::path::Path, at: &RecordAt, machine: String) -> Result<CallWhole, ServerError> {
    let home = Home::read(proxy.join("home")).map_err(|error| unavailable(error.to_string()))?;
    let file = home
        .session_path(&at.session)
        .map_err(|error| unavailable(error.to_string()))?;
    if !file.is_file() {
        return Err(CallError::KeptElsewhere { machine }.into());
    }
    call_whole(&home, &at.session, &at.entry).map_err(|error| unavailable(error.to_string()))
}

async fn whole(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((agent, call)): Path<(String, String)>,
) -> Result<Json<CallView>, ServerError> {
    operator(&state, &headers, &agent, "read_bytes")?;
    let (at, machine) = with_budgets(&state, |store| {
        let held = store.held();
        let usage = held
            .index
            .call(&agent, &call)
            .and_then(|position| held.uses.get(position))
            .ok_or(CallError::Unknown)?;
        let at = usage.call.as_ref().and_then(|seen| seen.record.clone());
        Ok((at, machine(usage, &call)))
    })?;
    let at = at.ok_or_else(|| {
        unavailable("the call's usage line named no record, so the call cannot be found whole")
    })?;
    let proxy = state.proxy_dir.clone().ok_or_else(|| {
        unavailable("the configuration names no proxy_dir, so this service reads no call records")
    })?;
    let read = tokio::task::spawn_blocking(move || read(&proxy, &at, machine))
        .await
        .map_err(|error| unavailable(format!("reading the call did not finish: {error}")))??;
    if read.call.call_id != call {
        return Err(unavailable(
            "the entry the call's use names holds another call",
        ));
    }
    let record = serde_json::to_value(&read.call)
        .map_err(|error| unavailable(format!("the call's record is not JSON: {error}")))?;
    Ok(Json(CallView {
        call: record,
        request: read.request.json,
        request_unreadable: read.request.unreadable,
        response: read.response.json,
        response_unreadable: read.response.unreadable,
    }))
}
