//! The team routes: a signed-in person creates a team they own, adds and
//! removes its members and retires it; the administrator may change any
//! team; a person reads only teams they own or belong to. An agent is added only by
//! a caller who may operate it, and a person only by themselves or the
//! administrator, because a team's goals and budgets act on its members.
//!
//! A team is a named group and nothing more: it carries no grant, and being
//! in one confers no authority. Whether a team should hold grants its
//! members inherit is not decided, and nothing here builds it.
//!
//! Each act is sent under an operation id. A creation is named by its
//! operation id; every later change carries one of its own. The same act
//! sent again in the same words answers the team as it stands and writes
//! nothing; the same operation in other words is refused `TeamReused`. A
//! member is a person or an agent the directory holds and has not retired.
//!
//! Every act answers the team as it stands now beside `recorded`, the line
//! its operation was first kept as. A retry therefore reads what it did the
//! first time, however the team has changed since.

use std::str::FromStr;
use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::rejection::{BytesRejection, JsonRejection};
use axum::extract::{OriginalUri, Path, State};
use axum::http::HeaderMap;
use axum::routing::{get, post};
use axum::{Json, Router};
use lys_identity::{Actor, AgentId, IdentityId, LifecycleState, OperationId, PersonId};
use serde::{Deserialize, Serialize};

use crate::error::ServerError;
use crate::error_team::TeamError;
use crate::read_api::{login, own_person};
use crate::read_views::Login;
use crate::routes::{AppState, signed_in, with_directory};
use crate::session::now;
use crate::teams_state::{Changed, Line, Team};
use crate::teams_store::TeamStore;

#[path = "teams_giving.rs"]
pub(crate) mod giving;

#[cfg(test)]
#[path = "teams_giving_tests.rs"]
mod giving_tests;

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::error::Error;
    use std::sync::Arc;

    use identity_contract::fake_issuer::Login;
    use identity_contract::harness::{ADMINISTRATOR, Service};
    use lys_core::Ed25519Identity;
    use identity_contract::lys_identity_server::Config;
    use identity_contract::lys_identity_server::dev_seed::seed_configured;
    use crate::teams_state::{Changed, Checked, Created, Hold, Line};
    use crate::teams_store::TeamStore;

    type Outcome = Result<(), Box<dyn Error>>;

    fn login(subject: &str) -> Login {
        Login { subject: subject.to_owned(), email: "scope@example.test".to_owned() }
    }

    fn fixture(config: &Config) -> Result<BTreeSet<String>, Box<dyn Error>> {
        let seeded = seed_configured(config, [ADMINISTRATOR, "member"])?;
        let administrator = seeded.people[0].id.to_string();
        let person = seeded.people[1].id.to_string();
        let by = crate::read_views::Login { provider: config.issuer.clone(), subject: ADMINISTRATOR.to_owned() };
        let mut store = TeamStore::open(config.teams_dir.as_deref().ok_or("teams disabled")?,
            Arc::new(Ed25519Identity::load(&config.event_key_file)?))?;
        let mut expected = BTreeSet::new();
        let mut migrated = Vec::new();
        for index in 1..=4 {
            let id = format!("op-{index:032x}");
            store.keep(Line::Created(Created { id: id.clone(), name: id.clone(),
                owner: if index == 1 { person.clone() } else { administrator.clone() },
                description: String::new(), by: by.clone(), at: 1 }))?;
            if matches!(index, 2 | 4) {
                store.keep(Line::Added(Changed { operation: format!("op-{:032x}", index + 10), team: id.clone(),
                    member: person.clone(), by: by.clone(), at: 2 }))?;
            }
            if index == 4 {
                // A hold is a migration line: it goes through the migration writer.
                migrated.push(Line::Held(Hold { operation: format!("op-{:032x}", 20), team: id.clone(),
                    member: person.clone(), reason: "membership awaits admission".to_owned(), at: 3 }));
            }
            if index <= 2 { expected.insert(id); }
        }
        migrated.push(Line::Checked(Checked {
            operation: "lys/teams/legacy-membership/v1/checked".to_owned(),
            at: 4,
        }));
        store.stage_migration(migrated)?;
        store.finish_migration()?;
        Ok(expected)
    }

    #[tokio::test]
    async fn personal_teams_include_only_owned_or_effective_memberships() -> Outcome {
        let (service, expected) = Service::start_with(fixture).await?;
        let member = service.sign_in(login("member")).await?;
        let administrator = service.sign_in(login(ADMINISTRATOR)).await?;
        let (status, personal) = service.get("/teams", Some(&member)).await?;
        assert_eq!(status, 200, "{personal}");
        let ids = personal["teams"].as_array().ok_or("teams missing")?.iter()
            .map(|team| team["id"].as_str().map(str::to_owned).ok_or("team id missing"))
            .collect::<Result<BTreeSet<_>, _>>()?;
        assert_eq!(ids, expected, "another person's and held memberships must not leak");
        for index in 1..=4 {
            let (status, one) = service.get(&format!("/teams/op-{index:032x}"), Some(&member)).await?;
            if index <= 2 {
                assert_eq!(status, 200, "{one}");
                assert_eq!(one["id"], format!("op-{index:032x}"));
            } else {
                assert_eq!(status, 404, "{one}");
                assert_eq!(one["refusal"], "TeamUnknown");
            }
        }
        let (status, all) = service.get("/teams", Some(&administrator)).await?;
        assert_eq!(status, 200, "{all}");
        assert_eq!(all["teams"].as_array().ok_or("teams missing")?.len(), 4);
        let unbound = service.sign_in(login("unbound")).await?;
        let (status, refused) = service.get("/teams", Some(&unbound)).await?;
        assert_eq!(status, 403, "{refused}");
        assert_eq!(refused["refusal"], "NoPerson");
        Ok(())
    }
}

/// A team as the routes answer it.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct TeamView {
    /// The team, named by the operation id it was created with.
    pub id: String,
    /// The person who owns it.
    pub owner: String,
    /// The parent team, null for a top-level team.
    pub parent: Option<String>,
    /// The admitted member who leads it, null when none is named.
    pub lead: Option<String>,
    /// Its name.
    pub name: String,
    /// What it is for; empty when its creator said nothing.
    pub description: String,
    /// Its members, people and agents, in the order added.
    pub members: Vec<String>,
    /// Memberships excluded from team actions until an administrator confirms them.
    pub held: Vec<crate::teams_state::Hold>,
    /// `active`, or `retired` once retired.
    pub state: String,
    /// The login that created it.
    pub created_by: Login,
    /// When it was created, in seconds since the Unix epoch.
    pub created_at: u64,
    /// When it was retired, null while it is in use.
    pub retired_at: Option<u64>,
}

/// The line an operation was first kept as.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[schema(as = TeamRecorded)]
pub struct Recorded {
    /// The operation id the act was sent under.
    pub operation: String,
    /// `created`, `added`, `removed` or `retired`.
    pub act: String,
    /// The member added or removed; null for a creation or a retirement.
    pub member: Option<String>,
    /// The login that made it.
    pub by: Login,
    /// When it was first kept, in seconds since the Unix epoch.
    pub at: u64,
}

/// The answer of every act on a team.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct TeamChanged {
    /// The team as it stands now.
    #[serde(flatten)]
    pub team: TeamView,
    /// What this operation was first kept as.
    pub recorded: Recorded,
}

/// The answer of `GET /teams`.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct TeamsView {
    /// Admitted teams, in the order created.
    pub teams: Vec<TeamView>,
}

pub(crate) use crate::teams_nesting::CreateBody;

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = TeamMemberBody)]
pub(crate) struct MemberBody {
    operation: String,
    member: String,
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = TeamRetireBody)]
pub(crate) struct RetireBody {
    operation: String,
}

/// The team routes.
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/teams", post(crate::teams_nesting::create).get(list))
        .route("/teams/{id}/nesting", post(crate::teams_nesting::nest))
        .route("/teams/{id}", get(one))
        .route("/teams/{id}/members", post(add))
        .route("/teams/{id}/members/{member}/remove", post(remove))
        .route("/teams/{id}/retire", post(retire))
        .route("/teams/{id}/members/{member}/confirm", post(confirm))
}

pub(crate) fn malformed(reason: impl Into<String>) -> ServerError {
    ServerError::RequestMalformed {
        reason: reason.into(),
    }
}

pub(crate) fn with_teams<T>(
    state: &AppState,
    act: impl FnOnce(&mut TeamStore) -> Result<T, ServerError>,
) -> Result<T, ServerError> {
    let store = state.teams.as_ref().ok_or_else(|| {
        ServerError::Team(TeamError::Unavailable {
            reason: "the configuration names no teams_dir".to_owned(),
        })
    })?;
    let mut store = store.lock().map_err(|error| {
        ServerError::Team(TeamError::Unavailable {
            reason: format!("the teams lock is poisoned: {error}"),
        })
    })?;
    store.settle()?;
    act(&mut store)
}

fn view(team: &Team) -> TeamView {
    let created = &team.created;
    TeamView {
        id: created.id.clone(),
        owner: created.owner.clone(),
        parent: team.parent.clone(),
        lead: team.lead.clone(),
        name: created.name.clone(),
        description: created.description.clone(),
        members: team.members.clone(),
        held: team.held.clone(),
        state: if team.retired.is_some() {
            "retired"
        } else {
            "active"
        }
        .to_owned(),
        created_by: created.by.clone(),
        created_at: created.at,
        retired_at: team.retired.as_ref().map(|retired| retired.at),
    }
}

fn recorded(line: &Line) -> Result<Recorded, ServerError> {
    let (act, member) = match line {
        Line::Created(_) | Line::CreatedV1(_) => ("created", None),
        Line::NestedV1(_) => ("nested", None),
        Line::Added(changed) => ("added", Some(changed.member.clone())),
        Line::Removed(changed) => ("removed", Some(changed.member.clone())),
        Line::Retired(_) => ("retired", None),
        Line::Confirmed(changed) => ("confirmed", Some(changed.member.clone())),
        Line::Held(_) | Line::Checked(_) => {
            return Err(ServerError::Team(TeamError::Unavailable {
                reason: format!(
                    "operation `{}` records a system migration, not a caller act",
                    line.operation()
                ),
            }));
        }
    };
    let (by, at) = match line {
        Line::Created(created) => (created.by.clone(), created.at),
        Line::CreatedV1(created) => (created.created.by.clone(), created.created.at),
        Line::NestedV1(nested) => (nested.by.clone(), nested.at),
        Line::Added(changed)
        | Line::Removed(changed)
        | Line::Retired(changed)
        | Line::Confirmed(changed) => (changed.by.clone(), changed.at),
        Line::Held(_) | Line::Checked(_) => {
            return Err(ServerError::Team(TeamError::Unavailable {
                reason: "a system migration has no authenticated caller".to_owned(),
            }));
        }
    };
    Ok(Recorded {
        operation: line.operation().to_owned(),
        act: act.to_owned(),
        member,
        by,
        at,
    })
}

/// Keep `line` and answer the team as it stands beside the line its
/// operation was first kept as.
pub(crate) fn kept(store: &mut TeamStore, line: Line) -> Result<TeamChanged, ServerError> {
    let operation = line.operation().to_owned();
    let team = store.keep(line)?;
    let first = store.recorded(&operation).ok_or_else(|| {
        ServerError::Team(TeamError::Unavailable {
            reason: format!("operation `{operation}` was kept and is not held"),
        })
    })?;
    Ok(TeamChanged {
        team: view(&team),
        recorded: recorded(&first)?,
    })
}

pub(crate) fn words(name: &str, text: &str) -> Result<String, ServerError> {
    let text = text.trim();
    if text.chars().any(char::is_control) {
        return Err(malformed(format!("{name} carries a control character")));
    }
    Ok(text.to_owned())
}

pub(crate) fn team_id(id: &str) -> Result<String, ServerError> {
    OperationId::from_str(id)
        .map(|id| id.to_string())
        .map_err(|_unread| ServerError::Team(TeamError::Unknown))
}

/// Keep the change `made` makes on team `id`, admitted only for the team's
/// owner or the administrator; a team the caller may not change is refused
/// `NotAdmitted`, and one never created `TeamUnknown`.
pub(crate) fn change(
    state: &AppState,
    actor: &Actor,
    id: &str,
    made: impl FnOnce(String, Login) -> Result<Line, ServerError>,
) -> Result<TeamChanged, ServerError> {
    crate::teams_migration::advance(state)?;
    let administrator = crate::routes::is_administrator(state, actor)?;
    with_directory(state, |directory| {
        let projection = directory.projection()?;
        let own = if administrator {
            None
        } else {
            Some(own_person(projection, actor)?.to_string())
        };
        let line = made(id.to_owned(), login(actor.binding()))?;
        if let Line::Added(changed) = &line {
            member_holds(projection, &changed.member)?;
        }
        with_teams(state, |store| {
            let team = store
                .team(id)
                .ok_or(ServerError::Team(TeamError::Unknown))?;
            if own
                .as_ref()
                .is_some_and(|person| *person != team.created.owner)
            {
                return Err(ServerError::NotAdmitted {
                    reason: "only a team's owner or the administrator changes it",
                });
            }
            kept(store, line)
        })
    })
}

/// Refuse, by name, a member the caller may not act for: an agent unless
/// the caller may operate it, since a team's goals are typed into its
/// members' sessions and its budgets stop them; a person unless it is the
/// caller or the administrator adds them.
fn may_add(
    state: &AppState,
    headers: &HeaderMap,
    actor: &Actor,
    member: &str,
) -> Result<(), ServerError> {
    if AgentId::from_str(member).is_ok() {
        return crate::runner_sessions::operator(state, headers, member, "add to a team")
            .map(|_caller| ());
    }
    if crate::routes::is_administrator(state, actor)? {
        return Ok(());
    }
    let own = with_directory(state, |directory| {
        own_person(directory.projection()?, actor).map(|person| person.to_string())
    })?;
    if own == member {
        return Ok(());
    }
    Err(ServerError::NotAdmitted {
        reason: "a person joins a team only by their own act or the administrator's",
    })
}

/// Refuse by name a member the directory does not hold or has retired.
fn member_holds(
    projection: &lys_identity::projection::Projection,
    member: &str,
) -> Result<(), ServerError> {
    let identity = PersonId::from_str(member)
        .map(IdentityId::Person)
        .or_else(|_unread| AgentId::from_str(member).map(IdentityId::Agent))
        .map_err(|_unread| ServerError::Team(TeamError::MemberUnknown))?;
    let record = projection
        .record(identity)
        .ok_or(ServerError::Team(TeamError::MemberUnknown))?;
    if record.state() == LifecycleState::Retired {
        return Err(ServerError::Team(TeamError::MemberUnknown));
    }
    Ok(())
}

fn changed(
    operation: &str,
    team: String,
    member: String,
    by: Login,
) -> Result<Changed, ServerError> {
    Ok(Changed {
        operation: OperationId::from_str(operation)?.to_string(),
        team,
        member,
        by,
        at: now(),
    })
}

async fn add(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    OriginalUri(uri): OriginalUri,
    Path(id): Path<String>,
    principal: Option<axum::Extension<crate::agent_signature::TokenPrincipal>>,
    body: Result<Bytes, BytesRejection>,
) -> Result<Json<TeamChanged>, ServerError> {
    let bytes = body.map_err(|refused| malformed(refused.body_text()))?;
    let actor = giving::actor(
        &state,
        &headers,
        ("POST", uri.path(), &bytes),
        principal.as_ref().map(|value| &value.0),
    )?;
    let body: MemberBody = giving::json(headers.clone(), bytes).await?;
    let id = team_id(&id)?;
    let member = body.member.trim().to_owned();
    if actor.provenance().agent().is_some() {
        return giving::add(
            &state,
            &actor,
            changed(&body.operation, id, member, login(actor.binding()))?,
        )
        .map(Json);
    }
    may_add(&state, &headers, &actor, &member)?;
    change(&state, &actor, &id, |team, by| {
        changed(&body.operation, team, member, by).map(Line::Added)
    })
    .map(Json)
}

async fn remove(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((id, member)): Path<(String, String)>,
    body: Result<Json<RetireBody>, JsonRejection>,
) -> Result<Json<TeamChanged>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    let Json(body) = body.map_err(|refused| malformed(refused.body_text()))?;
    let id = team_id(&id)?;
    change(&state, &actor, &id, |team, by| {
        changed(&body.operation, team, member, by).map(Line::Removed)
    })
    .map(Json)
}

async fn retire(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    body: Result<Json<RetireBody>, JsonRejection>,
) -> Result<Json<TeamChanged>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    let Json(body) = body.map_err(|refused| malformed(refused.body_text()))?;
    let id = team_id(&id)?;
    change(&state, &actor, &id, |team, by| {
        changed(&body.operation, team, String::new(), by).map(Line::Retired)
    })
    .map(Json)
}

async fn list(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<TeamsView>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    with_readable_teams(&state, &actor, |store, person| {
        Ok(TeamsView {
            teams: store.teams_iter().filter(|team| visible_to(team, person)).map(view).collect(),
        })
    })
    .map(Json)
}

async fn one(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<TeamView>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    let id = team_id(&id)?;
    with_readable_teams(&state, &actor, |store, person| {
        store
            .team(&id)
            .filter(|team| visible_to(team, person))
            .map(view)
            .ok_or(ServerError::Team(TeamError::Unknown))
    })
    .map(Json)
}

fn visible_to(team: &Team, person: Option<&str>) -> bool {
    person.is_none_or(|person| team.created.owner == person
        || (team.members.iter().any(|member| member == person)
            && !team.held.iter().any(|held| held.member == person)))
}

fn with_readable_teams<T>(
    state: &AppState,
    actor: &Actor,
    act: impl FnOnce(&TeamStore, Option<&str>) -> Result<T, ServerError>,
) -> Result<T, ServerError> {
    with_directory(state, |directory| {
        let projection = directory.projection()?;
        let person = if state.admission.is_administrator(projection, actor)? {
            None
        } else {
            Some(own_person(projection, actor)?.to_string())
        };
        with_teams(state, |store| act(store, person.as_deref()))
    })
}

async fn confirm(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((id, member)): Path<(String, String)>,
    body: Result<Json<RetireBody>, JsonRejection>,
) -> Result<Json<TeamChanged>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    crate::routes::administrator(&state, &actor)?;
    crate::teams_migration::require_committed(&state)?;
    let Json(body) = body.map_err(|refused| malformed(refused.body_text()))?;
    let id = team_id(&id)?;
    change(&state, &actor, &id, |team, by| {
        changed(&body.operation, team, member, by).map(Line::Confirmed)
    })
    .map(Json)
}
