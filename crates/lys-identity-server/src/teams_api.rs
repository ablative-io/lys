//! The team routes: a signed-in person creates a team they own, adds and
//! removes its members and retires it; the administrator may change any
//! team; every signed-in person reads every team.
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

use std::str::FromStr;
use std::sync::{Arc, PoisonError};

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::routing::{get, post};
use axum::{Json, Router};
use lys_identity::{Actor, AgentId, IdentityId, LifecycleState, OperationId, PersonId};
use serde::{Deserialize, Serialize};

use crate::error::ServerError;
use crate::read_api::{login, own_person};
use crate::read_views::Login;
use crate::routes::{AppState, signed_in, with_directory};
use crate::session::now;
use crate::teams_state::{Changed, Created, Line, Team};
use crate::teams_store::TeamStore;

/// The most characters a team's name carries.
const NAME_MAX: usize = 100;

/// The most characters a team's description carries.
const DESCRIPTION_MAX: usize = 500;

/// A team as the routes answer it.
#[derive(Debug, Clone, Serialize)]
pub struct TeamView {
    /// The team, named by the operation id it was created with.
    pub id: String,
    /// The person who owns it.
    pub owner: String,
    /// Its name.
    pub name: String,
    /// What it is for; empty when its creator said nothing.
    pub description: String,
    /// Its members, people and agents, in the order added.
    pub members: Vec<String>,
    /// `active`, or `retired` once retired.
    pub state: String,
    /// The login that created it.
    pub created_by: Login,
    /// When it was created, in seconds since the Unix epoch.
    pub created_at: u64,
    /// When it was retired, null while it is in use.
    pub retired_at: Option<u64>,
}

/// The answer of `GET /teams`.
#[derive(Debug, Clone, Serialize)]
pub struct TeamsView {
    /// Every team, in the order created.
    pub teams: Vec<TeamView>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CreateBody {
    operation: String,
    name: String,
    #[serde(default)]
    description: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MemberBody {
    operation: String,
    member: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RetireBody {
    operation: String,
}

/// The team routes.
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/teams", post(create).get(list))
        .route("/teams/{id}", get(one))
        .route("/teams/{id}/members", post(add))
        .route("/teams/{id}/members/{member}/remove", post(remove))
        .route("/teams/{id}/retire", post(retire))
}

fn malformed(reason: impl Into<String>) -> ServerError {
    ServerError::RequestMalformed {
        reason: reason.into(),
    }
}

fn with_teams<T>(
    state: &AppState,
    act: impl FnOnce(&mut TeamStore) -> Result<T, ServerError>,
) -> Result<T, ServerError> {
    let store = state
        .teams
        .as_ref()
        .ok_or_else(|| ServerError::TeamsUnavailable {
            reason: "the configuration names no teams_dir".to_owned(),
        })?;
    let mut store = store.lock().unwrap_or_else(PoisonError::into_inner);
    store.settle()?;
    act(&mut store)
}

fn view(team: &Team) -> TeamView {
    let created = &team.created;
    TeamView {
        id: created.id.clone(),
        owner: created.owner.clone(),
        name: created.name.clone(),
        description: created.description.clone(),
        members: team.members.clone(),
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

fn words(name: &str, text: &str, most: usize) -> Result<String, ServerError> {
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

fn team_id(id: &str) -> Result<String, ServerError> {
    OperationId::from_str(id)
        .map(|id| id.to_string())
        .map_err(|_unread| ServerError::TeamUnknown)
}

async fn create(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Result<Json<CreateBody>, JsonRejection>,
) -> Result<Json<TeamView>, ServerError> {
    let Json(body) = body.map_err(|refused| malformed(refused.body_text()))?;
    let actor = signed_in(&state, &headers)?;
    let id = OperationId::from_str(&body.operation)?.to_string();
    let name = words("name", &body.name, NAME_MAX)?;
    if name.is_empty() {
        return Err(malformed("a team has a name"));
    }
    let description = words("description", &body.description, DESCRIPTION_MAX)?;
    with_directory(&state, |directory| {
        let owner = own_person(directory.projection()?, &actor)?;
        let line = Line::Created(Created {
            id,
            owner: owner.to_string(),
            name,
            description,
            by: login(actor.binding()),
            at: now(),
        });
        with_teams(&state, |store| store.keep(line).map(|team| view(&team)))
    })
    .map(Json)
}

/// Keep the change `made` makes on team `id`, admitted only for the team's
/// owner or the administrator; a team the caller may not change is refused
/// `NotAdmitted`, and one never created `TeamUnknown`.
fn change(
    state: &AppState,
    actor: &Actor,
    id: &str,
    made: impl FnOnce(String, Login) -> Result<Line, ServerError>,
) -> Result<TeamView, ServerError> {
    let administrator = state.admission.administrator(actor).is_ok();
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
            let team = store.team(id).ok_or(ServerError::TeamUnknown)?;
            if own
                .as_ref()
                .is_some_and(|person| *person != team.created.owner)
            {
                return Err(ServerError::NotAdmitted {
                    reason: "only a team's owner or the administrator changes it",
                });
            }
            store.keep(line).map(|team| view(&team))
        })
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
        .map_err(|_unread| ServerError::TeamMemberUnknown)?;
    let record = projection
        .record(identity)
        .ok_or(ServerError::TeamMemberUnknown)?;
    if record.state() == LifecycleState::Retired {
        return Err(ServerError::TeamMemberUnknown);
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
    Path(id): Path<String>,
    body: Result<Json<MemberBody>, JsonRejection>,
) -> Result<Json<TeamView>, ServerError> {
    let Json(body) = body.map_err(|refused| malformed(refused.body_text()))?;
    let actor = signed_in(&state, &headers)?;
    let id = team_id(&id)?;
    let member = body.member.trim().to_owned();
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
) -> Result<Json<TeamView>, ServerError> {
    let Json(body) = body.map_err(|refused| malformed(refused.body_text()))?;
    let actor = signed_in(&state, &headers)?;
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
) -> Result<Json<TeamView>, ServerError> {
    let Json(body) = body.map_err(|refused| malformed(refused.body_text()))?;
    let actor = signed_in(&state, &headers)?;
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
        store.team(&id).map(view).ok_or(ServerError::TeamUnknown)
    })
    .map(Json)
}
