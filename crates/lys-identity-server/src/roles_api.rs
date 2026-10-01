//! The role routes. A role is a job and a starting point: what the holder
//! answers for, the profile an agent holding it starts from, and the grants
//! a holder usually needs.
//!
//! The administrator makes a role, makes its next version, assigns it, moves
//! a holder and ends a holding. Every signed-in identity the directory knows
//! may read the roles.
//!
//! A new version moves no holder: a holder stays at the version it holds
//! until the administrator moves it, and the move is kept with who made it.
//! The grant templates are copied when a grant is made, so assigning a role
//! issues no grant. A holding's end is set when it is assigned; no move and
//! no new version changes it, and a holding that lapsed is never moved.

use std::str::FromStr;
use std::sync::Arc;

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::routing::{get, post};
use axum::{Json, Router};
use lys_identity::grants::{Relation, Resource};
use lys_identity::projection::Projection;
use lys_identity::{Actor, OperationId};
use serde::Deserialize;

use crate::caller_admission::active_caller;
use crate::error::ServerError;
use crate::grants::caller;
use crate::roles_records::{Ending, Holding, Move, Role, Template, Version, Words};
use crate::roles_store::RolesStore;
use crate::roles_views::{
    HolderView, MoveView, MovedView, POLICY, RoleList, RoleView, VersionView,
};
use crate::routes::{AppState, identity_id, signed_in, with_directory};
use crate::session::now;

/// The most characters a role's name carries.
const NAME_MAX: usize = 100;
/// The most characters a version's note carries.
const NOTE_MAX: usize = 500;
/// The most characters each of a version's texts carries.
const TEXT_MAX: usize = 4000;

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = RoleResourceBody)]
pub(crate) struct ResourceBody {
    kind: String,
    id: String,
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct TemplateBody {
    resource: ResourceBody,
    relation: String,
    days: Option<u32>,
}

/// What a version says. Every member is required; `profile` may be empty
/// and `grant_templates` may be an empty list.
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct VersionBody {
    operation: String,
    responsibilities: String,
    goals: String,
    practice: String,
    profile: String,
    grant_templates: Vec<TemplateBody>,
    note: String,
}

/// A role to make: its name, and what its first version says.
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct MakeBody {
    operation: String,
    name: String,
    responsibilities: String,
    goals: String,
    practice: String,
    profile: String,
    grant_templates: Vec<TemplateBody>,
    note: String,
}

/// A holder to assign. `ends_at` may be null.
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct AssignBody {
    operation: String,
    holder: String,
    ends_at: Option<u64>,
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct MoveBody {
    assignment: String,
    from_version: u32,
    to_version: u32,
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct EndBody {
    assignment: String,
}

/// The role routes.
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/roles", get(list).post(make))
        .route("/roles/{id}", get(read))
        .route("/roles/{id}/versions", post(revise))
        .route("/roles/{id}/holders", post(assign))
        .route("/roles/{id}/holders/{holder}/move", post(move_holder))
        .route("/roles/{id}/holders/{holder}/end", post(end))
}

fn malformed(reason: impl Into<String>) -> ServerError {
    ServerError::RequestMalformed {
        reason: reason.into(),
    }
}

fn taken<T>(body: Result<Json<T>, JsonRejection>) -> Result<T, ServerError> {
    body.map(|Json(body)| body)
        .map_err(|refused| malformed(refused.body_text()))
}

fn words(name: &str, text: &str, most: usize, may_be_empty: bool) -> Result<String, ServerError> {
    let text = text.trim();
    if text.is_empty() && !may_be_empty {
        return Err(malformed(format!("{name} is empty")));
    }
    if text.chars().count() > most {
        return Err(malformed(format!(
            "{name} is longer than {most} characters"
        )));
    }
    Ok(text.to_owned())
}

fn with_roles<T>(
    state: &AppState,
    act: impl FnOnce(&mut RolesStore) -> Result<T, ServerError>,
) -> Result<T, ServerError> {
    let store = state
        .roles
        .as_ref()
        .ok_or_else(|| ServerError::RolesUnavailable {
            reason: "the configuration names no roles_file".to_owned(),
        })?;
    let mut store = store
        .lock()
        .map_err(|error| ServerError::RolesUnavailable {
            reason: format!("the roles lock is poisoned: {error}"),
        })?;
    store.settle()?;
    act(&mut store)
}

/// The administrator's session, or the refusal of any other caller.
fn administrator(state: &AppState, headers: &HeaderMap) -> Result<Actor, ServerError> {
    let actor = signed_in(state, headers)?;
    crate::routes::administrator(state, &actor)?;
    Ok(actor)
}

fn holder_view(directory: &Projection, role: &Role, holding: &Holding, at: u64) -> HolderView {
    let display_name = identity_id(&holding.holder)
        .ok()
        .and_then(|identity| directory.record(identity))
        .map(|record| record.profile().display_name().to_owned());
    HolderView {
        assignment: holding.operation.clone(),
        holder: holding.holder.clone(),
        display_name,
        version: holding.version,
        behind: holding.version < role.latest(),
        assigned_by: holding.assigned_by.clone(),
        assigned_at: holding.assigned_at,
        ends_at: holding.ends_at,
        moves_at: None,
        state: holding.state(at),
        moves: holding.moves.iter().map(MoveView::from).collect(),
        ended_by: holding.ended.as_ref().map(|ended| ended.by.clone()),
        ended_at: holding.ended.as_ref().map(|ended| ended.at),
    }
}

fn view(directory: &Projection, role: &Role, at: u64) -> RoleView {
    RoleView {
        id: role.id.clone(),
        name: role.name.clone(),
        latest: role.latest(),
        policy: POLICY,
        versions: role.versions.iter().map(VersionView::from).collect(),
        holders: role
            .holdings
            .iter()
            .map(|holding| holder_view(directory, role, holding, at))
            .collect(),
    }
}

/// What `body` says, checked against the grant model.
fn said(state: &AppState, body: &VersionBody) -> Result<Words, ServerError> {
    let mut grant_templates = Vec::new();
    for template in &body.grant_templates {
        let resource = Resource::new(&template.resource.kind, &template.resource.id)?;
        let relation = Relation::new(&template.relation)?;
        state.grant_setup.model()?.actions(&relation)?;
        if template.days == Some(0) {
            return Err(malformed(
                "a template's days is 1 or more, or null for no end of its own",
            ));
        }
        grant_templates.push(Template {
            resource_kind: resource.kind().to_owned(),
            resource_id: resource.id().to_owned(),
            relation: relation.to_string(),
            days: template.days,
        });
    }
    Ok(Words {
        responsibilities: words("responsibilities", &body.responsibilities, TEXT_MAX, false)?,
        goals: words("goals", &body.goals, TEXT_MAX, false)?,
        practice: words("practice", &body.practice, TEXT_MAX, false)?,
        profile: words("profile", &body.profile, TEXT_MAX, true)?,
        grant_templates,
        note: words("note", &body.note, NOTE_MAX, false)?,
    })
}

async fn list(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<RoleList>, ServerError> {
    with_directory(&state, |directory| {
        let directory = directory.projection()?;
        caller(&state, &headers, directory)?;
        let at = now();
        with_roles(&state, |store| {
            Ok(Json(RoleList {
                roles: store
                    .roles()
                    .iter()
                    .map(|role| view(directory, role, at))
                    .collect(),
            }))
        })
    })
}

async fn read(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<RoleView>, ServerError> {
    with_directory(&state, |directory| {
        let directory = directory.projection()?;
        caller(&state, &headers, directory)?;
        with_roles(&state, |store| {
            let role = store.role(&id).ok_or(ServerError::RoleUnknown)?;
            Ok(Json(view(directory, role, now())))
        })
    })
}

async fn make(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Result<Json<MakeBody>, JsonRejection>,
) -> Result<Json<RoleView>, ServerError> {
    let actor = administrator(&state, &headers)?;
    let body = taken(body)?;
    with_directory(&state, |directory| {
        let directory = directory.projection()?;
        let at = now();
        let name = words("name", &body.name, NAME_MAX, false)?;
        let body = VersionBody {
            operation: body.operation,
            responsibilities: body.responsibilities,
            goals: body.goals,
            practice: body.practice,
            profile: body.profile,
            grant_templates: body.grant_templates,
            note: body.note,
        };
        let first = Version {
            number: 1,
            operation: OperationId::from_str(&body.operation)?.to_string(),
            words: said(&state, &body)?,
            made_by: active_caller(directory, &actor)?.to_string(),
            made_at: at,
        };
        with_roles(&state, |store| {
            let id = first.operation.clone();
            store.make(name, first)?;
            let role = store.role(&id).ok_or(ServerError::RoleUnknown)?;
            Ok(Json(view(directory, role, at)))
        })
    })
}

async fn revise(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    body: Result<Json<VersionBody>, JsonRejection>,
) -> Result<Json<RoleView>, ServerError> {
    let actor = administrator(&state, &headers)?;
    let body = taken(body)?;
    with_directory(&state, |directory| {
        let directory = directory.projection()?;
        let by = active_caller(directory, &actor)?.to_string();
        let operation = OperationId::from_str(&body.operation)?.to_string();
        let said = said(&state, &body)?;
        let at = now();
        with_roles(&state, |store| {
            store.revise(&id, &operation, said, &by, at)?;
            let role = store.role(&id).ok_or(ServerError::RoleUnknown)?;
            Ok(Json(view(directory, role, at)))
        })
    })
}

async fn assign(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    body: Result<Json<AssignBody>, JsonRejection>,
) -> Result<Json<RoleView>, ServerError> {
    let actor = administrator(&state, &headers)?;
    let body = taken(body)?;
    with_directory(&state, |directory| {
        let directory = directory.projection()?;
        let by = active_caller(directory, &actor)?.to_string();
        let operation = OperationId::from_str(&body.operation)?.to_string();
        let holder = identity_id(&body.holder)
            .ok()
            .filter(|holder| directory.record(*holder).is_some())
            .ok_or(ServerError::HolderUnknown)?;
        let at = now();
        with_roles(&state, |store| {
            let kept = store
                .role(&id)
                .ok_or(ServerError::RoleUnknown)?
                .holdings
                .iter()
                .any(|holding| holding.operation == operation);
            if !kept && body.ends_at.is_some_and(|end| end <= at) {
                return Err(malformed("ends_at is not after now"));
            }
            store.assign(
                &id,
                Holding {
                    operation,
                    holder: holder.to_string(),
                    version: 0,
                    assigned_by: by,
                    assigned_at: at,
                    ends_at: body.ends_at,
                    moves: Vec::new(),
                    ended: None,
                },
            )?;
            let role = store.role(&id).ok_or(ServerError::RoleUnknown)?;
            Ok(Json(view(directory, role, at)))
        })
    })
}

async fn move_holder(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((id, holder)): Path<(String, String)>,
    body: Result<Json<MoveBody>, JsonRejection>,
) -> Result<Json<MovedView>, ServerError> {
    let actor = administrator(&state, &headers)?;
    let body = taken(body)?;
    with_directory(&state, |directory| {
        let directory = directory.projection()?;
        let by = active_caller(directory, &actor)?.to_string();
        let at = now();
        with_roles(&state, |store| {
            let moved = Move {
                from: body.from_version,
                to: body.to_version,
                by,
                at,
            };
            store.move_holder(&id, &holder, &body.assignment, moved)?;
            let role = store.role(&id).ok_or(ServerError::RoleUnknown)?;
            let holding = role
                .holdings
                .iter()
                .find(|kept| kept.operation == body.assignment)
                .ok_or(ServerError::HolderUnknown)?;
            let version = |number| {
                role.version(number)
                    .map(VersionView::from)
                    .ok_or(ServerError::RoleVersionUnknown)
            };
            Ok(Json(MovedView {
                role: role.id.clone(),
                holder: holder_view(directory, role, holding, at),
                from: version(body.from_version)?,
                to: version(body.to_version)?,
            }))
        })
    })
}

async fn end(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((id, holder)): Path<(String, String)>,
    body: Result<Json<EndBody>, JsonRejection>,
) -> Result<Json<RoleView>, ServerError> {
    let actor = administrator(&state, &headers)?;
    let body = taken(body)?;
    with_directory(&state, |directory| {
        let directory = directory.projection()?;
        let by = active_caller(directory, &actor)?.to_string();
        let at = now();
        with_roles(&state, |store| {
            store.end(&id, &holder, &body.assignment, Ending { by, at })?;
            let role = store.role(&id).ok_or(ServerError::RoleUnknown)?;
            Ok(Json(view(directory, role, at)))
        })
    })
}

/// Every role id kept; none when the configuration names no roles.
pub(crate) fn role_ids(state: &AppState) -> Result<Vec<String>, ServerError> {
    if state.roles.is_none() {
        return Ok(Vec::new());
    }
    with_roles(state, |store| {
        Ok(store.roles().iter().map(|role| role.id.clone()).collect())
    })
}

/// The ids of the roles `holder` holds at `at`: assigned, not ended, and
/// not past the end it was assigned with; none when no roles are kept.
pub(crate) fn held_roles(
    state: &AppState,
    holder: &str,
    at: u64,
) -> Result<Vec<String>, ServerError> {
    if state.roles.is_none() {
        return Ok(Vec::new());
    }
    with_roles(state, |store| {
        Ok(store
            .roles()
            .iter()
            .filter(|role| {
                role.holding(holder).is_some_and(|holding| {
                    holding.ended.is_none() && holding.ends_at.is_none_or(|ends| ends > at)
                })
            })
            .map(|role| role.id.clone())
            .collect())
    })
}
