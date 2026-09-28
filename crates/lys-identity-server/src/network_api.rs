//! The network routes: the machines agents can run on, which agents may run
//! on each, and what an agent on each may reach.
//!
//! The administrator names a machine and retires it. Every signed-in
//! identity the directory knows may read them. A machine with no runtime
//! enforces nothing, so it takes no slots and no agent is placed on it.
//!
//! A machine's last report is the latest report any runtime made of a
//! session on it. When the configuration names no runtime reports, no
//! machine has one; the answer says so in `reports_served`, and never shows
//! a machine as reporting.

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
use crate::network_store::{Machine, NetworkStore, Retirement};
use crate::read_api::own_person;
use crate::read_views::AgentSummary;
use crate::routes::{AppState, signed_in, with_directory};
use crate::runtime_api::last_reports;
use crate::session::now;

/// The most characters a machine's name, kind or runtime carries.
const WORDS_MAX: usize = 100;
/// The most characters a host name carries.
const HOST_MAX: usize = 253;

/// One machine.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct MachineView {
    /// The machine's id, which is the operation id it was named with.
    pub id: String,
    /// Its name.
    pub name: String,
    /// What kind of machine it is.
    pub kind: String,
    /// The runtime installed on it, null for none.
    pub runtime: Option<String>,
    /// How many agents it runs at once.
    pub slots: u32,
    /// The agents that may run on it, as the directory holds them now.
    pub may_run: Vec<AgentSummary>,
    /// The hosts an agent on it may reach.
    pub may_reach: Vec<String>,
    /// The roles whose holders may run on it, by role id.
    pub may_run_roles: Vec<String>,
    /// The person who named it.
    pub named_by: String,
    /// When it was named, in seconds since the Unix epoch.
    pub named_at: u64,
    /// `in_use` or `retired`.
    pub state: &'static str,
    /// When it was retired, in seconds since the Unix epoch, null while in use.
    pub retired_at: Option<u64>,
    /// When a runtime last reported a session on it, null while none has.
    pub last_report_at: Option<u64>,
}

/// The answer of `GET /network`.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct NetworkView {
    /// Every machine, in the order named.
    pub machines: Vec<MachineView>,
    /// Whether this service keeps runtime reports. While it does not, no
    /// machine has a last report.
    pub reports_served: bool,
}

/// A machine to name. Every member is required; `runtime` may be null.
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct NameBody {
    operation: String,
    name: String,
    kind: String,
    runtime: Option<String>,
    slots: u32,
    may_run: Vec<String>,
    #[serde(default)]
    may_run_roles: Vec<String>,
    may_reach: Vec<String>,
}

/// The network routes.
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/network", get(list))
        .route("/network/machines", post(name))
        .route("/network/machines/{id}/retire", post(retire))
}

fn malformed(reason: impl Into<String>) -> ServerError {
    ServerError::RequestMalformed {
        reason: reason.into(),
    }
}

fn words(name: &str, text: &str) -> Result<String, ServerError> {
    let text = text.trim();
    if text.is_empty() {
        return Err(malformed(format!("{name} is empty")));
    }
    if text.chars().count() > WORDS_MAX {
        return Err(malformed(format!(
            "{name} is longer than {WORDS_MAX} characters"
        )));
    }
    Ok(text.to_owned())
}

fn host(text: &str) -> Result<String, ServerError> {
    let named = text.trim().to_ascii_lowercase();
    let plain = named
        .chars()
        .all(|letter| letter.is_ascii_alphanumeric() || letter == '.' || letter == '-');
    if named.is_empty() || named.len() > HOST_MAX || !plain {
        return Err(malformed(format!(
            "`{text}` is not a host name: letters, digits, dots and hyphens only, with no scheme, port or path"
        )));
    }
    Ok(named)
}

pub(crate) fn with_network<T>(
    state: &AppState,
    act: impl FnOnce(&mut NetworkStore) -> Result<T, ServerError>,
) -> Result<T, ServerError> {
    let store = state
        .network
        .as_ref()
        .ok_or_else(|| ServerError::NetworkUnavailable {
            reason: "the configuration names no network_file".to_owned(),
        })?;
    let mut store = store.lock().unwrap_or_else(PoisonError::into_inner);
    store.settle()?;
    act(&mut store)
}

fn view(directory: &Projection, machine: &Machine, last_report_at: Option<u64>) -> MachineView {
    let may_run = machine
        .may_run
        .iter()
        .filter_map(|id| {
            let agent = AgentId::from_str(id).ok()?;
            let record = directory.record(IdentityId::Agent(agent))?;
            Some(AgentSummary {
                id: id.clone(),
                display_name: record.profile().display_name().to_owned(),
                state: record.state().to_string(),
            })
        })
        .collect();
    MachineView {
        id: machine.id.clone(),
        name: machine.name.clone(),
        kind: machine.kind.clone(),
        runtime: machine.runtime.clone(),
        slots: machine.slots,
        may_run,
        may_reach: machine.may_reach.clone(),
        may_run_roles: machine.may_run_roles.clone(),
        named_by: machine.named_by.clone(),
        named_at: machine.named_at,
        state: if machine.retired.is_some() {
            "retired"
        } else {
            "in_use"
        },
        retired_at: machine.retired.as_ref().map(|retired| retired.at),
        last_report_at,
    }
}

async fn list(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<NetworkView>, ServerError> {
    with_directory(&state, |directory| {
        let directory = directory.projection()?;
        caller(&state, &headers, directory)?;
        let last = last_reports(&state)?;
        with_network(&state, |store| {
            Ok(Json(NetworkView {
                machines: store
                    .machines()
                    .iter()
                    .map(|machine| view(directory, machine, last.get(&machine.id).copied()))
                    .collect(),
                reports_served: state.runtime.is_some(),
            }))
        })
    })
}

/// The machine `body` names, checked against the directory.
fn named(
    directory: &Projection,
    body: &NameBody,
    (known_roles, by, at): (&[String], String, u64),
) -> Result<Machine, ServerError> {
    let runtime = body
        .runtime
        .as_deref()
        .map(|runtime| words("runtime", runtime))
        .transpose()?;
    if runtime.is_none()
        && (body.slots > 0 || !body.may_run.is_empty() || !body.may_run_roles.is_empty())
    {
        return Err(malformed(
            "a machine with no runtime enforces nothing: it takes no slots and no agent is placed on it",
        ));
    }
    let mut may_run = Vec::new();
    for id in &body.may_run {
        let agent = AgentId::from_str(id)?;
        if directory.record(IdentityId::Agent(agent)).is_none() {
            return Err(ServerError::AgentNotVisible);
        }
        let id = agent.to_string();
        if !may_run.contains(&id) {
            may_run.push(id);
        }
    }
    let mut may_run_roles = Vec::new();
    for role in &body.may_run_roles {
        let role = role.trim().to_owned();
        if !known_roles.contains(&role) {
            return Err(ServerError::RoleUnknown);
        }
        if !may_run_roles.contains(&role) {
            may_run_roles.push(role);
        }
    }
    let mut may_reach = Vec::new();
    for text in &body.may_reach {
        let host = host(text)?;
        if !may_reach.contains(&host) {
            may_reach.push(host);
        }
    }
    Ok(Machine {
        id: OperationId::from_str(&body.operation)?.to_string(),
        name: words("name", &body.name)?,
        kind: words("kind", &body.kind)?,
        runtime,
        slots: body.slots,
        may_run,
        may_run_roles,
        may_reach,
        named_by: by,
        named_at: at,
        retired: None,
    })
}

async fn name(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Result<Json<NameBody>, JsonRejection>,
) -> Result<Json<MachineView>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    state.admission.administrator(&actor)?;
    let Json(body) = body.map_err(|refused| malformed(refused.body_text()))?;
    with_directory(&state, |directory| {
        let directory = directory.projection()?;
        let by = own_person(directory, &actor)?.to_string();
        let known = crate::roles_api::role_ids(&state)?;
        let machine = named(directory, &body, (&known, by, now()))?;
        let last = last_reports(&state)?;
        with_network(&state, |store| {
            store.name(machine.clone())?;
            let kept = store
                .machine(&machine.id)
                .ok_or(ServerError::MachineUnknown)?;
            Ok(Json(view(directory, kept, last.get(&kept.id).copied())))
        })
    })
}

async fn retire(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<MachineView>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    state.admission.administrator(&actor)?;
    with_directory(&state, |directory| {
        let directory = directory.projection()?;
        let by = own_person(directory, &actor)?.to_string();
        let last = last_reports(&state)?;
        with_network(&state, |store| {
            store.retire(&id, Retirement { by, at: now() })?;
            let kept = store.machine(&id).ok_or(ServerError::MachineUnknown)?;
            Ok(Json(view(directory, kept, last.get(&kept.id).copied())))
        })
    })
}
