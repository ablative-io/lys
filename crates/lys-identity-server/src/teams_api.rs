//! The team routes: a signed-in person creates a team they own, adds and
//! removes its members and retires it; the administrator may change any
//! team; every signed-in person reads every team. An agent is added only by
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

/// The most characters a team's name carries.
pub(crate) const NAME_MAX: usize = 100;

/// The most characters a team's description carries.
pub(crate) const DESCRIPTION_MAX: usize = 500;

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
    /// Every team, in the order created.
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

pub(crate) fn words(name: &str, text: &str, most: usize) -> Result<String, ServerError> {
    let text = text.trim();
    if text.chars().count() > most {
        return Err(malformed(format!(
            "{name} is longer than {most} characters"
        )));
    }
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
    body: Result<Bytes, BytesRejection>,
) -> Result<Json<TeamChanged>, ServerError> {
    let bytes = body.map_err(|refused| malformed(refused.body_text()))?;
    let actor = giving::actor(&state, &headers, ("POST", uri.path(), &bytes))?;
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
    signed_in(&state, &headers)?;
    with_teams(&state, |store| {
        Ok(TeamsView {
            teams: store.teams().iter().map(view).collect(),
        })
    })
    .map(Json)
}

async fn one(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<TeamView>, ServerError> {
    signed_in(&state, &headers)?;
    let id = team_id(&id)?;
    with_teams(&state, |store| {
        store
            .team(&id)
            .map(view)
            .ok_or(ServerError::Team(TeamError::Unknown))
    })
    .map(Json)
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
