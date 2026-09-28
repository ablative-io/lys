//! The runner routes: an agent's session is typed to, sent keys, read,
//! waited on, resized, compacted, ended and woken through Lys, whichever
//! runner holds it, and so the API and the MCP surface carry each act.
//!
//! - `POST /runtime/sessions/{id}/input` `{"text", "enter"}`
//! - `POST /runtime/sessions/{id}/keys` `{"keys": ["enter", "ctrl_c", ..]}`
//! - `POST /runtime/sessions/{id}/read` `{"cursor"?, "lines"?, "bytes"?, "follow"?}`
//! - `POST /runtime/sessions/{id}/wait` `{"pattern", "regex"?, "cursor"?}`
//! - `POST /runtime/sessions/{id}/resize` `{"columns", "rows"}`
//! - `POST /runtime/sessions/{id}/compact` `{}`
//! - `POST /runtime/sessions/{id}/end` `{}`
//! - `POST /agents/{id}/wake` `{"message"}`
//! - `GET /runtime/live`: the sessions still running, as their runners say,
//!   and by name each session whose runner did not answer
//! - `GET`, `POST /network/machines/{id}/runner`: a machine's runner
//! - `GET /runner/protocol`: the runner protocol, published
//!
//! Each act requires the operate relation on the session's agent and is
//! refused `not_permitted` without it. Each admitted act leaves a receipt
//! naming the caller and the act, whatever the runner answered; typed text,
//! a message and a pattern ride in it as length and SHA-256 digest only,
//! for every profile. A wait, and a read that follows, end when the runner
//! answers or when the caller closes the request; nothing ends one on a
//! clock.

use std::sync::Arc;

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::routing::{get, post};
use axum::{Json, Router};
use lys_runner::{Act, Key};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::error::ServerError;
use crate::network_api::with_network;
use crate::provisioning_store::SessionSettings;
use crate::routes::{AppState, signed_in};
use crate::runner_acts::{Digested, RunnerAct};
use crate::runner_client::RunnerRecord;
use crate::runner_sessions::{Driven, driven, ended_in, keep_act, kind, operator, record_end};
use crate::runtime_api::with_runtime;
use crate::session::now;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct InputBody {
    text: String,
    #[serde(default)]
    enter: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct KeysBody {
    keys: Vec<Key>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReadBody {
    #[serde(default)]
    cursor: Option<u64>,
    #[serde(default)]
    lines: Option<u32>,
    #[serde(default)]
    bytes: Option<u64>,
    #[serde(default)]
    follow: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WaitBody {
    pattern: String,
    #[serde(default)]
    regex: bool,
    #[serde(default)]
    cursor: Option<u64>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ResizeBody {
    columns: u16,
    rows: u16,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Empty {}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WakeBody {
    message: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RunnerBody {
    runner: Option<RunnerRecord>,
}

/// The runner routes.
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/runtime/sessions/{id}/input", post(input))
        .route("/runtime/sessions/{id}/keys", post(keys))
        .route("/runtime/sessions/{id}/read", post(read))
        .route("/runtime/sessions/{id}/wait", post(wait))
        .route("/runtime/sessions/{id}/resize", post(resize))
        .route("/runtime/sessions/{id}/compact", post(compact))
        .route("/runtime/sessions/{id}/end", post(end))
        .route("/agents/{id}/wake", post(wake))
        .route("/runtime/live", get(live))
        .route(
            "/network/machines/{id}/runner",
            get(runner).post(name_runner),
        )
        .route("/runner/protocol", get(protocol))
        .merge(crate::runner_dial::routes())
}

fn malformed(reason: impl Into<String>) -> ServerError {
    ServerError::RequestMalformed {
        reason: reason.into(),
    }
}

fn body<T>(body: Result<Json<T>, JsonRejection>) -> Result<T, ServerError> {
    body.map(|Json(body)| body)
        .map_err(|refused| malformed(refused.body_text()))
}

/// What a receipt keeps of an act beside its name.
#[derive(Default)]
struct Carried {
    text: Option<Digested>,
    keys: Vec<String>,
}

/// Ask `act` of `driven`'s runner for `caller`, keep its receipt whatever
/// the runner answered, and answer the runner's answer beside it.
async fn perform(
    state: &Arc<AppState>,
    (driven, caller, name): (&Driven, String, &str),
    carried: Carried,
    act: Act,
) -> Result<Json<Value>, ServerError> {
    let answered =
        crate::runner_client::ask(state, &driven.machine, driven.runner.clone(), act).await;
    if let Some(ended) = answered.as_ref().ok().and_then(ended_in) {
        record_end(state, driven, ended)?;
    }
    let outcome = answered
        .as_ref()
        .map_or_else(ServerError::name, |answer| kind(answer).to_owned());
    let receipt = keep_act(
        state,
        RunnerAct {
            act: name.to_owned(),
            caller,
            session: driven.session.clone(),
            agent: driven.agent.clone(),
            machine: driven.machine.clone(),
            at: now(),
            text: carried.text,
            keys: carried.keys,
            outcome,
        },
    )?;
    let answer = answered?;
    Ok(Json(json!({
        "session": driven.session,
        "answer": answer,
        "receipt": receipt,
    })))
}

/// The session `id` and the caller admitted to operate it for `name`.
fn admitted(
    state: &AppState,
    headers: &HeaderMap,
    id: &str,
    name: &str,
) -> Result<(Driven, String), ServerError> {
    let driven = driven(state, id)?;
    let caller = operator(state, headers, &driven.agent, name)?;
    Ok((driven, caller))
}

async fn input(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    given: Result<Json<InputBody>, JsonRejection>,
) -> Result<Json<Value>, ServerError> {
    let given = body(given)?;
    let (driven, caller) = admitted(&state, &headers, &id, "input")?;
    let carried = Carried {
        text: Some(Digested::of(&given.text)),
        keys: Vec::new(),
    };
    let act = Act::Input {
        session: id,
        text: given.text,
        enter: given.enter,
    };
    perform(&state, (&driven, caller, "input"), carried, act).await
}

async fn keys(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    given: Result<Json<KeysBody>, JsonRejection>,
) -> Result<Json<Value>, ServerError> {
    let given = body(given)?;
    if given.keys.is_empty() {
        return Err(malformed("keys names at least one key"));
    }
    let (driven, caller) = admitted(&state, &headers, &id, "keys")?;
    let names = serde_json::to_value(&given.keys)
        .and_then(serde_json::from_value::<Vec<String>>)
        .map_err(|error| malformed(format!("the keys do not name themselves: {error}")))?;
    let carried = Carried {
        text: None,
        keys: names,
    };
    let act = Act::Keys {
        session: id,
        keys: given.keys,
    };
    perform(&state, (&driven, caller, "keys"), carried, act).await
}

async fn read(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    given: Result<Json<ReadBody>, JsonRejection>,
) -> Result<Json<Value>, ServerError> {
    let given = body(given)?;
    let (driven, caller) = admitted(&state, &headers, &id, "read")?;
    let act = Act::Read {
        session: id,
        cursor: given.cursor,
        lines: given.lines,
        bytes: given.bytes,
        follow: given.follow,
    };
    perform(&state, (&driven, caller, "read"), Carried::default(), act).await
}

async fn wait(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    given: Result<Json<WaitBody>, JsonRejection>,
) -> Result<Json<Value>, ServerError> {
    let given = body(given)?;
    let (driven, caller) = admitted(&state, &headers, &id, "wait")?;
    let carried = Carried {
        text: Some(Digested::of(&given.pattern)),
        keys: Vec::new(),
    };
    let act = Act::Wait {
        session: id,
        cursor: given.cursor,
        pattern: given.pattern,
        regex: given.regex,
    };
    perform(&state, (&driven, caller, "wait"), carried, act).await
}

async fn resize(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    given: Result<Json<ResizeBody>, JsonRejection>,
) -> Result<Json<Value>, ServerError> {
    let given = body(given)?;
    let (driven, caller) = admitted(&state, &headers, &id, "resize")?;
    let act = Act::Resize {
        session: id,
        columns: given.columns,
        rows: given.rows,
    };
    perform(&state, (&driven, caller, "resize"), Carried::default(), act).await
}

/// The session settings of `agent`'s latest profile version; none when the
/// profile names none, or when the configuration keeps no profiles.
pub(crate) fn session_settings(
    state: &AppState,
    agent: &str,
) -> Result<Option<SessionSettings>, ServerError> {
    if state.provisioning.is_none() {
        return Ok(None);
    }
    crate::provisioning_api::with_provisioning(state, |store| {
        Ok(store
            .profile(agent)
            .and_then(|profile| profile.versions.last())
            .and_then(|version| version.settings.session.clone()))
    })
}

async fn compact(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    given: Result<Json<Empty>, JsonRejection>,
) -> Result<Json<Value>, ServerError> {
    body(given)?;
    let (driven, caller) = admitted(&state, &headers, &id, "compact")?;
    let command = session_settings(&state, &driven.agent)?
        .and_then(|settings| settings.compact)
        .ok_or_else(|| ServerError::Runner {
            refusal: "compact_unnamed".to_owned(),
            words: format!(
                "the profile of {} names no compaction command",
                driven.agent
            ),
        })?;
    let carried = Carried {
        text: Some(Digested::of(&command)),
        keys: Vec::new(),
    };
    let act = Act::Input {
        session: id,
        text: command,
        enter: true,
    };
    perform(&state, (&driven, caller, "compact"), carried, act).await
}

async fn end(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    given: Result<Json<Empty>, JsonRejection>,
) -> Result<Json<Value>, ServerError> {
    body(given)?;
    let (driven, caller) = admitted(&state, &headers, &id, "end")?;
    let act = Act::End { session: id };
    perform(&state, (&driven, caller, "end"), Carried::default(), act).await
}

/// The sessions of `agent` not confirmed stopped, newest first, each on a
/// machine that names a runner.
fn open_sessions(state: &AppState, agent: &str) -> Result<Vec<Driven>, ServerError> {
    if state.runtime.is_none() {
        return Ok(Vec::new());
    }
    let open: Vec<String> = with_runtime(state, |store| {
        Ok(store
            .sessions()
            .iter()
            .rev()
            .filter(|tracked| tracked.agent.as_deref() == Some(agent) && !tracked.stopped())
            .map(|tracked| tracked.session.clone())
            .collect())
    })?;
    let mut driven_sessions = Vec::new();
    for session in open {
        match driven(state, &session) {
            Ok(found) => driven_sessions.push(found),
            Err(ServerError::RunnerAbsent { .. }) => {}
            Err(other) => return Err(other),
        }
    }
    Ok(driven_sessions)
}

async fn wake(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(agent): Path<String>,
    given: Result<Json<WakeBody>, JsonRejection>,
) -> Result<Json<Value>, ServerError> {
    let given = body(given)?;
    if given.message.trim().is_empty() {
        return Err(malformed("message is empty: say what wakes the agent"));
    }
    let caller = operator(&state, &headers, &agent, "wake")?;
    let prefix = session_settings(&state, &agent)?
        .map(|settings| settings.message_prefix)
        .unwrap_or_default();
    let delivered = format!("{prefix}{}", given.message);
    for driven in open_sessions(&state, &agent)? {
        let carried = Carried {
            text: Some(Digested::of(&delivered)),
            keys: Vec::new(),
        };
        let act = Act::Input {
            session: driven.session.clone(),
            text: delivered.clone(),
            enter: true,
        };
        match perform(&state, (&driven, caller.clone(), "wake"), carried, act).await {
            Err(ServerError::Runner { refusal, .. })
                if matches!(refusal.as_str(), "session_ended" | "session_unknown") => {}
            answered => return answered,
        }
    }
    Err(ServerError::NoLiveSession { agent })
}

/// The sessions the caller may see that are not confirmed stopped, each
/// asked of its runner first, so a session its runner saw end is kept
/// stopped and not shown. A runner that does not answer is named beside
/// the list, session by session, with its refusal: its session is listed
/// as last reported, and never shown as its runner's word.
async fn live(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Value>, ServerError> {
    let seen = crate::runtime_api::visible_sessions(&state, &headers)?;
    let mut unanswered = Vec::new();
    for tracked in seen.iter().filter(|tracked| !tracked.stopped()) {
        let driven = match driven(&state, &tracked.session) {
            Ok(driven) => driven,
            Err(ServerError::RunnerAbsent { .. }) => continue,
            Err(other) => return Err(other),
        };
        let act = Act::Status {
            session: Some(driven.session.clone()),
        };
        match crate::runner_client::ask(&state, &driven.machine, driven.runner.clone(), act).await {
            Ok(answer) => {
                if let Some(ended) = ended_in(&answer) {
                    record_end(&state, &driven, ended)?;
                }
            }
            Err(refused) => {
                (state.say)(&format!(
                    "runner of machine {} did not answer for session {}: {refused}",
                    driven.machine, driven.session
                ));
                unanswered.push(json!({
                    "session": driven.session,
                    "machine": driven.machine,
                    "refusal": refused.name(),
                    "reason": refused.to_string(),
                }));
            }
        }
    }
    let sessions: Vec<_> = crate::runtime_api::visible_sessions(&state, &headers)?
        .iter()
        .filter(|tracked| !tracked.stopped())
        .filter_map(|tracked| crate::runtime_api::view(&state, tracked))
        .collect();
    Ok(Json(
        json!({ "sessions": sessions, "unanswered": unanswered }),
    ))
}

async fn runner(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<Value>, ServerError> {
    signed_in(&state, &headers)?;
    let runner = with_network(&state, |store| {
        store.machine(&id).ok_or(ServerError::MachineUnknown)?;
        Ok(store.runner(&id).cloned())
    })?;
    Ok(Json(json!({ "machine": id, "runner": runner })))
}

async fn name_runner(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    given: Result<Json<RunnerBody>, JsonRejection>,
) -> Result<Json<Value>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    state.admission.administrator(&actor)?;
    let runner = body(given)?.runner.map(RunnerRecord::checked).transpose()?;
    with_network(&state, |store| store.name_runner(&id, runner.clone()))?;
    Ok(Json(json!({ "machine": id, "runner": runner })))
}

async fn protocol() -> Json<Value> {
    Json(lys_runner::published::section())
}
