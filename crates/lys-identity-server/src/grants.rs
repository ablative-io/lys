//! The grant routes: list, read, issue, pass on, revoke, and the two
//! questions, why this caller may act and who can.
//!
//! Every route calls the grants' one authority owner through `with_grants`,
//! which takes the directory lock first and the grants' second, so every
//! decision reads one directory projection and one grant revision. Browser,
//! API and tool requests reach the same owner and the same checks; the route a
//! body names is recorded and never changes a decision.
//!
//! The root authority is the person the configured administrator's login is
//! bound to. The grants are opened on first use, once that person exists, so
//! no second setting can name a different person. A caller sees a grant when
//! it holds, issued or answers for it, when it holds a grant the grant derives
//! from, or when it is the root authority. A refusal that would name a grant
//! or identity the caller may not see is answered with its name and without
//! the record.

use std::collections::{BTreeSet, HashMap};
use std::path::PathBuf;
use std::sync::{Arc, PoisonError};

use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::routing::{get, post};
use axum::{Json, Router};
use lys_identity::grants::{ExerciseRequest, GrantError, Grants, MemoryRelationships, Model};
use lys_identity::projection::Projection;
use lys_identity::signer::load_service_key;
use lys_identity::{IdentityId, PersonId};
use lys_log_store::FileLeafStore;

use crate::error::ServerError;
use crate::grant_contract::{
    ActionBody, DelegateBody, GrantList, GrantView, HolderView, ModelView, PAGE_MAX, PermitView,
    RecordedView, RevokeBody, RootBody, WhoBody, WhoPage, grant_id,
};
use crate::grant_sight::{as_seen_by, sees, sees_with, visible_or};
use crate::routes::{AppState, signed_in, with_directory};
use crate::session::now;
use crate::spicedb::{Relationships, SpiceDb, SpiceDbSettings};

/// The grants as the service holds them.
pub type GrantState = Grants<FileLeafStore, Relationships>;

/// What the grants are opened from.
pub struct GrantSetup {
    /// The directory the grant log is kept in.
    pub log_dir: PathBuf,
    /// The grant log's origin, used when the log is created.
    pub log_origin: String,
    /// The file holding the service's event signing key seed.
    pub key_file: PathBuf,
    /// The model grants are judged against.
    pub model: Model,
    /// The permission engine the grants are mirrored into, if one is named.
    pub spicedb: Option<SpiceDbSettings>,
}

impl GrantSetup {
    fn open(&self, root_authority: PersonId) -> Result<GrantState, ServerError> {
        if !self.log_dir.exists() {
            FileLeafStore::create(&self.log_dir, &self.log_origin).map_err(|error| {
                ServerError::ConfigInvalid {
                    reason: format!("the grant log could not be created: {error}"),
                }
            })?;
        }
        let log_dir = self.log_dir.clone();
        let relationships = match &self.spicedb {
            Some(settings) => Relationships::SpiceDb(SpiceDb::open(settings, &self.model)?),
            None => Relationships::Memory(MemoryRelationships::default()),
        };
        let mut grants = Grants::open(
            Box::new(move || FileLeafStore::open(&log_dir)),
            load_service_key(&self.key_file)?,
            relationships,
            self.model.clone(),
            root_authority,
        )?;
        if self.spicedb.is_some() {
            grants.project()?;
        }
        Ok(grants)
    }
}

/// The grant routes.
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/grants", get(list).post(delegate))
        .route("/grants/model", get(model))
        .route("/grants/roots", post(issue_root))
        .route("/grants/check", post(check))
        .route("/grants/why", post(why))
        .route("/grants/who", post(who))
        .route("/grants/{id}", get(read))
        .route("/grants/{id}/revoke", post(revoke))
}

/// What one grant request is judged with.
pub struct Judged<'a> {
    /// The directory's projection.
    pub directory: &'a Projection,
    /// The grants.
    pub grants: &'a mut GrantState,
    /// The root authority.
    pub root: PersonId,
}

/// Run `act` with the directory's projection and the grants, directory lock
/// first, opening the grants on first use.
pub(crate) fn with_grants<T>(
    state: &AppState,
    act: impl FnOnce(Judged<'_>) -> Result<T, ServerError>,
) -> Result<T, ServerError> {
    with_directory(state, |directory| {
        let projection = directory.projection()?;
        let root = projection
            .person_for(state.admission.administrator_login())
            .ok_or(ServerError::NotAdmitted {
                reason: "the configured administrator's login is bound to no person, so there is no root authority to judge a grant under",
            })?;
        let mut slot = state.grants.lock().unwrap_or_else(PoisonError::into_inner);
        let grants = match &mut *slot {
            Some(grants) => grants,
            None => slot.insert(state.grant_setup.open(root)?),
        };
        act(Judged {
            directory: projection,
            grants,
            root,
        })
    })
}

/// Refuse unless the permission engine, where one is named, gives the caller
/// the action on the resource. The grants' own decision is made first and
/// nothing is recorded, so a refusal the grants name is answered by its name.
fn engine_permits(
    grants: &mut GrantState,
    directory: &Projection,
    request: &ExerciseRequest,
    at: u64,
) -> Result<(), GrantError> {
    if matches!(grants.relationships(), Relationships::Memory(_)) {
        return Ok(());
    }
    let permit = grants.explain(directory, request, at, None)?;
    let Relationships::SpiceDb(engine) = grants.relationships() else {
        return Ok(());
    };
    if engine.check(&request.resource, &request.action, request.caller, at)? {
        return Ok(());
    }
    Err(GrantError::PermissionAbsent {
        grant: permit.grant.to_string(),
    })
}

/// The identity the signed-in caller's login is bound to, person or agent.
pub(crate) fn caller(
    state: &AppState,
    headers: &HeaderMap,
    directory: &Projection,
) -> Result<IdentityId, ServerError> {
    let actor = signed_in(state, headers)?;
    if let Some(person) = directory.person_for(actor.binding()) {
        return Ok(IdentityId::Person(person));
    }
    directory
        .agent_for(actor.binding())
        .map(IdentityId::Agent)
        .ok_or(ServerError::NoPerson)
}

async fn list(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<GrantList>, ServerError> {
    with_grants(&state, |judged| {
        let caller = caller(&state, &headers, judged.directory)?;
        let mut known = HashMap::new();
        let grants = judged
            .grants
            .book()
            .records()
            .filter(|record| sees_with(&judged, caller, record, &mut known))
            .map(GrantView::from)
            .collect();
        Ok(Json(GrantList {
            grants,
            revision: judged.grants.revision(),
        }))
    })
}

/// The permission model, so a screen offers only the relations it defines.
async fn model(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<ModelView>, ServerError> {
    signed_in(&state, &headers)?;
    Ok(Json(ModelView::from(&state.grant_setup.model)))
}

async fn read(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<GrantView>, ServerError> {
    let id = grant_id(&id)?;
    with_grants(&state, |judged| {
        let caller = caller(&state, &headers, judged.directory)?;
        let record = judged
            .grants
            .book()
            .record(id)
            .filter(|record| sees(&judged, caller, record))
            .ok_or(ServerError::GrantNotVisible)?;
        Ok(Json(GrantView::from(record)))
    })
}

async fn issue_root(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<RootBody>,
) -> Result<Json<RecordedView>, ServerError> {
    with_grants(&state, |judged| {
        let request = body.request(caller(&state, &headers, judged.directory)?)?;
        let recorded = judged
            .grants
            .issue_root(judged.directory, &request, now())?;
        Ok(Json(RecordedView::from(&recorded)))
    })
}

async fn delegate(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<DelegateBody>,
) -> Result<Json<RecordedView>, ServerError> {
    with_grants(&state, |judged| {
        let caller = caller(&state, &headers, judged.directory)?;
        let request = body.request(caller)?;
        visible_or(
            &judged,
            caller,
            request.source,
            GrantError::SourceUnknown {
                grant: request.source.to_string(),
            },
        )?;
        match judged.grants.delegate(judged.directory, &request, now()) {
            Ok(recorded) => Ok(Json(RecordedView::from(&recorded))),
            Err(error) => Err(as_seen_by(&judged, caller, error)),
        }
    })
}

async fn revoke(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<RevokeBody>,
) -> Result<Json<RecordedView>, ServerError> {
    let id = grant_id(&id)?;
    with_grants(&state, |judged| {
        let caller = caller(&state, &headers, judged.directory)?;
        let request = body.request(caller, id)?;
        visible_or(
            &judged,
            caller,
            id,
            GrantError::GrantUnknown {
                grant: id.to_string(),
            },
        )?;
        match judged.grants.revoke(&request, now()) {
            Ok(recorded) => Ok(Json(RecordedView::from(&recorded))),
            Err(error) => Err(as_seen_by(&judged, caller, error)),
        }
    })
}

/// The enforcement point: the caller is about to take the action, and a
/// permitted check records the use.
async fn check(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<ActionBody>,
) -> Result<Json<PermitView>, ServerError> {
    let (route, resource, action) = body.parts()?;
    with_grants(&state, |judged| {
        let caller = caller(&state, &headers, judged.directory)?;
        let request = ExerciseRequest {
            caller,
            route,
            resource,
            action,
        };
        let at = now();
        let decided = engine_permits(&mut *judged.grants, judged.directory, &request, at)
            .and_then(|()| judged.grants.check(judged.directory, &request, at, None));
        match decided {
            Ok(permit) => Ok(Json(PermitView::from(&permit))),
            Err(error) => Err(as_seen_by(&judged, caller, error)),
        }
    })
}

async fn why(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<ActionBody>,
) -> Result<Json<PermitView>, ServerError> {
    let (route, resource, action) = body.parts()?;
    with_grants(&state, |judged| {
        let caller = caller(&state, &headers, judged.directory)?;
        let request = ExerciseRequest {
            caller,
            route,
            resource,
            action,
        };
        match judged
            .grants
            .explain(judged.directory, &request, now(), None)
        {
            Ok(permit) => Ok(Json(PermitView::from(&permit))),
            Err(error) => Err(as_seen_by(&judged, caller, error)),
        }
    })
}

async fn who(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<WhoBody>,
) -> Result<Json<WhoPage>, ServerError> {
    if body.page_size == 0 || body.page_size > PAGE_MAX {
        return Err(ServerError::RequestMalformed {
            reason: format!("page_size is 1 to {PAGE_MAX}"),
        });
    }
    let (route, resource, action) = body.question.parts()?;
    let at = now();
    with_grants(&state, |judged| {
        let caller = caller(&state, &headers, judged.directory)?;
        let after = body.after.as_deref();
        let mut known = HashMap::new();
        let holders: BTreeSet<(String, IdentityId)> = judged
            .grants
            .book()
            .on_resource(&resource)
            .filter(|record| {
                record.grant().actions().contains(&action)
                    && sees_with(&judged, caller, record, &mut known)
            })
            .map(|record| {
                let holder = record.grant().holder();
                (holder.to_string(), holder)
            })
            .filter(|(text, _)| after.is_none_or(|after| text.as_str() > after))
            .collect();
        // A holder is on a page only when its own grant permits the action,
        // and the page names a next holder only when a later holder permits.
        let (page, more) = {
            let mut permitted = holders.into_iter().filter_map(|(text, holder)| {
                let request = ExerciseRequest {
                    caller: holder,
                    route,
                    resource: resource.clone(),
                    action: action.clone(),
                };
                judged
                    .grants
                    .explain(judged.directory, &request, at, None)
                    .ok()
                    .map(|permit| HolderView {
                        holder: text,
                        permit: PermitView::from(&permit),
                    })
            });
            let page: Vec<HolderView> = permitted.by_ref().take(body.page_size).collect();
            let more = page.len() == body.page_size && permitted.next().is_some();
            (page, more)
        };
        let next = if more {
            page.last().map(|last| last.holder.clone())
        } else {
            None
        };
        Ok(Json(WhoPage {
            holders: page,
            revision: judged.grants.revision(),
            complete: next.is_none(),
            next,
        }))
    })
}
