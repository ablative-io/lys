//! The emergency stop of an agent: one act by the person responsible for
//! it or the administrator that suspends the agent, withdraws every
//! certificate it holds, ends every credential handle it holds at the
//! secrets broker, and asks the runtime of every session of its to end it.
//!
//! The stop never claims a session ended. Each session shows unconfirmed
//! from the ask until its runtime reports it stopped. Sent again under the
//! same operation id it is kept once and answers the same; each part is
//! kept once by its own record.
//!
//! The broker is asked after the suspension, the certificates and the
//! sessions are recorded, so a broker that cannot be reached leaves those
//! in force; its refusal is named in the answer, never hidden.

use std::str::FromStr;
use std::sync::{Arc, PoisonError};

use axum::body::Bytes;
use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, Method};
use axum::routing::post;
use axum::{Json, Router};
use lys_identity::{AgentId, IdentityId, LifecycleState, OperationId, Transition};
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

/// The most characters a reason carries.
const REASON_MAX: usize = 500;

/// A stop to make.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StopBody {
    operation: String,
    reason: String,
}

/// The answer of the stop route: what was done, and what was only asked.
#[derive(Debug, Clone, Serialize)]
pub struct StopView {
    /// The agent stopped.
    pub agent: String,
    /// The operation id the stop was made under.
    pub operation: String,
    /// The agent's lifecycle state after the stop: `suspended`.
    pub state: String,
    /// The serials of the certificates withdrawn, in the log's order.
    pub certificates_withdrawn: Vec<String>,
    /// The credential handles ended at the broker, by id; null when the
    /// broker refused, and then `credentials_refused` names why.
    pub credentials_ended: Option<Vec<String>>,
    /// The broker's refusal, by name, when the handles could not be ended.
    pub credentials_refused: Option<String>,
    /// The sessions whose runtimes were asked to end them. Each stays
    /// unconfirmed until its runtime reports it stopped.
    pub sessions_asked: Vec<String>,
    /// Why, in the stopper's words.
    pub reason: String,
}

/// The stop route.
pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/agents/{id}/stop", post(stop))
}

fn malformed(reason: impl Into<String>) -> ServerError {
    ServerError::RequestMalformed {
        reason: reason.into(),
    }
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
    let actor = signed_in(&state, &headers)?;
    let agent = AgentId::from_str(&id).map_err(|_unread| ServerError::AgentNotVisible)?;
    let administrator = state.admission.administrator(&actor).is_ok();
    let by = with_directory(&state, |directory| {
        let projection = directory.projection()?;
        let person = own_person(projection, &actor)?;
        let record = projection
            .record(IdentityId::Agent(agent))
            .ok_or(ServerError::AgentNotVisible)?;
        if !administrator && record.responsible() != Some(person) {
            return Err(ServerError::AgentNotVisible);
        }
        match record.state() {
            LifecycleState::Active => {
                directory.transition(
                    actor.clone(),
                    operation,
                    IdentityId::Agent(agent),
                    Transition::Suspend,
                    &reason,
                    now(),
                )?;
            }
            LifecycleState::Suspended => {}
            other => {
                return Err(ServerError::AgentNotActive {
                    state: other.to_string(),
                });
            }
        }
        Ok(person.to_string())
    })?;
    let agent = agent.to_string();
    let certificates_withdrawn = withdraw_certificates(&state, &agent, &by, &reason)?;
    let sessions_asked = ask_sessions(&state, &agent, &operation.to_string(), &by, &reason)?;
    let (credentials_ended, credentials_refused) =
        match end_handles(&state, &headers, &agent, &operation.to_string()).await {
            Ok(ended) => (Some(ended), None),
            Err(refused) => (None, Some(refused.to_string())),
        };
    Ok(Json(StopView {
        agent,
        operation: operation.to_string(),
        state: "suspended".to_owned(),
        certificates_withdrawn,
        credentials_ended,
        credentials_refused,
        sessions_asked,
        reason,
    }))
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
/// id made from the stop's, so the same stop sent again ends nothing twice.
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
        if handle["dropped"].as_bool() == Some(true) {
            continue;
        }
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
        ended.push(id.to_owned());
    }
    Ok(ended)
}
