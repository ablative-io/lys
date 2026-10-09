//! The network routes: the machines agents can run on, which agents may run
//! on each, and what an agent on each may reach.
//!
//! The administrator names a machine and retires it. Every signed-in
//! person reads only their owned, start-admitted or team machines. A machine with no runtime
//! enforces nothing, so it takes no slots and no agent is placed on it.
//!
//! A machine's last report is the latest report any runtime made of a
//! session on it. When the configuration names no runtime reports, no
//! machine has one; the answer says so in `reports_served`, and never shows
//! a machine as reporting.

use std::collections::BTreeSet;
use std::ops::Bound;
use std::str::FromStr;
use std::sync::Arc;

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::routing::{get, post};
use axum::{Json, Router};
use lys_identity::projection::Projection;
use lys_identity::{Actor, AgentId, IdentityId, LifecycleState, OperationId};
use serde::{Deserialize, Serialize};

use crate::error::ServerError;
use crate::error_team::TeamError;
use crate::network_store::{AgentsRecorded, Machine, NetworkStore, Retirement, TeamRecorded};
use crate::read_api::own_person;
use crate::read_views::AgentSummary;
use crate::routes::{AppState, signed_in, with_directory};
use crate::runtime_api::last_reports;
use crate::session::now;
use crate::teams_api::{team_id, with_teams};
use crate::teams_state::Team;

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
        .route("/network/machines", post(name))
        .route("/network/machines/{id}/retire", post(retire))
        .route("/network/machines/{id}/team", post(assign_team))
        .route("/network/machines/{id}/agents", post(change_agent))
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
    act: impl FnOnce(&mut NetworkStore) -> Result<T, ServerError>,
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
    act(&mut store)
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

struct PersonalNetwork {
    person: String,
    teams: BTreeSet<String>,
    agents: BTreeSet<String>,
    roles: BTreeSet<String>,
}

impl PersonalNetwork {
    fn read(
        state: &AppState,
        directory: &Projection,
        actor: &Actor,
    ) -> Result<Option<Self>, ServerError> {
        if state.admission.is_administrator(directory, actor)? {
            return Ok(None);
        }
        let person = own_person(directory, actor)?;
        let agents = directory
            .agents_of(person)
            .filter_map(|entry| match entry {
                Ok((id, record)) if record.state() == LifecycleState::Active => {
                    Some(Ok(id.to_string()))
                }
                Ok(_) => None,
                Err(error) => Some(Err(ServerError::from(error))),
            })
            .collect::<Result<BTreeSet<_>, _>>()?;
        let person = person.to_string();
        let teams = if state.teams.is_some() {
            with_teams(state, |store| {
                Ok(store
                    .teams_iter()
                    .filter(|team| {
                        team.created.owner == person
                            || (team.members.contains(&person)
                                && !team.held.iter().any(|held| held.member == person))
                    })
                    .map(|team| team.created.id.clone())
                    .collect())
            })?
        } else {
            BTreeSet::new()
        };
        let roles = if let Some(store) = &state.roles {
            let mut store = store
                .lock()
                .map_err(|error| ServerError::RolesUnavailable {
                    reason: format!("the roles lock is poisoned: {error}"),
                })?;
            store.settle()?;
            let at = now();
            store
                .roles()
                .iter()
                .filter(|role| {
                    agents.iter().any(|agent| {
                        role.holding(agent)
                            .is_some_and(|holding| holding.state(at) == "holding")
                    })
                })
                .map(|role| role.id.clone())
                .collect()
        } else {
            BTreeSet::new()
        };
        Ok(Some(Self {
            person,
            teams,
            agents,
            roles,
        }))
    }

    fn permits(&self, machine: &Machine) -> bool {
        machine.named_by == self.person
            || machine
                .team
                .as_ref()
                .is_some_and(|team| self.teams.contains(team))
            || (machine.retired.is_none()
                && machine.runtime.is_some()
                && (machine
                    .may_run
                    .iter()
                    .any(|agent| self.agents.contains(agent))
                    || machine
                        .may_run_roles
                        .iter()
                        .any(|role| self.roles.contains(role))))
    }
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

async fn name(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Result<Json<NameBody>, JsonRejection>,
) -> Result<Json<MachineView>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    crate::routes::administrator(&state, &actor)?;
    let Json(mut body) = body.map_err(|refused| malformed(refused.body_text()))?;
    body.team = body.team.as_deref().map(team_id).transpose()?;
    with_directory(&state, |directory| {
        let directory = directory.projection()?;
        let by = own_person(directory, &actor)?.to_string();
        let known = crate::roles_api::role_ids(&state)?;
        let machine = named(directory, &body, (&known, by, now()))?;
        let team = owning_team(&state, body.team.as_deref())?;
        let last = last_reports(&state)?;
        with_network(&state, |store| {
            if store.machine(&machine.id).is_none() {
                active_team(team.as_ref())?;
            }
            store.name(machine.clone())?;
            let kept = store
                .machine(&machine.id)
                .ok_or(ServerError::MachineUnknown)?;
            Ok(Json(view(directory, kept, last.get(&kept.id).copied())?))
        })
    })
}

fn owning_team(state: &AppState, id: Option<&str>) -> Result<Option<Team>, ServerError> {
    id.map(|id| {
        with_teams(state, |store| {
            store
                .team(id)
                .cloned()
                .ok_or(ServerError::Team(TeamError::Unknown))
        })
    })
    .transpose()
}

fn active_team(team: Option<&Team>) -> Result<(), ServerError> {
    if let Some(team) = team.filter(|team| team.retired.is_some()) {
        return Err(ServerError::Team(TeamError::Retired {
            team: team.created.id.clone(),
        }));
    }
    Ok(())
}

async fn assign_team(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    body: Result<Json<TeamBody>, JsonRejection>,
) -> Result<Json<MachineTeamChanged>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    let administrator = crate::routes::is_administrator(&state, &actor)?;
    let Json(body) = body.map_err(|refused| malformed(refused.body_text()))?;
    let id = OperationId::from_str(&id)
        .map_err(|error| malformed(format!("computer id does not read: {error}")))?
        .to_string();
    let operation = OperationId::from_str(&body.operation)
        .map_err(|error| malformed(format!("operation does not read: {error}")))?
        .to_string();
    let team = body.team.as_deref().map(team_id).transpose()?;
    with_directory(&state, |directory| {
        let directory = directory.projection()?;
        let by = own_person(directory, &actor)?.to_string();
        let owning = owning_team(&state, team.as_deref())?;
        let last = last_reports(&state)?;
        with_network(&state, |store| {
            let machine = store.machine(&id).ok_or(ServerError::MachineUnknown)?;
            let first = store.team_recorded(&operation);
            let repeating = first
                .is_some_and(|first| first.machine == id && first.team == team && first.by == by);
            let claiming = machine.team.is_none()
                && owning.as_ref().is_some_and(|team| team.created.owner == by);
            if !administrator && !repeating && !claiming {
                return Err(ServerError::NotAdmitted {
                    reason: "only the directory administrator changes computer ownership; a team's owner may claim an unowned computer for their own team",
                });
            }
            if first.is_none() {
                active_team(owning.as_ref())?;
            }
            let recorded = store.assign_team(TeamRecorded {
                operation,
                machine: id.clone(),
                team,
                by,
                at: now(),
            })?;
            let machine = store.machine(&id).ok_or(ServerError::MachineUnknown)?;
            Ok(Json(MachineTeamChanged {
                machine: view(directory, machine, last.get(&id).copied())?,
                recorded,
            }))
        })
    })
}

async fn change_agent(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    body: Result<Json<AgentBody>, JsonRejection>,
) -> Result<Json<MachineAgentsChanged>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    crate::routes::administrator(&state, &actor)?;
    let Json(body) = body.map_err(|refused| malformed(refused.body_text()))?;
    let id = OperationId::from_str(&id)
        .map_err(|error| malformed(format!("computer id does not read: {error}")))?
        .to_string();
    let operation = OperationId::from_str(&body.operation)
        .map_err(|error| malformed(format!("operation does not read: {error}")))?
        .to_string();
    let agent = AgentId::from_str(&body.agent)?;
    // The directory and the network are never locked together: the network write
    // syncs to disk, and no directory reader may wait on that flush.
    let by = with_directory(&state, |directory| {
        let directory = directory.projection()?;
        let by = own_person(directory, &actor)?.to_string();
        let record = directory
            .record(IdentityId::Agent(agent))
            .ok_or(ServerError::AgentNotVisible)?;
        if body.allow && record.state() != LifecycleState::Active {
            return Err(ServerError::Inactive {
                identity: agent.to_string(),
                state: record.state(),
            });
        }
        Ok(by)
    })?;
    let last = last_reports(&state)?;
    let (recorded, machine) = with_network(&state, |store| {
        let recorded = store.change_agent(AgentsRecorded {
            operation,
            machine: id.clone(),
            agent: agent.to_string(),
            allow: body.allow,
            by,
            at: now(),
            original_may_run: None,
        })?;
        let machine = store
            .machine(&id)
            .cloned()
            .ok_or(ServerError::MachineUnknown)?;
        Ok((recorded, machine))
    })?;
    with_directory(&state, |directory| {
        Ok(Json(MachineAgentsChanged {
            machine: view(directory.projection()?, &machine, last.get(&id).copied())?,
            recorded,
        }))
    })
}

async fn retire(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<MachineView>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    crate::routes::administrator(&state, &actor)?;
    with_directory(&state, |directory| {
        let directory = directory.projection()?;
        let by = own_person(directory, &actor)?.to_string();
        let last = last_reports(&state)?;
        with_network(&state, |store| {
            store.retire(&id, Retirement { by, at: now() })?;
            let kept = store.machine(&id).ok_or(ServerError::MachineUnknown)?;
            Ok(Json(view(directory, kept, last.get(&kept.id).copied())?))
        })
    })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::error::Error;

    use identity_contract::fake_issuer::Login;
    use identity_contract::harness::{ADMINISTRATOR, Service};
    use lys_core::Ed25519Identity;
    use lys_identity::LifecycleState;
    use serde_json::Value;

    use crate::network_store::{Machine, NetworkStore, Retirement};
    use crate::roles_records::{Holding, Version, Words};
    use crate::roles_store::RolesStore;
    use crate::teams_state::{Changed, Created, Line};
    use crate::teams_store::TeamStore;
    use identity_contract::lys_identity_server::dev_seed::seed_configured;

    type Outcome = Result<(), Box<dyn Error>>;

    fn login(subject: &str) -> Login {
        Login {
            subject: subject.to_owned(),
            email: "scope@example.test".to_owned(),
        }
    }

    fn ids(answer: &Value) -> Result<BTreeSet<String>, Box<dyn Error>> {
        answer["machines"]
            .as_array()
            .ok_or("machines missing")?
            .iter()
            .map(|machine| {
                machine["id"]
                    .as_str()
                    .map(str::to_owned)
                    .ok_or_else(|| "machine id missing".into())
            })
            .collect()
    }

    fn fixture(
        config: &identity_contract::lys_identity_server::Config,
    ) -> Result<BTreeSet<String>, Box<dyn Error>> {
        let seeded = seed_configured(config, [ADMINISTRATOR, "member"])?;
        let owner = seeded.people[0].id.to_string();
        let person = seeded.people[1].id.to_string();
        let agent = seeded.people[1]
            .agents
            .iter()
            .find(|agent| agent.state == LifecycleState::Active)
            .ok_or("active agent missing")?
            .id
            .to_string();
        let by = crate::read_views::Login {
            provider: config.issuer.clone(),
            subject: ADMINISTRATOR.to_owned(),
        };
        let owned_team = format!("op-{:032x}", 101);
        let member_team = format!("op-{:032x}", 102);
        let mut teams = TeamStore::open(
            config.teams_dir.as_deref().ok_or("teams disabled")?,
            std::sync::Arc::new(Ed25519Identity::load(&config.event_key_file)?),
        )?;
        for (id, team_owner) in [(&owned_team, &person), (&member_team, &owner)] {
            teams.keep(Line::Created(Created {
                id: id.clone(),
                owner: team_owner.clone(),
                name: id.clone(),
                description: String::new(),
                by: by.clone(),
                at: 1,
            }))?;
        }
        teams.keep(Line::Added(Changed {
            operation: format!("op-{:032x}", 103),
            team: member_team.clone(),
            member: person.clone(),
            by,
            at: 2,
        }))?;
        let role = format!("op-{:032x}", 104);
        let mut roles = RolesStore::open(config.roles_file.as_deref().ok_or("roles disabled")?)?;
        roles.make(
            "Runner".to_owned(),
            Version {
                number: 1,
                operation: role.clone(),
                words: Words {
                    responsibilities: String::new(),
                    goals: String::new(),
                    practice: String::new(),
                    profile: String::new(),
                    grant_templates: Vec::new(),
                    note: String::new(),
                },
                made_by: owner.clone(),
                made_at: 1,
            },
        )?;
        roles.assign(
            &role,
            Holding {
                operation: format!("op-{:032x}", 105),
                holder: agent.clone(),
                version: 1,
                assigned_by: owner.clone(),
                assigned_at: 1,
                ends_at: None,
                moves: Vec::new(),
                ended: None,
            },
        )?;
        let mut network =
            NetworkStore::open(config.network_file.as_deref().ok_or("network disabled")?)?;
        let mut expected = BTreeSet::new();
        for index in 1..=8 {
            let id = format!("op-{index:032x}");
            let own = index <= 2;
            let team = match index {
                3 => Some(owned_team.clone()),
                4 => Some(member_team.clone()),
                _ => None,
            };
            let retired = matches!(index, 2 | 8).then(|| Retirement {
                by: owner.clone(),
                at: 2,
            });
            network.name(Machine {
                id: id.clone(),
                name: id.clone(),
                kind: "server".to_owned(),
                runtime: Some("norn".to_owned()),
                slots: 1,
                may_run: if matches!(index, 5 | 8) {
                    vec![agent.clone()]
                } else {
                    Vec::new()
                },
                may_run_roles: if index == 6 {
                    vec![role.clone()]
                } else {
                    Vec::new()
                },
                may_reach: Vec::new(),
                named_by: if own { person.clone() } else { owner.clone() },
                named_at: 1,
                retired,
                creation_team: team.clone(),
                team,
            })?;
            if index <= 6 {
                expected.insert(id);
            }
        }
        Ok(expected)
    }

    #[tokio::test]
    async fn personal_network_filters_before_paging_and_counts() -> Outcome {
        let (service, expected) = Service::start_with(fixture).await?;
        let member = service.sign_in(login("member")).await?;
        let administrator = service.sign_in(login(ADMINISTRATOR)).await?;
        let (status, personal) = service.get("/network", Some(&member)).await?;
        assert_eq!(status, 200, "{personal}");
        assert_eq!(
            ids(&personal)?,
            expected,
            "foreign and retired start-only machines must not leak"
        );
        let (status, all) = service
            .get("/network?limit=20", Some(&administrator))
            .await?;
        assert_eq!(status, 200, "{all}");
        assert_eq!(all["total"], 8);
        assert_eq!(ids(&all)?.len(), 8);
        let (status, first) = service.get("/network?limit=2", Some(&member)).await?;
        assert_eq!(status, 200, "{first}");
        assert_eq!(first["total"], 6);
        assert_eq!(ids(&first)?.len(), 2);
        let cursor = first["next"].as_str().ok_or("next cursor missing")?;
        let (status, second) = service
            .get(&format!("/network?limit=2&after={cursor}"), Some(&member))
            .await?;
        assert_eq!(status, 200, "{second}");
        assert_eq!(second["total"], 6);
        assert!(ids(&first)?.is_disjoint(&ids(&second)?));
        let (status, hidden) = service
            .get(
                "/network?q=00000000000000000000000000000007&limit=2",
                Some(&member),
            )
            .await?;
        assert_eq!(status, 200, "{hidden}");
        assert_eq!(hidden["total"], 0);
        assert!(ids(&hidden)?.is_empty());
        let unbound = service.sign_in(login("unbound")).await?;
        let (status, refused) = service.get("/network", Some(&unbound)).await?;
        assert_eq!(status, 403, "{refused}");
        assert_eq!(refused["refusal"], "NoPerson");
        Ok(())
    }
}
