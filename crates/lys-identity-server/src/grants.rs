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
//!
//! Road step 2 holds: every check, why and who-can question is answered by
//! `SpiceDB` through the step-2 engine, and a change is answered only once
//! the engine's projection has applied it. A configuration that names no
//! `SpiceDB` gRPC address has no engine, and every grant route is then
//! refused `spicedb_grpc_absent`: no grant is ever decided in this process.
//! The administrator's admission by its configured issuer and subject never
//! asks `SpiceDB`.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, PoisonError};

use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::routing::{get, post};
use axum::{Json, Router};
use lys_identity::grants::{
    ExerciseRequest, GrantError, Grants, MemoryRelationships, Model, Recorded,
};
use lys_identity::projection::Projection;
use lys_identity::signer::load_service_key;
use lys_identity::{IdentityId, PersonId};
use lys_log_store::FileLeafStore;

use crate::error::ServerError;
use crate::grant_contract::{
    ActionBody, DelegateBody, GrantList, GrantView, ModelView, PAGE_MAX, PermitView, RecordedView,
    RevokeBody, RootBody, WhoBody, WhoPage, grant_id,
};
use crate::grant_sight::{as_seen_by, sees, sees_with, visible_or};
use crate::grants_explain::{engine, who_page};
use crate::routes::{AppState, signed_in, with_directory};
use crate::session::now;
use crate::spicedb::engine::Engine;
use crate::spicedb::freshness::Fresh;
use crate::spicedb::{SpiceDbError, SpiceDbSettings};

/// The grants as the service holds them. The in-process relationships keep
/// only the grants' own bookkeeping: no decision is made from them.
pub type GrantState = Grants<FileLeafStore, MemoryRelationships>;

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
    /// The `SpiceDB` the configuration names, shown on the connections
    /// screen; no grant is written or decided through these settings.
    pub spicedb: Option<SpiceDbSettings>,
    /// The step-2 engine, which answers every grant and permission check;
    /// absent when the configuration names no `SpiceDB` gRPC address, and
    /// every grant route is then refused.
    pub engine: Option<Engine>,
    /// Why there is no engine, when there is none: what the configuration
    /// names in place of a `SpiceDB` gRPC address.
    pub engine_absent: String,
}

impl GrantSetup {
    fn open(&self, root_authority: PersonId) -> Result<GrantState, ServerError> {
        if self.engine.is_none() {
            return Err(SpiceDbError::GrpcAbsent {
                reason: self.engine_absent.clone(),
            }
            .into());
        }
        if !self.log_dir.exists() {
            FileLeafStore::create(&self.log_dir, &self.log_origin).map_err(|error| {
                ServerError::ConfigInvalid {
                    reason: format!("the grant log could not be created: {error}"),
                }
            })?;
        }
        let log_dir = self.log_dir.clone();
        Ok(Grants::open(
            Box::new(move || FileLeafStore::open(&log_dir)),
            load_service_key(&self.key_file)?,
            MemoryRelationships::default(),
            self.model.clone(),
            root_authority,
        )?)
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
        .route("/grants/explain", post(crate::grants_explain::explain))
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
        let grants = if let Some(grants) = &mut *slot {
            grants
        } else {
            let opened = state.grant_setup.open(root)?;
            (state.say)(&format!("grant log {}", opened.ledger().start()));
            slot.insert(opened)
        };
        act(Judged {
            directory: projection,
            grants,
            root,
        })
    })
}

/// Answer a recorded change once the step-2 engine's projection has applied
/// it.
fn settled(
    state: &AppState,
    judged: &Judged<'_>,
    recorded: Recorded,
) -> Result<Recorded, GrantError> {
    engine(state)?.settle(judged.grants, judged.root, recorded)
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
        let recorded = settled(&state, &judged, recorded)?;
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
        match judged
            .grants
            .delegate(judged.directory, &request, now())
            .and_then(|recorded| settled(&state, &judged, recorded))
        {
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
        match judged
            .grants
            .revoke(&request, now())
            .and_then(|recorded| settled(&state, &judged, recorded))
        {
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
        let decided = engine(&state).and_then(|engine| {
            let reached = engine.catch_up(judged.grants, judged.root)?;
            let evaluator = Fresh::new(engine.evaluator(), &reached);
            let at = engine.evaluator().now();
            judged
                .grants
                .check_with(judged.directory, &request, &evaluator, at)
        });
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
        let decided = engine(&state).and_then(|engine| {
            let reached = engine.catch_up(judged.grants, judged.root)?;
            let evaluator = Fresh::new(engine.evaluator(), &reached);
            let at = engine.evaluator().now();
            judged
                .grants
                .decide(judged.directory, &request, &evaluator, at)
        });
        match decided {
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
    let (_, resource, action) = body.question.parts()?;
    with_grants(&state, |judged| {
        let caller = caller(&state, &headers, judged.directory)?;
        let question = (&resource, &action);
        Ok(Json(who_page(
            engine(&state)?,
            &judged,
            caller,
            question,
            body.after.as_deref(),
            body.page_size,
        )?))
    })
}
