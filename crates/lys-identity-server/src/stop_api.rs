//! The emergency stop of an agent: one act by the person responsible for
//! it or the administrator that suspends the agent, withdraws every
//! certificate it holds, ends every credential handle it holds at the
//! secrets broker, and asks the runtime of every session of its to end it.
//!
//! The stop never claims a session ended. Each session shows unconfirmed
//! from the ask until its runtime reports it stopped. Once every part is
//! done the stop is kept whole in its own log, so sent again under the same
//! operation id it answers exactly what was kept and does nothing twice;
//! the same operation in other words is refused `StopReused`. Each part is
//! kept once by its own record too, so a stop cut off before it was kept
//! does nothing twice when it is sent again; and since the suspension is
//! the first part and the directory keeps it under the stop's operation, a
//! stop cut off after it and sent again in other words is refused by that
//! record.
//!
//! The broker is asked after the suspension, the certificates and the
//! sessions are recorded, so a broker that cannot be reached leaves those
//! in force; its refusal is named in the answer, never hidden.
//!
//! A session on a machine whose record names a runner is ended through that
//! runner: the stop asks it to end the session and waits for its answer,
//! and the session shows confirmed, with the instant of its exit, only once
//! the runner reports the exit. A runner that refuses or cannot be reached
//! is named in the answer, and its session stays unconfirmed.

use std::str::FromStr;
use std::sync::{Arc, PoisonError};

use axum::body::Bytes;
use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, Method};
use axum::routing::{get, post};
use axum::{Json, Router};
use lys_identity::{AgentId, IdentityError, IdentityId, LifecycleState, OperationId, Transition};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::certificates_store::Withdrawn;
use crate::error::ServerError;
use crate::read_api::own_person;
use crate::routes::{AppState, hex, signed_in, with_directory};
use crate::runtime_api::with_runtime;
use crate::runtime_state::{Report, Reported};
use crate::session::now;
use crate::stops_state::Stop;
use crate::stops_store::StopStore;

/// The most characters a reason carries.
const REASON_MAX: usize = 500;

/// A stop to make.
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct StopBody {
    operation: String,
    reason: String,
}

/// The answer of the stop route: what was done, and what was only asked.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct StopView {
    /// The agent stopped.
    pub agent: String,
    /// The operation id the stop was made under.
    pub operation: String,
    /// `suspended` once every part of the stop is done; `asked` while the
    /// stop is kept as asked and its parts are not all done.
    pub state: String,
    /// Whether every part is done. A stop that is only asked claims
    /// nothing about certificates, sessions or credentials yet.
    pub done: bool,
    /// The person who stopped it.
    pub by: String,
    /// When, in seconds since the Unix epoch.
    pub at: u64,
    /// The serials of the certificates withdrawn.
    pub certificates_withdrawn: Vec<String>,
    /// The credential handles ended at the broker, by id; null when the
    /// broker refused, and then `credentials_refused` names why.
    pub credentials_ended: Option<Vec<String>>,
    /// The broker's refusal, by name, when the handles could not be ended.
    pub credentials_refused: Option<String>,
    /// The sessions whose runtimes were asked to end them. Each stays
    /// unconfirmed until its runtime reports it stopped.
    pub sessions_asked: Vec<String>,
    /// The asked sessions whose end is confirmed, each with its runtime's
    /// words, which carry the instant of its exit.
    pub sessions_confirmed: Vec<ConfirmedEnd>,
    /// The asked sessions whose runner refused to end them or could not be
    /// reached, each with the refusal by name.
    pub sessions_refused: Vec<String>,
    /// Why, in the stopper's words.
    pub reason: String,
}

/// An asked session whose end its runtime confirmed, never inferred.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct ConfirmedEnd {
    /// The session.
    pub session: String,
    /// When the confirmation was kept, in seconds since the Unix epoch.
    pub at: u64,
    /// The runtime's words, with the instant of the exit.
    pub confirmation: String,
}

impl From<Stop> for StopView {
    fn from(stop: Stop) -> Self {
        Self {
            agent: stop.agent,
            operation: stop.operation,
            state: if stop.done { "suspended" } else { "asked" }.to_owned(),
            done: stop.done,
            by: stop.by,
            at: stop.at,
            certificates_withdrawn: stop.certificates_withdrawn,
            credentials_ended: stop.credentials_ended,
            credentials_refused: stop.credentials_refused,
            sessions_asked: stop.sessions_asked,
            sessions_confirmed: Vec::new(),
            sessions_refused: Vec::new(),
            reason: stop.reason,
        }
    }
}

/// The stop route.
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/agents/{id}/stop", post(stop))
        .route("/agents/{id}/stops", get(stops))
}

/// The answer of `GET /agents/{id}/stops`.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct StopsView {
    /// Every stop kept on the agent, in the order kept.
    pub stops: Vec<StopView>,
}

/// Admit the caller to `agent`: the person responsible for it or the
/// administrator; anyone else is refused `AgentNotVisible`, as is an id
/// the directory does not hold.
fn admitted(
    state: &AppState,
    actor: &lys_identity::Actor,
    id: &str,
) -> Result<(AgentId, String, LifecycleState), ServerError> {
    let agent = AgentId::from_str(id).map_err(|_unread| ServerError::AgentNotVisible)?;
    let administrator = state.admission.administrator(actor).is_ok();
    with_directory(state, |directory| {
        let projection = directory.projection()?;
        let person = own_person(projection, actor)?;
        let record = projection
            .record(IdentityId::Agent(agent))
            .ok_or(ServerError::AgentNotVisible)?;
        if !administrator && record.responsible() != Some(person) {
            return Err(ServerError::AgentNotVisible);
        }
        Ok((agent, person.to_string(), record.state()))
    })
}

async fn stops(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<StopsView>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    let (agent, _by, _lifecycle) = admitted(&state, &actor, &id)?;
    let kept = with_stops(&state, |store| Ok(store.of_agent(&agent.to_string())))?;
    let mut stops = Vec::new();
    for stop in kept {
        stops.push(confirmed(&state, StopView::from(stop))?);
    }
    Ok(Json(StopsView { stops }))
}

/// `view` with each asked session whose end its runtime confirmed.
fn confirmed(state: &AppState, mut view: StopView) -> Result<StopView, ServerError> {
    if state.runtime.is_none() {
        return Ok(view);
    }
    view.sessions_confirmed = with_runtime(state, |store| {
        Ok(view
            .sessions_asked
            .iter()
            .filter_map(|session| store.session(session))
            .filter(|tracked| tracked.stopped())
            .filter_map(|tracked| {
                tracked.latest().map(|report| ConfirmedEnd {
                    session: tracked.session.clone(),
                    at: report.at,
                    confirmation: report.confirmation.clone(),
                })
            })
            .collect())
    })?;
    Ok(view)
}

/// Ask the runner of each session to end it, every session at once so no
/// runner waits on another, answering, by name, each session whose runner
/// refused or could not be reached. A session on a machine that names no
/// runner stays asked, as its runtime has not said.
async fn end_sessions(state: &Arc<AppState>, sessions: Vec<String>, by: &str) -> Vec<String> {
    let mut asked = tokio::task::JoinSet::new();
    for session in sessions {
        let (state, by) = (Arc::clone(state), by.to_owned());
        asked.spawn(async move { end_session(&state, &session, &by).await });
    }
    let mut refused = Vec::new();
    while let Some(ended) = asked.join_next().await {
        match ended {
            Ok(named) => refused.extend(named),
            Err(failed) => refused.push(format!(
                "runner_unreachable: asking a runner to end a session ended abnormally: {failed}"
            )),
        }
    }
    refused.sort();
    refused
}

/// Ask the runner of `session` to end it, answering each refusal by name.
async fn end_session(state: &Arc<AppState>, session: &str, by: &str) -> Vec<String> {
    let mut refused = Vec::new();
    let driven = match crate::runner_sessions::driven(state, session) {
        Ok(driven) => driven,
        Err(ServerError::RunnerAbsent { .. }) => return refused,
        Err(other) => {
            refused.push(format!("{session}: {other}"));
            return refused;
        }
    };
    let act = lys_runner::Act::End {
        session: session.to_owned(),
    };
    let answered =
        crate::runner_client::ask(state, &driven.machine, driven.runner.clone(), act).await;
    let kept = match &answered {
        Ok(lys_runner::Answer::Ended { ended, .. }) => {
            crate::runner_sessions::record_end(state, &driven, ended)
        }
        Ok(other) => Err(ServerError::Runner {
            refusal: "runner_reply_malformed".to_owned(),
            words: format!(
                "an end was answered {}",
                crate::runner_sessions::kind(other)
            ),
        }),
        Err(error) => Err(ServerError::Runner {
            refusal: error.name(),
            words: error.to_string(),
        }),
    };
    let outcome = kept
        .as_ref()
        .map_or_else(ServerError::name, |()| "ended".to_owned());
    let receipt = crate::runner_sessions::keep_act(
        state,
        crate::runner_acts::RunnerAct {
            act: "end".to_owned(),
            caller: by.to_owned(),
            session: session.to_owned(),
            agent: driven.agent.clone(),
            machine: driven.machine.clone(),
            at: now(),
            text: None,
            keys: Vec::new(),
            outcome,
        },
    );
    for failed in [kept.err(), receipt.err()].into_iter().flatten() {
        refused.push(format!("{session}: {failed}"));
    }
    refused
}

fn malformed(reason: impl Into<String>) -> ServerError {
    ServerError::RequestMalformed {
        reason: reason.into(),
    }
}

fn with_stops<T>(
    state: &AppState,
    act: impl FnOnce(&mut StopStore) -> Result<T, ServerError>,
) -> Result<T, ServerError> {
    let store = state
        .stops
        .as_ref()
        .ok_or_else(|| ServerError::StopsUnavailable {
            reason: "the configuration names no stops_dir".to_owned(),
        })?;
    let mut store = store.lock().unwrap_or_else(PoisonError::into_inner);
    store.settle()?;
    act(&mut store)
}

/// An operation id for one part of the stop, the same each time the same
/// stop is sent, so each part is kept once.
fn part(operation: &str, kind: &str, key: &str) -> String {
    let digest = Sha256::digest(format!("lys-identity/stop/v1\n{operation}\n{kind}\n{key}"));
    format!("op-{}", &hex(&digest)[..32])
}

async fn stop(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    body: Result<Json<StopBody>, JsonRejection>,
) -> Result<Json<StopView>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    let Json(body) = body.map_err(|refused| malformed(refused.body_text()))?;
    let operation = OperationId::from_str(&body.operation)?;
    let reason = body.reason.trim().to_owned();
    if reason.is_empty() {
        return Err(malformed("reason is empty: say why the agent is stopped"));
    }
    if reason.chars().count() > REASON_MAX {
        return Err(malformed(format!(
            "reason is longer than {REASON_MAX} characters"
        )));
    }
    let (agent, by, lifecycle) = admitted(&state, &actor, &id)?;
    let asked = Stop {
        operation: operation.to_string(),
        agent: agent.to_string(),
        by: by.clone(),
        reason: reason.clone(),
        at: now(),
        certificates_withdrawn: Vec::new(),
        sessions_asked: Vec::new(),
        credentials_ended: None,
        credentials_refused: None,
        done: false,
    };
    let suspended = with_directory(&state, |directory| {
        Ok(directory.transition(
            actor.clone(),
            operation,
            IdentityId::Agent(agent),
            Transition::Suspend,
            &reason,
            now(),
        ))
    })?;
    match (suspended, lifecycle) {
        // Suspended now, or before by this same stop and answered again; or
        // suspended by another act, and the stop still withdraws, asks and ends.
        (Ok(_), _) | (Err(IdentityError::TransitionRefused { .. }), LifecycleState::Suspended) => {}
        // This operation already suspended the agent in other words: a stop cut
        // off before it was kept, sent again changed.
        (Err(IdentityError::OperationReused { .. }), _) => {
            return Err(ServerError::StopReused {
                operation: asked.operation,
            });
        }
        (Err(IdentityError::TransitionRefused { .. }), other) => {
            return Err(ServerError::AgentNotActive {
                state: other.to_string(),
            });
        }
        (Err(other), _) => return Err(other.into()),
    }
    // Asked only once the directory holds this operation's suspension, so a
    // refused stop leaves no asked line in other words behind it.
    if let Some(kept) = with_stops(&state, |store| store.ask(asked.clone()))? {
        return through_runners(&state, kept.into(), &by).await.map(Json);
    }
    let agent = agent.to_string();
    let certificates_withdrawn = withdraw_certificates(&state, &agent, &by, &reason)?;
    let sessions_asked = ask_sessions(&state, &agent, &asked.operation, &by, &reason)?;
    let (credentials_ended, credentials_refused) =
        match end_handles(&state, &headers, &agent, &asked.operation).await {
            Ok(ended) => (Some(ended), None),
            Err(refused) => (None, Some(refused.to_string())),
        };
    let done = Stop {
        certificates_withdrawn,
        sessions_asked,
        credentials_ended,
        credentials_refused,
        ..asked
    };
    // Kept before any runner is waited on, so a runner that never answers
    // leaves the stop recorded whole, and the same stop sent again asks it
    // again.
    let kept = with_stops(&state, |store| store.keep(done))?;
    through_runners(&state, kept.into(), &by).await.map(Json)
}

/// `view` once each asked session not yet confirmed ended has been asked of
/// its runner, all at once, with each confirmed end and each refusal.
async fn through_runners(
    state: &Arc<AppState>,
    view: StopView,
    by: &str,
) -> Result<StopView, ServerError> {
    let view = confirmed(state, view)?;
    let unconfirmed: Vec<String> = view
        .sessions_asked
        .iter()
        .filter(|session| {
            !view
                .sessions_confirmed
                .iter()
                .any(|end| &end.session == *session)
        })
        .cloned()
        .collect();
    let sessions_refused = end_sessions(state, unconfirmed, by).await;
    let mut view = confirmed(state, view)?;
    view.sessions_refused = sessions_refused;
    Ok(view)
}

/// Withdraw every certificate of the agent that stands, naming the stop.
fn withdraw_certificates(
    state: &AppState,
    agent: &str,
    by: &str,
    reason: &str,
) -> Result<Vec<String>, ServerError> {
    let Some(store) = state.certificates.as_ref() else {
        return Ok(Vec::new());
    };
    let mut store = store.lock().unwrap_or_else(PoisonError::into_inner);
    let reason = format!("emergency stop: {reason}");
    let mut serials: Vec<String> = store
        .certificates()
        .filter(|entered| entered.issued.agent == agent)
        .filter(|entered| {
            entered
                .withdrawn
                .as_ref()
                .is_none_or(|earlier| earlier.by == by && earlier.reason == reason)
        })
        .map(|entered| entered.issued.serial.clone())
        .collect();
    serials.sort();
    for serial in &serials {
        store.withdraw(Withdrawn {
            serial: serial.clone(),
            by: by.to_owned(),
            reason: reason.clone(),
            withdrawn_at: now(),
        })?;
    }
    Ok(serials)
}

/// Record, on every session of the agent not stopped, that its runtime is
/// asked to end it.
fn ask_sessions(
    state: &AppState,
    agent: &str,
    operation: &str,
    by: &str,
    reason: &str,
) -> Result<Vec<String>, ServerError> {
    if state.runtime.is_none() {
        return Ok(Vec::new());
    }
    let open: Vec<(String, String)> = with_runtime(state, |store| {
        Ok(store
            .sessions()
            .iter()
            .filter(|tracked| tracked.agent.as_deref() == Some(agent) && !tracked.stopped())
            .map(|tracked| (tracked.session.clone(), tracked.machine.clone()))
            .collect())
    })?;
    let mut asked = Vec::new();
    for (session, machine) in open {
        with_runtime(state, |store| {
            store.report(Report {
                operation: part(operation, "session", &session),
                session: session.clone(),
                agent: Some(agent.to_owned()),
                machine,
                state: Reported::StopAsked,
                what: format!("emergency stop: {reason}"),
                confirmation: String::new(),
                reported_by: by.to_owned(),
                at: now(),
                launch: None,
            })
        })?;
        asked.push(session);
    }
    Ok(asked)
}

/// End every handle the agent holds at the broker, each under an operation
/// id made from the stop's, so a stop cut off and sent again ends nothing
/// twice; a handle ended by this stop counts, one ended by anyone else does
/// not.
async fn end_handles(
    state: &AppState,
    headers: &HeaderMap,
    agent: &str,
    operation: &str,
) -> Result<Vec<String>, ServerError> {
    let path = format!("/_lys/handles?holder={agent}");
    let held = crate::secrets_api::ask(state, headers, Method::GET, &path, Bytes::new()).await?;
    let handles = held["handles"]
        .as_array()
        .ok_or_else(|| ServerError::SecretsUnavailable {
            reason: "the broker's handles answer lists no handles member".to_owned(),
        })?;
    let mut ended = Vec::new();
    for handle in handles {
        let Some(id) = handle["id"].as_str() else {
            return Err(ServerError::SecretsUnavailable {
                reason: "the broker listed a handle without an id".to_owned(),
            });
        };
        let body = json!({ "handle": id, "operation": part(operation, "handle", id) });
        let answer: Value = crate::secrets_api::ask(
            state,
            headers,
            Method::POST,
            "/_lys/drop",
            Bytes::from(body.to_string()),
        )
        .await?;
        if answer["handle"].as_str() != Some(id) {
            return Err(ServerError::SecretsUnavailable {
                reason: format!("the broker did not confirm ending handle {id}"),
            });
        }
        match answer["outcome"].as_str() {
            Some("ended" | "repeated") => ended.push(id.to_owned()),
            Some("already_ended") => {}
            other => {
                return Err(ServerError::SecretsUnavailable {
                    reason: format!("the broker answered handle {id} with outcome {other:?}"),
                });
            }
        }
    }
    Ok(ended)
}
