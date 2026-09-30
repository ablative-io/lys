//! The grant routes: list, read, issue, pass on, revoke, the two questions,
//! why this caller may act and who can, and the delegation form's question,
//! what the caller cannot give a recipient.
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
//! Every route that names a resource kind, issuing, passing on and every
//! check, first asks the apps whether an approved app declares that kind:
//! a kind no approved app declares is refused `kind_not_registered`, one of
//! an app not yet approved `app_not_approved`, and one of a retired app
//! `app_retired`, so a grant on a retired app's kind stays readable and is
//! never exercised. A check on a resource of an app kind also reaches the
//! resources it is placed in whose kinds its schema lists as parents,
//! nearest first, so a relation held on a parent flows to its children.
//!
//! The cannot-give question, `GET /grants/cannot-give`, is asked from a
//! source grant the caller holds, for a recipient the caller may name: any
//! person, or an agent it may see. A source it does not hold is refused
//! exactly as one that does not exist, and a recipient it may not name exactly
//! as one that does not exist, so no refusal says whether either exists. It
//! reads and records nothing.
//!
//! Every grant answered carries whether it stands and its effective end,
//! judged here over its whole chain by the same admission a check runs, so a
//! screen renders the service's judgement and walks no chain of its own. A
//! refusal that stops a grant standing is read as the caller may read it.

use std::collections::{BTreeSet, HashMap};
use std::path::PathBuf;
use std::sync::{Arc, PoisonError, RwLock};

use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::routing::{get, post};
use axum::{Json, Router};
use lys_identity::grants::admission::effective;
use lys_identity::grants::{
    ExerciseRequest, GrantError, GrantRecord, Grants, MemoryRelationships, Model,
};
use lys_identity::projection::Projection;
use lys_identity::signer::load_service_key;
use lys_identity::{IdentityError, IdentityId, PersonId};
use lys_log_store::FileLeafStore;

use crate::apps_store::AppStore;
use crate::error::ServerError;
use crate::grant_contract::{
    ActionBody, DelegateBody, GrantList, GrantView, HolderView, ModelView, PAGE_MAX, PermitView,
    RecordedView, RefusedView, RevokeBody, RootBody, StandingView, WhoBody, WhoPage, grant_id,
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
    /// Lys's own model, the app `lys`'s current schema, as the apps log
    /// last gave it. The log is its only source once the app `lys` exists.
    pub model: RwLock<Model>,
    /// The permission engine the grants are mirrored into, if one is named.
    pub spicedb: Option<SpiceDbSettings>,
}

impl GrantSetup {
    /// Lys's own model as the apps log last gave it.
    pub fn model(&self) -> Model {
        self.model
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// Hold `model` as Lys's own model from now on.
    pub fn hold_model(&self, model: Model) {
        *self.model.write().unwrap_or_else(PoisonError::into_inner) = model;
    }

    fn open(&self, root_authority: PersonId, model: Model) -> Result<GrantState, ServerError> {
        if !self.log_dir.exists() {
            FileLeafStore::create(&self.log_dir, &self.log_origin).map_err(|error| {
                ServerError::ConfigInvalid {
                    reason: format!("the grant log could not be created: {error}"),
                }
            })?;
        }
        let log_dir = self.log_dir.clone();
        let relationships = match &self.spicedb {
            Some(settings) => Relationships::SpiceDb(SpiceDb::open(settings, &model)?),
            None => Relationships::Memory(MemoryRelationships::default()),
        };
        let mut grants = Grants::open(
            Box::new(move || FileLeafStore::open(&log_dir)),
            load_service_key(&self.key_file)?,
            relationships,
            model,
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
        .route("/grants/check/batch", post(crate::grants_batch::batch))
        .route("/grants/which", post(crate::grants_batch::which))
        .route("/grants/why", post(why))
        .route("/grants/who", post(who))
        .route(
            "/grants/cannot-give",
            get(crate::grant_delegation::cannot_give),
        )
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
    /// The apps, whose approved schemas say which kinds are judged at all.
    pub apps: &'a mut AppStore,
}

/// Run `act` with the directory's projection, the apps and the grants, in
/// that lock order, opening the grants on first use under the model the
/// apps log gives.
pub(crate) fn with_grants<T>(
    state: &AppState,
    act: impl FnOnce(Judged<'_>) -> Result<T, ServerError>,
) -> Result<T, ServerError> {
    with_directory(state, |directory| {
        let projection = directory.projection()?;
        let administrator =
            state
                .admission
                .administrator_login()
                .ok_or(ServerError::NotAdmitted {
                    reason: "no administrator is set up yet, so there is no root authority to judge a grant under",
                })?;
        let root = projection
            .person_for(&administrator)
            .ok_or(ServerError::NotAdmitted {
                reason: "the configured administrator's login is bound to no person, so there is no root authority to judge a grant under",
            })?;
        let mut apps = state.apps.lock().unwrap_or_else(PoisonError::into_inner);
        apps.settle()?;
        let mut slot = state.grants.lock().unwrap_or_else(PoisonError::into_inner);
        let grants = if let Some(grants) = &mut *slot {
            grants
        } else {
            let opened = state.grant_setup.open(root, apps.model()?)?;
            (state.say)(&format!("grant log {}", opened.ledger().start()));
            slot.insert(opened)
        };
        act(Judged {
            directory: projection,
            grants,
            root,
            apps: &mut apps,
        })
    })
}

/// Refuse unless the permission engine, where one is named, gives the caller
/// the action on the resource. The grants' own decision is made first and
/// nothing is recorded, so a refusal the grants name is answered by its name.
pub(crate) fn engine_permits(
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

/// The grant a refusal names on the chain it was judged over, if any.
fn named_grant(error: &GrantError) -> Option<String> {
    match error {
        GrantError::Revoked { grant }
        | GrantError::Expired { grant, .. }
        | GrantError::NotStarted { grant, .. }
        | GrantError::OperationUnresolved { grant, .. } => Some(grant.clone()),
        _ => None,
    }
}

/// `record` as `caller` reads it at `at`: whether it stands, judged by
/// admission over its whole chain, and the earliest end on that chain.
pub(crate) fn grant_view(
    judged: &Judged<'_>,
    caller: IdentityId,
    record: &GrantRecord,
    at: u64,
) -> GrantView {
    let id = record.grant().id();
    let book = judged.grants.book();
    let effective_ends_at = book
        .lineage(id)
        .ok()
        .and_then(|lineage| lineage.ends)
        .map(|(ends, _)| ends);
    let standing = match effective(book, judged.directory, id, at) {
        Ok(_) => StandingView {
            stands: true,
            refused: None,
        },
        Err(error) => {
            let named = named_grant(&error);
            let seen = as_seen_by(judged, caller, error);
            let grant = if matches!(seen, ServerError::Withheld { .. }) {
                None
            } else {
                named
            };
            StandingView {
                stands: false,
                refused: Some(RefusedView {
                    refusal: seen.name(),
                    grant,
                    reason: seen.to_string(),
                }),
            }
        }
    };
    GrantView::new(
        record,
        judged.grants.unreported(id),
        standing,
        effective_ends_at,
    )
}

/// Whether a decision is an exercise, recorded as a use, or a question.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Decision {
    /// The caller is about to act: a permit is recorded as a use.
    Exercise,
    /// A question: nothing is recorded.
    Explain,
}

/// The grants' decision on `request`, reaching from its resource to each
/// parent it is placed in that its schema lists, nearest first, answered
/// with the permit and the resource whose grant permits it. The first
/// resource's refusal is the one answered when none permits.
pub(crate) fn decide(
    judged: &mut Judged<'_>,
    request: &ExerciseRequest,
    at: u64,
    at_least: Option<u64>,
    decision: Decision,
) -> Result<(lys_identity::grants::Permit, lys_identity::grants::Resource), GrantError> {
    let mut first = None;
    for resource in judged.apps.reach(&request.resource) {
        let asked = ExerciseRequest {
            resource: resource.clone(),
            ..request.clone()
        };
        let decided =
            engine_permits(judged.grants, judged.directory, &asked, at).and_then(
                |()| match decision {
                    Decision::Exercise => {
                        judged.grants.check(judged.directory, &asked, at, at_least)
                    }
                    Decision::Explain => {
                        judged
                            .grants
                            .explain(judged.directory, &asked, at, at_least)
                    }
                },
            );
        match decided {
            Ok(permit) => return Ok((permit, resource)),
            Err(error) => {
                first.get_or_insert(error);
            }
        }
    }
    Err(first.unwrap_or_else(|| GrantError::NotHeld {
        identity: request.caller.to_string(),
        resource: request.resource.to_string(),
        action: request.action.to_string(),
    }))
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
        let at = now();
        let mut known = HashMap::new();
        let grants = judged
            .grants
            .book()
            .records()
            .filter(|record| sees_with(&judged, caller, record, &mut known))
            .map(|record| grant_view(&judged, caller, record, at))
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
    Ok(Json(ModelView::from(&state.grant_setup.model())))
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
        Ok(Json(grant_view(&judged, caller, record, now())))
    })
}

async fn issue_root(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<RootBody>,
) -> Result<Json<RecordedView>, ServerError> {
    with_grants(&state, |judged| {
        let request = body.request(caller(&state, &headers, judged.directory)?)?;
        judged.apps.admit_kind(None, request.resource.kind())?;
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
        judged.apps.admit_kind(None, request.resource.kind())?;
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
    with_grants(&state, |mut judged| {
        let caller = caller(&state, &headers, judged.directory)?;
        judged.apps.admit_kind(None, resource.kind())?;
        judged.apps.admit_action(resource.kind(), action.as_str())?;
        let request = ExerciseRequest {
            caller,
            route,
            resource,
            action,
        };
        match decide(&mut judged, &request, now(), None, Decision::Exercise) {
            Ok((permit, _)) => Ok(Json(PermitView::from(&permit))),
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
    with_grants(&state, |mut judged| {
        let caller = caller(&state, &headers, judged.directory)?;
        judged.apps.admit_kind(None, resource.kind())?;
        judged.apps.admit_action(resource.kind(), action.as_str())?;
        let request = ExerciseRequest {
            caller,
            route,
            resource,
            action,
        };
        match decide(&mut judged, &request, now(), None, Decision::Explain) {
            Ok((permit, _)) => Ok(Json(PermitView::from(&permit))),
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
        judged.apps.admit_kind(None, resource.kind())?;
        judged.apps.admit_action(resource.kind(), action.as_str())?;
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
