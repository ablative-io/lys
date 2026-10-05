//! The runtime routes: a started session reports starting, running and
//! stopped, and the sessions screen reads what each runtime has reported.
//!
//! An agent's session is reported by the agent itself, as the directory
//! resolves the signed-in caller, or by the person responsible for the agent
//! or the administrator, who already answer for it; no other identity
//! reports in its place, and each report is kept under who delivered it. The
//! directory binds logins only to people, so an agent reports for itself by
//! signing the request with the key its certificate names, as
//! `agent_signature` checks. A session a runtime sees that carries no identity is
//! found: any identity the directory knows may report one, it is kept under
//! who reported it, and it is never given an identity here.
//!
//! A session is shown stopped only when its runtime confirmed the stop, in
//! the runtime's own words. An agent's session whose runtime has reported
//! nothing since `starting` is shown `unconfirmed`, however long ago that
//! was: nothing is inferred from the clock.

use std::collections::{BTreeMap, BTreeSet};
use std::str::FromStr;
use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, Uri};
use axum::routing::{get, post};
use axum::{Json, Router};
use lys_identity::projection::Projection;
use lys_identity::{AgentId, IdentityId, OperationId};
use serde::{Deserialize, Serialize};

use crate::agent_signature::signed_agent;
use crate::error::ServerError;
use crate::grants::caller;
use crate::launch_api::placed;
use crate::network_api::with_network;
use crate::routes::{AppState, signed_in, with_directory};
use crate::runtime_state::{Report, Reported, Tracked};
use crate::runtime_store::{RuntimeStore, SessionActivity};
use crate::session::now;

/// A confirmed stop.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[schema(as = RuntimeStopView)]
pub struct StopView {
    /// When the stop was reported, in seconds since the Unix epoch.
    pub at: u64,
    /// The runtime's confirmation, in its words.
    pub confirmation: String,
}

/// One session as the runtimes have reported it.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[schema(as = RuntimeSessionView)]
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
    /// When this service asked the runtime to end it, null when it never did.
    pub stop_asked_at: Option<u64>,
    /// The identity that delivered its first report.
    pub reported_by: String,
}

/// Sessions, in the order first reported.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[schema(as = RuntimeSessionsView)]
pub struct SessionsView {
    /// The sessions.
    pub sessions: Vec<SessionView>,
}

/// A report as a runtime sends it.
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReportBody {
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
    let mut store = store
        .lock()
        .map_err(|error| ServerError::RuntimeUnavailable {
            reason: format!("the runtime lock is poisoned: {error}"),
        })?;
    store.settle()?;
    act(&mut store)
}

/// Selected agents' session activity; absent when no runtime history is configured.
pub(crate) fn session_agents(
    state: &AppState,
    selected: &BTreeSet<String>,
) -> Result<Option<BTreeMap<String, SessionActivity>>, ServerError> {
    if state.runtime.is_none() {
        return Ok(None);
    }
    with_runtime(state, |store| store.agents_with_sessions(selected)).map(Some)
}

/// When a runtime last reported a session on each machine, by machine id;
/// none when the configuration names no runtime reports.
pub(crate) fn last_reports(state: &AppState) -> Result<Arc<BTreeMap<String, u64>>, ServerError> {
    if state.runtime.is_none() {
        return Ok(Arc::new(BTreeMap::new()));
    }
    with_runtime(state, |store| Ok(store.last_reports()))
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
    let confirmation = body.confirmation.trim().to_owned();
    match body.state {
        Reported::StopAsked => {
            return Err(malformed(
                "a stop is asked through the agent's emergency stop, never reported",
            ));
        }
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
        what: body.what.trim().to_owned(),
        confirmation,
        reported_by,
        at: now(),
        launch: None,
    })
}

pub(crate) fn view(
    state: &AppState,
    tracked: &Tracked,
) -> Result<Option<SessionView>, ServerError> {
    let (Some(first), Some(latest)) = (tracked.first(), tracked.latest()) else {
        return Ok(None);
    };
    let machine = match &state.network {
        Some(store) => {
            let store = store
                .lock()
                .map_err(|error| ServerError::NetworkUnavailable {
                    reason: format!("the network lock is poisoned: {error}"),
                })?;
            store.machine(&tracked.machine).cloned()
        }
        None => None,
    };
    Ok(Some(SessionView {
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
        stop_asked_at: tracked.stop_asked_at(),
        reported_by: first.reported_by.clone(),
    }))
}

pub(crate) fn views<'a>(
    state: &AppState,
    mut tracked: impl Iterator<Item = &'a Tracked>,
) -> Result<Vec<SessionView>, ServerError> {
    tracked.try_fold(Vec::new(), |mut answers, tracked| {
        if let Some(answer) = view(state, tracked)? {
            answers.push(answer);
        }
        Ok(answers)
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
        IdentityId::ServiceAccount(_) | IdentityId::Connector(_) => false,
        IdentityId::Person(person) => directory
            .record(IdentityId::Agent(agent))
            .is_some_and(|record| record.responsible() == Some(person)),
    }
}

async fn report_agent(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((id, session)): Path<(String, String)>,
    uri: Uri,
    bytes: Bytes,
) -> Result<Json<SessionView>, ServerError> {
    let body: ReportBody = crate::signed_json::read(&state, &headers, &bytes)?;
    let agent = AgentId::from_str(&id).map_err(|_unread| ServerError::AgentNotVisible)?;
    with_directory(&state, |directory| {
        let directory = directory.projection()?;
        let target = uri.path_and_query().map_or(uri.path(), |target| target.as_str());
        let signed = signed_agent(&state, directory, &headers, ("POST", target, &bytes))?;
        let asker = match signed {
            Some(own) => IdentityId::Agent(own),
            None => caller(&state, &headers, directory)?,
        };
        let Some(record) = directory.record(IdentityId::Agent(agent)) else {
            return Err(ServerError::AgentNotVisible);
        };
        let answers = match asker {
            IdentityId::Agent(own) => own == agent,
                IdentityId::ServiceAccount(_) | IdentityId::Connector(_) => false,
            IdentityId::Person(person) => {
                record.responsible() == Some(person)
                    || state.admission.is_administrator(directory, &signed_in(&state, &headers)?)?
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
            let held = crate::roles_api::held_roles(&state, &agent, now())?;
            with_network(&state, |store| {
                placed(store, &report.machine, (&agent, &held)).map(drop)
            })?;
        }
        let tracked = with_runtime(&state, |store| store.report(report))?;
        if tracked.stopped() { crate::agent_pass::end_session(&state, &tracked.session)?; }
        if tracked.stopped() && let Some(agent) = &tracked.agent {
            crate::budgets_context::finish(&state, agent, &tracked.session)?;
        }
        view(&state, &tracked)?.ok_or(ServerError::RuntimeSessionUnknown)
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
        if tracked.stopped()
            && let Some(agent) = &tracked.agent
        {
            crate::budgets_context::finish(&state, agent, &tracked.session)?;
        }
        view(&state, &tracked)?.ok_or(ServerError::RuntimeSessionUnknown)
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
        let administrator = state.admission.is_administrator(directory, &actor)?;
        if directory.record(IdentityId::Agent(agent)).is_none() {
            return Err(ServerError::AgentNotVisible);
        }
        let agent = agent.to_string();
        if !sees(directory, administrator, asker, &agent) {
            return Err(ServerError::AgentNotVisible);
        }
        with_runtime(&state, |store| {
            Ok(SessionsView {
                sessions: views(
                    &state,
                    store
                        .sessions()
                        .iter()
                        .filter(|tracked| tracked.agent.as_deref() == Some(agent.as_str())),
                )?,
            })
        })
    })
    .map(Json)
}

/// Every agent's session the signed-in caller may see, in the order first
/// reported.
pub(crate) fn visible_sessions(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<Vec<Tracked>, ServerError> {
    let actor = signed_in(state, headers)?;
    with_directory(state, |directory| {
        let directory = directory.projection()?;
        let asker = caller(state, headers, directory)?;
        let administrator = state.admission.is_administrator(directory, &actor)?;
        with_runtime(state, |store| {
            Ok(store
                .sessions()
                .iter()
                .filter(|tracked| {
                    tracked
                        .agent
                        .as_deref()
                        .is_some_and(|agent| sees(directory, administrator, asker, agent))
                })
                .cloned()
                .collect())
        })
    })
}

/// A queried live list builds only the selected views; visibility still narrows metadata.
pub(crate) fn live_page(
    state: &AppState,
    headers: &HeaderMap,
    page: &crate::list_page::Page,
) -> Result<(Vec<SessionView>, crate::list_page::Totals), ServerError> {
    let actor = signed_in(state, headers)?;
    let members = page.members(state)?;
    with_directory(state, |directory| {
        let projection = directory.projection()?;
        let asker = caller(state, headers, projection)?;
        let administrator = state.admission.is_administrator(projection, &actor)?;
        let filtered = page.filtered() || !administrator;
        let after = if filtered {
            std::ops::Bound::Unbounded
        } else {
            page.after()
        };
        with_runtime(state, |store| {
            page.select(
                store.live_ordered(after),
                (!filtered).then_some(store.live_count()),
                |tracked| {
                    if !filtered {
                        return Ok(true);
                    }
                    let agent = tracked
                        .agent
                        .as_deref()
                        .ok_or(ServerError::RuntimeSessionUnknown)?;
                    if !sees(projection, administrator, asker, agent) {
                        return Ok(false);
                    }
                    let id = agent.parse::<AgentId>()?;
                    let record = projection
                        .record(IdentityId::Agent(id))
                        .ok_or(ServerError::AgentNotVisible)?;
                    let person = record.responsible().map(|person| person.to_string());
                    Ok(
                        crate::list_page::member(members.as_ref(), agent, person.as_deref())
                            && page.matches([record.profile().display_name()]),
                    )
                },
                |tracked| &tracked.session,
                |tracked| {
                    let agent = tracked
                        .agent
                        .as_deref()
                        .ok_or(ServerError::RuntimeSessionUnknown)?;
                    let id = agent.parse::<AgentId>()?;
                    if projection.record(IdentityId::Agent(id)).is_none() {
                        return Err(ServerError::AgentNotVisible);
                    }
                    view(state, tracked)?.ok_or(ServerError::RuntimeSessionUnknown)
                },
            )
        })
    })
}

async fn sessions(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<SessionsView>, ServerError> {
    let sessions = views(&state, visible_sessions(&state, &headers)?.iter())?;
    Ok(Json(SessionsView { sessions }))
}

async fn found(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<SessionsView>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    crate::routes::administrator(&state, &actor)?;
    with_runtime(&state, |store| {
        Ok(SessionsView {
            sessions: views(
                &state,
                store
                    .sessions()
                    .iter()
                    .filter(|tracked| tracked.agent.is_none()),
            )?,
        })
    })
    .map(Json)
}
