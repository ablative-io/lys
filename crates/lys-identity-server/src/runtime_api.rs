//! The runtime routes: a started session reports starting, running and
//! stopped, and the sessions screen reads what each runtime has reported.
//!
//! An agent's session is reported by the agent itself, as the directory
//! resolves the signed-in caller, or by the person responsible for the agent
//! or the administrator, who already answer for it; no other identity
//! reports in its place, and each report is kept under who delivered it. The
//! directory binds logins only to people, so an agent reports for itself
//! only once it has a credential this service accepts. A session a runtime sees that carries no identity is
//! found: any identity the directory knows may report one, it is kept under
//! who reported it, and it is never given an identity here.
//!
//! A session is shown stopped only when its runtime confirmed the stop, in
//! the runtime's own words. An agent's session whose runtime has reported
//! nothing since `starting` is shown `unconfirmed`, however long ago that
//! was: nothing is inferred from the clock.

use std::collections::BTreeMap;
use std::str::FromStr;
use std::sync::{Arc, PoisonError};

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::routing::{get, post};
use axum::{Json, Router};
use lys_identity::projection::Projection;
use lys_identity::{AgentId, IdentityId, OperationId};
use serde::{Deserialize, Serialize};

use crate::error::ServerError;
use crate::grants::caller;
use crate::launch_api::placed;
use crate::network_api::with_network;
use crate::routes::{AppState, signed_in, with_directory};
use crate::runtime_state::{Report, Reported, Tracked};
use crate::runtime_store::RuntimeStore;
use crate::session::now;

/// The most characters a runtime's words carry.
const WORDS_MAX: usize = 500;

/// A confirmed stop.
#[derive(Debug, Clone, Serialize)]
pub struct StopView {
    /// When the stop was reported, in seconds since the Unix epoch.
    pub at: u64,
    /// The runtime's confirmation, in its words.
    pub confirmation: String,
}

/// One session as the runtimes have reported it.
#[derive(Debug, Clone, Serialize)]
pub struct SessionView {
    /// The session.
    pub session: String,
    /// The agent it is held under, null for a found session.
    pub agent: Option<String>,
    /// The machine it runs on.
    pub machine: String,
    /// The machine's name, null when the machines are not kept here.
    pub machine_name: Option<String>,
    /// The runtime on that machine, null when none is kept.
    pub runtime: Option<String>,
    /// How it is shown: `unconfirmed`, `running` or `stopped`.
    pub shown: &'static str,
    /// The state last reported.
    pub last_reported: &'static str,
    /// When its first report was received, in seconds since the Unix epoch.
    pub first_report_at: u64,
    /// When its latest report was received, in seconds since the Unix epoch.
    pub last_report_at: u64,
    /// What the runtime last said it saw; empty when it said nothing.
    pub what: String,
    /// Its confirmed stop, null until the runtime confirms one.
    pub stopped: Option<StopView>,
    /// The identity that delivered its first report.
    pub reported_by: String,
}

/// Sessions, in the order first reported.
#[derive(Debug, Clone, Serialize)]
pub struct SessionsView {
    /// The sessions.
    pub sessions: Vec<SessionView>,
}

/// A report as a runtime sends it.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReportBody {
    operation: String,
    state: Reported,
    machine: String,
    #[serde(default)]
    what: String,
    #[serde(default)]
    confirmation: String,
}

/// The runtime routes.
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/agents/{id}/runtime/sessions/{session}/reports",
            post(report_agent),
        )
        .route("/agents/{id}/runtime/sessions", get(agent_sessions))
        .route("/runtime/sessions", get(sessions))
        .route("/runtime/found/{session}/reports", post(report_found))
        .route("/runtime/found", get(found))
}

fn malformed(reason: impl Into<String>) -> ServerError {
    ServerError::RequestMalformed {
        reason: reason.into(),
    }
}

pub(crate) fn with_runtime<T>(
    state: &AppState,
    act: impl FnOnce(&mut RuntimeStore) -> Result<T, ServerError>,
) -> Result<T, ServerError> {
    let store = state
        .runtime
        .as_ref()
        .ok_or_else(|| ServerError::RuntimeUnavailable {
            reason: "the configuration names no runtime_dir".to_owned(),
        })?;
    let mut store = store.lock().unwrap_or_else(PoisonError::into_inner);
    store.settle()?;
    act(&mut store)
}

/// When a runtime last reported a session on each machine, by machine id;
/// none when the configuration names no runtime reports.
pub(crate) fn last_reports(state: &AppState) -> Result<BTreeMap<String, u64>, ServerError> {
    let mut last = BTreeMap::new();
    if state.runtime.is_none() {
        return Ok(last);
    }
    with_runtime(state, |store| {
        for tracked in store.sessions() {
            if let Some(report) = tracked.latest() {
                let at = last.entry(tracked.machine.clone()).or_insert(report.at);
                *at = (*at).max(report.at);
            }
        }
        Ok(())
    })?;
    Ok(last)
}

fn words(name: &str, text: &str) -> Result<String, ServerError> {
    let text = text.trim();
    if text.chars().count() > WORDS_MAX {
        return Err(malformed(format!(
            "{name} is longer than {WORDS_MAX} characters"
        )));
    }
    Ok(text.to_owned())
}

/// The report `body` makes on `session`, checked, for `agent` or found.
fn report(
    body: ReportBody,
    session: &str,
    agent: Option<String>,
    reported_by: String,
) -> Result<Report, ServerError> {
    let session = OperationId::from_str(session)
        .map_err(|_unread| malformed("a session is named `op-` and 32 hex digits"))?;
    let confirmation = words("confirmation", &body.confirmation)?;
    match body.state {
        Reported::Stopped if confirmation.is_empty() => {
            return Err(malformed(
                "a stop is kept only with the runtime's confirmation",
            ));
        }
        Reported::Starting | Reported::Running if !confirmation.is_empty() => {
            return Err(malformed("only a stop carries a confirmation"));
        }
        _ => {}
    }
    Ok(Report {
        operation: OperationId::from_str(&body.operation)?.to_string(),
        session: session.to_string(),
        agent,
        machine: body.machine,
        state: body.state,
        what: words("what", &body.what)?,
        confirmation,
        reported_by,
        at: now(),
        launch: None,
    })
}

fn view(state: &AppState, tracked: &Tracked) -> Option<SessionView> {
    let (first, latest) = (tracked.first()?, tracked.latest()?);
    let machine = state.network.as_ref().and_then(|store| {
        let store = store.lock().unwrap_or_else(PoisonError::into_inner);
        store.machine(&tracked.machine).cloned()
    });
    Some(SessionView {
        session: tracked.session.clone(),
        agent: tracked.agent.clone(),
        machine: tracked.machine.clone(),
        machine_name: machine.as_ref().map(|machine| machine.name.clone()),
        runtime: machine.and_then(|machine| machine.runtime),
        shown: tracked.shown(),
        last_reported: latest.state.name(),
        first_report_at: first.at,
        last_report_at: latest.at,
        what: tracked
            .reports
            .iter()
            .rev()
            .find(|report| !report.what.is_empty())
            .map(|report| report.what.clone())
            .unwrap_or_default(),
        stopped: (latest.state == Reported::Stopped).then(|| StopView {
            at: latest.at,
            confirmation: latest.confirmation.clone(),
        }),
        reported_by: first.reported_by.clone(),
    })
}

/// Whether `asker` may see the sessions of `agent`: the administrator, the
/// person responsible for it, or the agent itself.
fn sees(directory: &Projection, administrator: bool, asker: IdentityId, agent: &str) -> bool {
    if administrator {
        return true;
    }
    let Ok(agent) = AgentId::from_str(agent) else {
        return false;
    };
    match asker {
        IdentityId::Agent(own) => own == agent,
        IdentityId::Person(person) => directory
            .record(IdentityId::Agent(agent))
            .is_some_and(|record| record.responsible() == Some(person)),
    }
}

async fn report_agent(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((id, session)): Path<(String, String)>,
    body: Result<Json<ReportBody>, JsonRejection>,
) -> Result<Json<SessionView>, ServerError> {
    let Json(body) = body.map_err(|refused| malformed(refused.body_text()))?;
    let agent = AgentId::from_str(&id).map_err(|_unread| ServerError::AgentNotVisible)?;
    with_directory(&state, |directory| {
        let directory = directory.projection()?;
        let asker = caller(&state, &headers, directory)?;
        let Some(record) = directory.record(IdentityId::Agent(agent)) else {
            return Err(ServerError::AgentNotVisible);
        };
        let answers = match asker {
            IdentityId::Agent(own) => own == agent,
            IdentityId::Person(person) => {
                record.responsible() == Some(person)
                    || signed_in(&state, &headers)
                        .is_ok_and(|actor| state.admission.administrator(&actor).is_ok())
            }
        };
        if !answers {
            return Err(ServerError::NotAdmitted {
                reason: "only the agent, the person responsible for it or the administrator reports its sessions",
            });
        }
        let agent = agent.to_string();
        let report = report(body, &session, Some(agent.clone()), asker.to_string())?;
        if report.state == Reported::Starting {
            with_network(&state, |store| placed(store, &report.machine, &agent).map(drop))?;
        }
        let tracked = with_runtime(&state, |store| store.report(report))?;
        view(&state, &tracked).ok_or(ServerError::RuntimeSessionUnknown)
    })
    .map(Json)
}

async fn report_found(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(session): Path<String>,
    body: Result<Json<ReportBody>, JsonRejection>,
) -> Result<Json<SessionView>, ServerError> {
    let Json(body) = body.map_err(|refused| malformed(refused.body_text()))?;
    if body.state == Reported::Starting {
        return Err(malformed(
            "a found session is reported running or stopped, never starting",
        ));
    }
    with_directory(&state, |directory| {
        let directory = directory.projection()?;
        let asker = caller(&state, &headers, directory)?;
        let report = report(body, &session, None, asker.to_string())?;
        with_network(&state, |store| {
            store
                .machine(&report.machine)
                .map(drop)
                .ok_or(ServerError::MachineUnknown)
        })?;
        let tracked = with_runtime(&state, |store| store.report(report))?;
        view(&state, &tracked).ok_or(ServerError::RuntimeSessionUnknown)
    })
    .map(Json)
}

async fn agent_sessions(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<SessionsView>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    let agent = AgentId::from_str(&id).map_err(|_unread| ServerError::AgentNotVisible)?;
    with_directory(&state, |directory| {
        let directory = directory.projection()?;
        let asker = caller(&state, &headers, directory)?;
        let administrator = state.admission.administrator(&actor).is_ok();
        if directory.record(IdentityId::Agent(agent)).is_none() {
            return Err(ServerError::AgentNotVisible);
        }
        let agent = agent.to_string();
        if !sees(directory, administrator, asker, &agent) {
            return Err(ServerError::AgentNotVisible);
        }
        with_runtime(&state, |store| {
            Ok(SessionsView {
                sessions: store
                    .sessions()
                    .iter()
                    .filter(|tracked| tracked.agent.as_deref() == Some(agent.as_str()))
                    .filter_map(|tracked| view(&state, tracked))
                    .collect(),
            })
        })
    })
    .map(Json)
}

async fn sessions(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<SessionsView>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    with_directory(&state, |directory| {
        let directory = directory.projection()?;
        let asker = caller(&state, &headers, directory)?;
        let administrator = state.admission.administrator(&actor).is_ok();
        with_runtime(&state, |store| {
            Ok(SessionsView {
                sessions: store
                    .sessions()
                    .iter()
                    .filter(|tracked| {
                        tracked
                            .agent
                            .as_deref()
                            .is_some_and(|agent| sees(directory, administrator, asker, agent))
                    })
                    .filter_map(|tracked| view(&state, tracked))
                    .collect(),
            })
        })
    })
    .map(Json)
}

async fn found(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<SessionsView>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    state.admission.administrator(&actor)?;
    with_runtime(&state, |store| {
        Ok(SessionsView {
            sessions: store
                .sessions()
                .iter()
                .filter(|tracked| tracked.agent.is_none())
                .filter_map(|tracked| view(&state, tracked))
                .collect(),
        })
    })
    .map(Json)
}
