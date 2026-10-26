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
//! - `GET /network/machines/{id}/runner`: a machine's runner record, and
//!   whether its runner answers now: `answers` is `true` when it answered a
//!   status request, `false` with the `refusal` and the `reason` when it did
//!   not, and `null` when the machine names no runner. A dialled runner none
//!   of whose bridges is dialled in is not asked: it answers `false`,
//!   `runner_not_dialled_in`. `joining` is `true` while a connection code
//!   given for the machine waits to be used (`network_join.rs`)
//! - `POST /network/machines/{id}/runner`: name a machine's runner
//! - `POST /network/machines/{id}/folders`: the folders its runner names
//!   (`machine_folders.rs`)
//! - `GET /runner/protocol`: the runner protocol, published
//!
//! Each act requires the operate relation on the session's agent and
//! answers `RuntimeSessionUnknown` when the session is absent or not visible.
//! Authentication precedes session and runner lookups. Each admitted act leaves a receipt
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

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct InputBody {
    text: String,
    #[serde(default)]
    enter: bool,
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct KeysBody {
    #[schema(value_type = Vec<String>)]
    keys: Vec<Key>,
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReadBody {
    #[serde(default)]
    cursor: Option<u64>,
    #[serde(default)]
    lines: Option<u32>,
    #[serde(default)]
    bytes: Option<u64>,
    #[serde(default)]
    follow: bool,
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct WaitBody {
    pattern: String,
    #[serde(default)]
    regex: bool,
    #[serde(default)]
    cursor: Option<u64>,
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ResizeBody {
    columns: u16,
    rows: u16,
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct Empty {}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct WakeBody {
    message: String,
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct RunnerBody {
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
        .merge(crate::machine_folders::routes())
        .merge(crate::runner_dial::routes())
        .merge(crate::network_join::routes())
        .merge(crate::runner_bytes_api::routes())
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
pub(crate) struct Carried {
    pub(crate) text: Option<Digested>,
    pub(crate) keys: Vec<String>,
}

/// Ask `act` of `driven`'s runner for `caller`, keep its receipt whatever
/// the runner answered, and answer the runner's answer beside it.
pub(crate) async fn perform(
    state: &Arc<AppState>,
    (driven, caller, name): (&Driven, String, &str),
    carried: Carried,
    act: Act,
) -> Result<Json<Value>, ServerError> {
    let require_reap = matches!(act, Act::End { .. });
    let act = if matches!(
        act,
        Act::Input { .. } | Act::InputBytes { .. } | Act::Keys { .. } | Act::Operate { .. }
    ) {
        Act::AsCaller {
            caller: caller.clone(),
            done: Box::new(act),
        }
    } else {
        act
    };
    let answered = crate::runner_client::ask(state, &driven.machine, driven.runner.clone(), act)
        .await
        .and_then(|answer| {
            if require_reap {
                let lys_runner::Answer::Ended { session, ended } = &answer else {
                    return Err(ServerError::Runner {
                        refusal: "runner_reply_malformed".to_owned(),
                        words: "the end was not answered with a reap".to_owned(),
                    });
                };
                if session != &driven.session {
                    return Err(ServerError::Runner {
                        refusal: "runner_reply_malformed".to_owned(),
                        words: "the end's reap names another session".to_owned(),
                    });
                }
                ended.require_reaped(session)?;
            }
            Ok(answer)
        });
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
pub(crate) fn admitted(
    state: &AppState,
    headers: &HeaderMap,
    id: &str,
    name: &str,
) -> Result<(Driven, String), ServerError> {
    signed_in(state, headers)?;
    crate::routes::with_directory(state, |directory| {
        crate::grants::caller(state, headers, directory.projection()?)
    })?;
    let agent = crate::runner_sessions::session_agent(state, id)?;
    let caller = match operator(state, headers, &agent, name) {
        Err(ServerError::NotPermitted { .. } | ServerError::AgentNotVisible) => {
            return Err(ServerError::RuntimeSessionUnknown);
        }
        other => other?,
    };
    let driven = driven(state, id)?;
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
    let (act, command, version) =
        crate::budgets_act::prepare_compact(&state, &driven.agent, &id).await?;
    let carried = Carried {
        text: Some(Digested::of(&command)),
        keys: Vec::new(),
    };
    let Json(mut answer) = perform(&state, (&driven, caller, "compact"), carried, act).await?;
    answer["profile_version"] = json!(version);
    Ok(Json(answer))
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
pub(crate) fn open_sessions(state: &AppState, agent: &str) -> Result<Vec<Driven>, ServerError> {
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
    let plain = format!("{prefix}{}", given.message);
    for driven in open_sessions(&state, &agent)? {
        // The wake-up slot's words, resolved for this session and rendered
        // now (AGENTS-001 R1, R3); the plain message with the profile's
        // prefix when no layer sets the slot, or when the words cannot be
        // rendered, which is said.
        let delivered =
            crate::runner_words::wake_words(&state, &agent, &driven.session, &given.message)
                .unwrap_or_else(|| plain.clone());
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

#[path = "runner_api/live.rs"]
mod runtime_live;
use runtime_live::live;

#[path = "runner_api/machine.rs"]
mod machine;
use machine::{name_runner, protocol, runner};
