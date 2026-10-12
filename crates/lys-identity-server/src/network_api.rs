//! The network routes: the machines agents can run on, which agents may run
//! on each, and what an agent on each may reach.
//!
//! A person names their own computer; the administrator names and retires machines. Every signed-in
//! person reads only their owned, start-admitted or team machines. A machine with no runtime
//! enforces nothing, so it takes no slots and no agent is placed on it.
//!
//! A machine's last report is the latest report any runtime made of a
//! session on it. When the configuration names no runtime reports, no
//! machine has one; the answer says so in `reports_served`, and never shows
//! a machine as reporting.

use std::ops::Bound;

#[path = "network_api/access.rs"]
mod access;
#[path = "network_api/personal.rs"]
mod personal;
#[path = "network_api/writes.rs"]
mod writes;

pub(crate) use personal::computer_owner;
use std::str::FromStr;
use std::sync::Arc;

use axum::extract::State;
use axum::http::HeaderMap;
use axum::routing::{get, post};
use axum::{Json, Router};
use lys_identity::projection::Projection;
use lys_identity::{AgentId, IdentityId, OperationId};
use serde::{Deserialize, Serialize};

use crate::error::ServerError;
use crate::network_personal::PersonalNetwork;
use crate::network_store::{AgentsRecorded, Machine, TeamRecorded};
use crate::read_views::AgentSummary;
use crate::routes::{AppState, signed_in, with_directory};
use crate::runtime_api::last_reports;

/// The most characters a host name carries: DNS caps a name at 255 octets on
/// the wire (RFC 1035, section 2.3.4), which is 253 characters as text.
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
    /// The owning team, null while unowned.
    pub team: Option<String>,
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
    /// The person who owns this computer.
    pub owner: String,
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
    /// Admitted machines, in the order named.
    pub machines: Vec<MachineView>,
    /// Whether this service keeps runtime reports. While it does not, no
    /// machine has a last report.
    pub reports_served: bool,
    /// Counters and continuation, absent when no query was supplied.
    #[serde(flatten)]
    pub page: Option<crate::list_page::Totals>,
}

/// A machine to name; an omitted team leaves it unowned.
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct NameBody {
    operation: String,
    name: String,
    kind: String,
    runtime: Option<String>,
    #[serde(default)]
    team: Option<String>,
    slots: u32,
    may_run: Vec<String>,
    #[serde(default)]
    may_run_roles: Vec<String>,
    may_reach: Vec<String>,
}

/// An ownership assignment; an explicit null clears the team.
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct TeamBody {
    operation: String,
    #[serde(deserialize_with = "required_team")]
    #[schema(required = true)]
    team: Option<String>,
}

/// The current computer beside the original ownership receipt.
#[derive(Serialize, utoipa::ToSchema)]
pub struct MachineTeamChanged {
    /// The computer as it stands now.
    pub machine: MachineView,
    /// The act this operation first recorded.
    pub recorded: TeamRecorded,
}

/// One agent's allowance on a computer.
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct AgentBody {
    operation: String,
    agent: String,
    allow: bool,
}

/// The current computer beside the original agent allowance receipt.
#[derive(Serialize, utoipa::ToSchema)]
pub struct MachineAgentsChanged {
    /// The computer as it stands now.
    pub machine: MachineView,
    /// The act this operation first recorded.
    pub recorded: AgentsRecorded,
}

fn required_team<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<String>, D::Error> {
    Option::<String>::deserialize(deserializer)
}

/// The network routes.
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/network", get(list))
        .route("/network/machines", post(writes::name))
        .route("/network/machines/{id}/retire", post(writes::retire))
        .route("/network/machines/{id}/team", post(writes::assign_team))
        .route("/network/machines/{id}/agents", post(writes::change_agent))
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
    act: impl FnOnce(&mut access::NetworkAccess<'_>) -> Result<T, ServerError>,
) -> Result<T, ServerError> {
    let store = state
        .network
        .as_ref()
        .ok_or_else(|| ServerError::NetworkUnavailable {
            reason: "the configuration names no network_file".to_owned(),
        })?;
    let mut store = store
        .lock()
        .map_err(|error| ServerError::NetworkUnavailable {
            reason: format!("the network lock is poisoned: {error}"),
        })?;
    store.settle()?;
    act(&mut access::NetworkAccess::new(&mut store))
}

fn view(
    directory: &Projection,
    machine: &Machine,
    last_report_at: Option<u64>,
) -> Result<MachineView, ServerError> {
    let may_run = machine
        .may_run
        .iter()
        .map(|id| {
            let agent = AgentId::from_str(id)?;
            let identity = IdentityId::Agent(agent);
            let record = directory.record(identity).ok_or_else(|| {
                lys_identity::IdentityError::IdentityUnknown {
                    identity: id.clone(),
                }
            })?;
            crate::read_api::agent_summary(directory, identity, record)
        })
        .collect::<Result<_, ServerError>>()?;
    Ok(MachineView {
        id: machine.id.clone(),
        name: machine.name.clone(),
        kind: machine.kind.clone(),
        runtime: machine.runtime.clone(),
        team: machine.team.clone(),
        slots: machine.slots,
        may_run,
        may_reach: machine.may_reach.clone(),
        may_run_roles: machine.may_run_roles.clone(),
        named_by: machine.named_by.clone(),
        owner: machine.named_by.clone(),
        named_at: machine.named_at,
        state: if machine.retired.is_some() {
            "retired"
        } else {
            "in_use"
        },
        retired_at: machine.retired.as_ref().map(|retired| retired.at),
        last_report_at,
    })
}

async fn list(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    query: crate::list_page::Input,
) -> Result<Json<NetworkView>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    with_directory(&state, |directory| {
        let directory = directory.projection()?;
        crate::caller_admission::active_caller(directory, &actor)?;
        let scope = PersonalNetwork::read(&state, directory, &actor)?;
        let page = crate::list_page::Page::read(query, "/network")?;
        let teams = page
            .as_ref()
            .map(|page| page.teams(&state))
            .transpose()?
            .flatten();
        let last = last_reports(&state)?;
        with_network(&state, |store| {
            let (machines, totals) = if let Some(page) = &page {
                let filtered = page.filtered() || scope.is_some();
                let after = if filtered {
                    Bound::Unbounded
                } else {
                    page.after()
                };
                let (machines, totals) = page.select(
                    store.machines_ordered(after),
                    (!filtered).then_some(store.machines().len()),
                    |machine| {
                        Ok(scope.as_ref().is_none_or(|scope| scope.permits(machine))
                            && page.matches([machine.name.as_str()])
                            && teams.as_ref().is_none_or(|teams| {
                                machine
                                    .team
                                    .as_ref()
                                    .is_some_and(|team| teams.contains(team))
                            }))
                    },
                    |machine| &machine.id,
                    |machine| view(directory, machine, last.get(&machine.id).copied()),
                )?;
                (machines, Some(totals))
            } else {
                (
                    store
                        .machines()
                        .iter()
                        .filter(|machine| scope.as_ref().is_none_or(|scope| scope.permits(machine)))
                        .map(|machine| view(directory, machine, last.get(&machine.id).copied()))
                        .collect::<Result<_, _>>()?,
                    None,
                )
            };
            Ok(Json(NetworkView {
                machines,
                reports_served: state.runtime.is_some(),
                page: totals,
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
        team: body.team.clone(),
        creation_team: body.team.clone(),
        slots: body.slots,
        may_run,
        may_run_roles,
        may_reach,
        named_by: by,
        named_at: at,
        retired: None,
    })
}

#[cfg(test)]
#[path = "network_api/tests.rs"]
mod tests;
