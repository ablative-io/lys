//! An app's schema after registration: versioned changes, their dry run,
//! every version readable, and where an app's resources are placed.
//!
//! `PUT /apps/{app}/schema` names the version it replaces and is refused
//! `schema_version_moved` when that is not the current one. A change that
//! would strand a standing grant, by removing its kind or its relation or
//! an action it carries, is refused `schema_change_strands_grants` naming
//! each relation and its count; it is taken only once those grants are
//! revoked through the ordinary revoke route, and nothing here revokes or
//! rewrites a grant. `POST /apps/{app}/schema/check` is the same computation
//! and writes nothing. A change the administrator makes is the next version
//! at once; one an app makes waits for the administrator on the Apps
//! screen, and is judged against the standing grants again when approved.
//! The app `lys`'s schema changes only by the administrator. Every version
//! stays readable at `GET /apps/{app}/schema?version=N`.
//!
//! A change naming roles (ACCESS-004 R1) is judged the same way: one an app
//! makes that widens a role waits for the administrator like any app-made
//! change, its diff naming each widening in words, and one that narrows or
//! removes a role is refused `schema_change_strands_grants` while a standing
//! grant names that role.
//!
//! `POST /apps/{app}/placements` records that a resource of an app kind is
//! in a parent its kind's schema lists, so the relations held on the parent
//! flow to it; unless it is `restricted` (ACCESS-004 R2), when nothing held
//! on the parent reaches the child and the engine is given no parent
//! relationship for it, so only a grant on the child itself reaches it.

use std::str::FromStr;
use std::sync::Arc;

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::routing::{get, post};
use axum::{Json, Router};
use lys_identity::OperationId;
use lys_identity::grants::{
    AppSchema, GrantBook, Resource, Standing as Held, diff, owner_of, stranded,
};
use serde::Deserialize;
use serde_json::Value;

use crate::apps_api::refresh;
use crate::apps_binding::{Acting, acting};
use crate::apps_error::{AppError, Strand};
use crate::apps_state::{Applied, By, Decided, Line, Placed, Proposed, Standing};
use crate::apps_store::AppStore;
use crate::apps_views::{AppView, DiffView, SchemaChanged, SchemaCheck, SchemaVersionView};
use crate::error::ServerError;
use crate::grants::with_grants;
use crate::routes::AppState;
use crate::session::now;
use crate::spicedb::Relationships;

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ChangeBody {
    operation: String,
    replaces: u64,
    #[schema(value_type = Object)]
    schema: Value,
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct CheckBody {
    #[serde(default)]
    replaces: Option<u64>,
    #[schema(value_type = Object)]
    schema: Value,
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ChangeDecision {
    operation: String,
    #[serde(default)]
    reason: String,
}

#[derive(Deserialize)]
struct VersionQuery {
    version: Option<u64>,
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ResourceBody {
    kind: String,
    id: String,
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct PlaceBody {
    operation: String,
    child: ResourceBody,
    parent: ResourceBody,
    /// Whether the parent's relations stop at the child (ACCESS-004 R2);
    /// absent, they flow to it as before.
    #[serde(default)]
    restricted: bool,
}

/// The schema routes.
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/apps/{app}/schema", get(version).put(change))
        .route("/apps/{app}/schema/check", post(check))
        .route("/apps/{app}/schema/approve", post(approve))
        .route("/apps/{app}/schema/decline", post(decline))
        .route("/apps/{app}/placements", post(place))
}

fn malformed(reason: impl Into<String>) -> ServerError {
    ServerError::RequestMalformed {
        reason: reason.into(),
    }
}

/// Refuse unless `who` may change or ask about the schema of `app`: the
/// administrator any app's, an app its own and never `lys`'s.
fn may_change(who: &Acting, app: &str) -> Result<(), ServerError> {
    match who {
        Acting::Administrator(_) => Ok(()),
        Acting::App { app: own, .. } if own == app => Ok(()),
        _ => Err(ServerError::NotAdmitted {
            reason: "only the administrator, or the app itself, changes an app's schema",
        }),
    }
}

/// The current schema and version of the approved app `id`.
fn current(apps: &AppStore, id: &str) -> Result<(AppSchema, u64), ServerError> {
    let app = apps
        .app(id)
        .ok_or_else(|| AppError::AppUnknown { app: id.to_owned() })?;
    match app.standing() {
        Standing::Approved => {}
        Standing::Retired => return Err(AppError::AppRetired { app: id.to_owned() }.into()),
        Standing::Pending | Standing::Declined => {
            return Err(AppError::AppNotApproved { app: id.to_owned() }.into());
        }
    }
    let schema = apps
        .schema(id)
        .cloned()
        .ok_or_else(|| AppError::AppNotApproved { app: id.to_owned() })?;
    let version = app.current().map_or(0, |held| held.version);
    Ok((schema, version))
}

/// Each relation changing `old` to `new` would strand, with its count of
/// standing grants: not revoked and not ended at `at`.
fn strands(book: &GrantBook, old: &AppSchema, new: &AppSchema, at: u64) -> Vec<Strand> {
    let standing = book
        .in_app(old.app())
        .filter(|record| record.revoked().is_none())
        .map(lys_identity::grants::GrantRecord::grant)
        .filter(|grant| grant.window().ends_at().is_none_or(|end| end > at))
        .map(|grant| Held {
            kind: grant.resource().kind(),
            relation: &grant.parts().relation,
            actions: grant.actions(),
            held: grant.mode().is_held(),
            role: grant.names_role(),
        });
    stranded(old, new, standing)
        .into_iter()
        .map(|(named, count)| Strand {
            kind: named.kind,
            relation: named.name,
            count,
        })
        .collect()
}

async fn check(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    body: Result<Json<CheckBody>, JsonRejection>,
) -> Result<Json<SchemaCheck>, ServerError> {
    let Json(body) = body.map_err(|refused| malformed(refused.body_text()))?;
    with_grants(&state, |judged| {
        let who = acting(&state, judged.apps.held(), &headers, judged.directory)?;
        may_change(&who, &id)?;
        let (old, version) = current(judged.apps, &id)?;
        let new = AppSchema::parse(&id, &body.schema).map_err(AppError::from)?;
        let stranded = strands(judged.grants.book(), &old, &new, now());
        let moved = body.replaces.is_some_and(|replaces| replaces != version);
        Ok(Json(SchemaCheck {
            app: id.clone(),
            current: version,
            next: version + 1,
            diff: DiffView::from(&diff(&old, &new)),
            applies: stranded.is_empty() && !moved,
            stranded,
        }))
    })
}

async fn change(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    body: Result<Json<ChangeBody>, JsonRejection>,
) -> Result<Json<SchemaChanged>, ServerError> {
    let Json(body) = body.map_err(|refused| malformed(refused.body_text()))?;
    let operation = OperationId::from_str(&body.operation)?.to_string();
    let answer = crate::grants::with_schema_grants(&state, |judged| {
        let who = acting(&state, judged.apps.held(), &headers, judged.directory)?;
        may_change(&who, &id)?;
        if let Some(kept) = judged.apps.held().operation(&operation) {
            return answer_kept(judged.apps, &id, &operation, &kept);
        }
        let (old, version) = current(judged.apps, &id)?;
        if body.replaces != version {
            return Err(AppError::SchemaVersionMoved {
                replaces: body.replaces,
                current: version,
            }
            .into());
        }
        let new = AppSchema::parse(&id, &body.schema).map_err(AppError::from)?;
        let stranded = strands(judged.grants.book(), &old, &new, now());
        if !stranded.is_empty() {
            return Err(AppError::SchemaChangeStrandsGrants { stranded }.into());
        }
        let change = diff(&old, &new);
        let applied = who.is_administrator();
        let line = if applied {
            Line::Applied(Applied {
                operation,
                app: id.clone(),
                version: version + 1,
                schema: new.to_json(),
                proposal: None,
                by: who.by(),
                at: now(),
            })
        } else {
            Line::Proposed(Proposed {
                operation,
                app: id.clone(),
                replaces: version,
                schema: new.to_json(),
                by: who.by(),
                at: now(),
            })
        };
        judged.apps.keep(line)?;
        Ok(Json(SchemaChanged {
            app: app_view(judged.apps, &id)?,
            applied,
            diff: DiffView::from(&change),
        }))
    })?;
    if answer.applied {
        refresh(&state)?;
    }
    Ok(answer)
}

/// The answer to a change sent again under an operation already kept.
fn answer_kept(
    apps: &AppStore,
    id: &str,
    operation: &str,
    kept: &Line,
) -> Result<Json<SchemaChanged>, ServerError> {
    let applied = match kept {
        Line::Applied(line) if line.app == id => true,
        Line::Proposed(line) if line.app == id => false,
        _ => {
            return Err(AppError::AppOperationReused {
                operation: operation.to_owned(),
            }
            .into());
        }
    };
    Ok(Json(SchemaChanged {
        app: app_view(apps, id)?,
        applied,
        diff: DiffView::from(&lys_identity::grants::SchemaDiff::default()),
    }))
}

fn app_view(apps: &AppStore, id: &str) -> Result<AppView, ServerError> {
    apps.app(id)
        .map(AppView::from)
        .ok_or_else(|| AppError::AppUnknown { app: id.to_owned() }.into())
}

async fn approve(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    body: Result<Json<ChangeDecision>, JsonRejection>,
) -> Result<Json<SchemaChanged>, ServerError> {
    let Json(body) = body.map_err(|refused| malformed(refused.body_text()))?;
    let operation = OperationId::from_str(&body.operation)?.to_string();
    let answer = crate::grants::with_schema_grants(&state, |judged| {
        let who = acting(&state, judged.apps.held(), &headers, judged.directory)?;
        who.administrator()?;
        if let Some(kept) = judged.apps.held().operation(&operation) {
            return answer_kept(judged.apps, &id, &operation, &kept);
        }
        let (old, version) = current(judged.apps, &id)?;
        let pending = judged
            .apps
            .app(&id)
            .and_then(|app| app.pending.clone())
            .ok_or_else(|| AppError::AppDecided { app: id.clone() })?;
        let new = AppSchema::parse(&id, &pending.schema).map_err(AppError::from)?;
        let stranded = strands(judged.grants.book(), &old, &new, now());
        if !stranded.is_empty() {
            return Err(AppError::SchemaChangeStrandsGrants { stranded }.into());
        }
        let change = diff(&old, &new);
        judged.apps.keep(Line::Applied(Applied {
            operation,
            app: id.clone(),
            version: version + 1,
            schema: new.to_json(),
            proposal: Some(pending.operation),
            by: who.by(),
            at: now(),
        }))?;
        Ok(Json(SchemaChanged {
            app: app_view(judged.apps, &id)?,
            applied: true,
            diff: DiffView::from(&change),
        }))
    })?;
    if answer.applied {
        refresh(&state)?;
    }
    Ok(answer)
}

async fn decline(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    body: Result<Json<ChangeDecision>, JsonRejection>,
) -> Result<Json<AppView>, ServerError> {
    let Json(body) = body.map_err(|refused| malformed(refused.body_text()))?;
    let operation = OperationId::from_str(&body.operation)?.to_string();
    crate::apps_api::with_apps(&state, |apps, projection| {
        let who = acting(&state, apps.held(), &headers, projection)?;
        who.administrator()?;
        apps.keep(Line::ChangeDeclined(Decided {
            operation,
            app: id.clone(),
            reason: body.reason.trim().to_owned(),
            by: who.by(),
            at: now(),
        }))?;
        app_view(apps, &id)
    })
    .map(Json)
}

async fn version(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Query(query): Query<VersionQuery>,
) -> Result<Json<SchemaVersionView>, ServerError> {
    crate::apps_api::with_apps(&state, |apps, projection| {
        let who = acting(&state, apps.held(), &headers, projection)?;
        let unknown = || AppError::AppUnknown { app: id.clone() };
        let app = apps.app(&id).ok_or_else(unknown)?;
        let visible = match &who {
            Acting::Administrator(_) => true,
            Acting::App { app: own, .. } => *own == id,
            Acting::Registrar { service_account } => {
                app.registered.by
                    == By::ServiceAccount {
                        id: service_account.clone(),
                    }
            }
            Acting::Person(_) => app.standing() == Standing::Approved,
        };
        if !visible {
            return Err(unknown().into());
        }
        let held = match query.version {
            Some(number) => app.version(number),
            None => app.current(),
        };
        let held = held.ok_or_else(|| AppError::SchemaVersionUnknown {
            app: id.clone(),
            version: query.version.unwrap_or(0),
        })?;
        Ok(SchemaVersionView {
            app: id.clone(),
            version: held.version,
            schema: held.schema.clone(),
        })
    })
    .map(Json)
}

async fn place(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    body: Result<Json<PlaceBody>, JsonRejection>,
) -> Result<Json<Value>, ServerError> {
    let Json(body) = body.map_err(|refused| malformed(refused.body_text()))?;
    let operation = OperationId::from_str(&body.operation)?.to_string();
    let child = Resource::new(&body.child.kind, &body.child.id)?;
    let parent = Resource::new(&body.parent.kind, &body.parent.id)?;
    // Kept under the grants' one hold, after the revision they stand at, so
    // the grant change stream orders it with the grant changes.
    crate::grants::with_schema_grants(&state, |judged| {
        let who = acting(&state, judged.apps.held(), &headers, judged.directory)?;
        may_change(&who, &id)?;
        let apps = judged.apps;
        for kind in [child.kind(), parent.kind()] {
            if owner_of(kind) != id {
                return Err(AppError::NotYourApp {
                    kind: kind.to_owned(),
                    owner: owner_of(kind).to_owned(),
                    acting_for: id.clone(),
                }
                .into());
            }
            apps.admit_kind(Some(&id), kind)?;
        }
        let flows = apps
            .schema(&id)
            .and_then(|schema| schema.kind(child.kind()))
            .is_some_and(|kind| kind.parents.contains(parent.kind()));
        if !flows {
            return Err(AppError::PlacementInvalid {
                reason: format!(
                    "the kind {} does not list {} among its parents",
                    child.kind(),
                    parent.kind()
                ),
            }
            .into());
        }
        // An uncertain grant append is resolved first, so the revision read
        // is the one every change before this placement stands at.
        judged.grants.settle_log()?;
        let revision = match apps.held().operation(&operation) {
            // Sent again, it is the same act: it names the revision it was
            // first kept after.
            Some(Line::Placed(kept)) => kept.revision,
            _ => Some(judged.grants.revision()),
        };
        let kept = apps.keep(Line::Placed(Placed {
            operation,
            app: id.clone(),
            child_kind: child.kind().to_owned(),
            child_id: child.id().to_owned(),
            parent_kind: parent.kind().to_owned(),
            parent_id: parent.id().to_owned(),
            restricted: body.restricted,
            revision,
            by: who.by(),
            at: now(),
        }))?;
        let count = u64::try_from(apps.held().placements.len()).map_err(|error| {
            lys_identity::grants::GrantError::LogUnavailable {
                reason: format!("the placements cannot be counted: {error}"),
            }
        })?;
        state.grant_setup.changes.placed(count);
        // A restricted child is given no parent relationship, so the
        // engine's permissions on the parent never flow to it.
        if !body.restricted
            && let Relationships::SpiceDb(engine) = judged.grants.relationships()
        {
            engine.place(&child, &parent)?;
        }
        Ok(serde_json::json!({
            "placed": {
                "child": child.to_string(),
                "parent": parent.to_string(),
                "restricted": body.restricted,
            },
            "operation": kept.operation(),
        }))
    })
    .map(Json)
}

#[cfg(test)]
#[path = "apps_schema_index_tests.rs"]
mod index_tests;
