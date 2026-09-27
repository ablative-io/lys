//! The grant routes: list, read, issue, pass on, revoke, and the two
//! questions, why this caller may act and who can.
//!
//! Every route calls the grants' one authority owner through [`with_grants`],
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

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::{Arc, PoisonError};

use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::routing::{get, post};
use axum::{Json, Router};
use lys_identity::grants::lineage::MAX_DEPTH;
use lys_identity::grants::{
    ExerciseRequest, GrantError, GrantId, GrantRecord, Grants, MemoryRelationships, Model, Source,
};
use lys_identity::projection::Projection;
use lys_identity::signer::load_service_key;
use lys_identity::{IdentityError, IdentityId, PersonId};
use lys_log_store::FileLeafStore;
use serde_json::{Value, json};

use crate::error::ServerError;
use crate::grant_contract::{
    ActionBody, DelegateBody, PAGE_MAX, RevokeBody, RootBody, WhoBody, grant_id, grant_json,
    permit_json, recorded_json,
};
use crate::routes::{AppState, signed_in, with_directory};
use crate::session::now;

/// The grants as the service holds them.
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
        .route("/grants/roots", post(issue_root))
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

/// The identity the signed-in caller's login is bound to, person or agent.
fn caller(
    state: &AppState,
    headers: &HeaderMap,
    directory: &Projection,
) -> Result<IdentityId, ServerError> {
    let actor = signed_in(state, headers)?;
    if let Some(person) = directory.person_for(actor.binding()) {
        return Ok(IdentityId::Person(person));
    }
    directory
        .records()
        .find(|(id, record)| {
            matches!(id, IdentityId::Agent(_)) && record.bindings().contains(actor.binding())
        })
        .map(|(id, _)| *id)
        .ok_or(ServerError::NoPerson)
}

fn is_root(caller: IdentityId, root: PersonId) -> bool {
    caller == IdentityId::Person(root)
}

/// Whether `caller` may see the grant `record`.
fn sees(judged: &Judged<'_>, caller: IdentityId, record: &GrantRecord) -> bool {
    if is_root(caller, judged.root) {
        return true;
    }
    let book = judged.grants.book();
    let mut next = Some(record.grant());
    let mut hops = 0;
    while let Some(grant) = next {
        let parts = grant.parts();
        if parts.holder == caller
            || parts.issuer == caller
            || IdentityId::Person(parts.responsible) == caller
        {
            return true;
        }
        hops += 1;
        next = match parts.source {
            Source::Grant(id) if hops <= MAX_DEPTH => book.grant(id),
            Source::Grant(_) | Source::Root => None,
        };
    }
    false
}

/// Whether `caller` may see `identity`'s records: its own, an agent it answers for, or any as the root authority.
fn sees_identity(judged: &Judged<'_>, caller: IdentityId, identity: IdentityId) -> bool {
    identity == caller
        || is_root(caller, judged.root)
        || judged
            .directory
            .record(identity)
            .and_then(lys_identity::projection::Record::responsible)
            .is_some_and(|person| IdentityId::Person(person) == caller)
}

/// Whether `id` is on the authority path of a grant `caller` holds, where a
/// why-permitted answer already names it.
fn on_callers_path(judged: &Judged<'_>, caller: IdentityId, id: GrantId) -> bool {
    let book = judged.grants.book();
    book.held_by(caller).any(|record| {
        let mut next = Some(record.grant());
        let mut hops = 0;
        while let Some(grant) = next {
            if grant.id() == id {
                return true;
            }
            hops += 1;
            next = match grant.source() {
                Source::Grant(source) if hops <= MAX_DEPTH => book.grant(source),
                Source::Grant(_) | Source::Root => None,
            };
        }
        false
    })
}

/// Whether a refusal may name the grant `text` to `caller`.
fn grant_seen(judged: &Judged<'_>, caller: IdentityId, text: &str) -> bool {
    GrantId::from_str(text).ok().is_some_and(|id| {
        on_callers_path(judged, caller, id)
            || judged
                .grants
                .book()
                .record(id)
                .is_some_and(|record| sees(judged, caller, record))
    })
}

/// Refuse a grant `caller` may not see exactly as a grant the grants do not hold.
fn visible_or(
    judged: &Judged<'_>,
    caller: IdentityId,
    id: GrantId,
    unknown: GrantError,
) -> Result<(), ServerError> {
    match judged.grants.book().record(id) {
        Some(record) if !sees(judged, caller, record) && !on_callers_path(judged, caller, id) => {
            Err(unknown.into())
        }
        Some(_) | None => Ok(()),
    }
}

fn identity_seen(judged: &Judged<'_>, caller: IdentityId, text: &str) -> bool {
    crate::routes::identity_id(text).is_ok_and(|identity| sees_identity(judged, caller, identity))
}

/// The refusal as `caller` may read it: whole when every grant and identity it
/// names is one the caller may see, else its name and the condition alone.
fn as_seen_by(judged: &Judged<'_>, caller: IdentityId, error: GrantError) -> ServerError {
    let hidden = match &error {
        GrantError::Revoked { grant }
        | GrantError::Expired { grant, .. }
        | GrantError::NotStarted { grant, .. }
        | GrantError::OperationUnresolved { grant, .. } => !grant_seen(judged, caller, grant),
        GrantError::IdentityNotActive { identity, .. }
        | GrantError::ResponsibleMismatch { identity, .. }
        | GrantError::Identity(IdentityError::IdentityUnknown { identity }) => {
            !identity_seen(judged, caller, identity)
        }
        _ => false,
    };
    if hidden {
        ServerError::Withheld {
            refusal: ServerError::from(error).name(),
        }
    } else {
        error.into()
    }
}

async fn list(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Value>, ServerError> {
    with_grants(&state, |judged| {
        let caller = caller(&state, &headers, judged.directory)?;
        let grants = judged
            .grants
            .book()
            .records()
            .filter(|record| sees(&judged, caller, record))
            .map(grant_json)
            .collect::<Vec<_>>();
        Ok(Json(
            json!({ "grants": grants, "revision": judged.grants.revision() }),
        ))
    })
}

async fn read(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<Value>, ServerError> {
    let id = grant_id(&id)?;
    with_grants(&state, |judged| {
        let caller = caller(&state, &headers, judged.directory)?;
        let record = judged
            .grants
            .book()
            .record(id)
            .filter(|record| sees(&judged, caller, record))
            .ok_or(ServerError::GrantNotVisible)?;
        Ok(Json(grant_json(record)))
    })
}

async fn issue_root(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<RootBody>,
) -> Result<Json<Value>, ServerError> {
    with_grants(&state, |judged| {
        let request = body.request(caller(&state, &headers, judged.directory)?)?;
        let recorded = judged
            .grants
            .issue_root(judged.directory, &request, now())?;
        Ok(Json(recorded_json(&recorded)))
    })
}

async fn delegate(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<DelegateBody>,
) -> Result<Json<Value>, ServerError> {
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
            Ok(recorded) => Ok(Json(recorded_json(&recorded))),
            Err(error) => Err(as_seen_by(&judged, caller, error)),
        }
    })
}

async fn revoke(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<RevokeBody>,
) -> Result<Json<Value>, ServerError> {
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
            Ok(recorded) => Ok(Json(recorded_json(&recorded))),
            Err(error) => Err(as_seen_by(&judged, caller, error)),
        }
    })
}

async fn why(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<ActionBody>,
) -> Result<Json<Value>, ServerError> {
    let (route, resource, action) = body.parts()?;
    with_grants(&state, |judged| {
        let caller = caller(&state, &headers, judged.directory)?;
        let request = ExerciseRequest {
            caller,
            route,
            resource,
            action,
        };
        match judged.grants.check(judged.directory, &request, now(), None) {
            Ok(permit) => Ok(Json(permit_json(&permit))),
            Err(error) => Err(as_seen_by(&judged, caller, error)),
        }
    })
}

async fn who(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<WhoBody>,
) -> Result<Json<Value>, ServerError> {
    if body.page_size == 0 || body.page_size > PAGE_MAX {
        return Err(ServerError::RequestMalformed {
            reason: format!("page_size is 1 to {PAGE_MAX}"),
        });
    }
    let (route, resource, action) = body.question.parts()?;
    let at = now();
    with_grants(&state, |judged| {
        let caller = caller(&state, &headers, judged.directory)?;
        let holders: BTreeSet<IdentityId> = judged
            .grants
            .book()
            .records()
            .filter(|record| {
                record.grant().resource() == &resource
                    && record.grant().actions().contains(&action)
                    && sees(&judged, caller, record)
            })
            .map(|record| record.grant().holder())
            .collect();
        let after = body.after.as_deref();
        let mut ordered: Vec<(String, IdentityId)> = holders
            .into_iter()
            .map(|holder| (holder.to_string(), holder))
            .filter(|(text, _)| after.is_none_or(|after| text.as_str() > after))
            .collect();
        ordered.sort();
        let mut page = Vec::new();
        let mut revision = judged.grants.revision();
        let mut remaining = ordered.into_iter();
        for (text, holder) in remaining.by_ref() {
            let request = ExerciseRequest {
                caller: holder,
                route,
                resource: resource.clone(),
                action: action.clone(),
            };
            if let Ok(permit) = judged.grants.check(judged.directory, &request, at, None) {
                revision = permit.revision;
                let mut answer = permit_json(&permit);
                answer["holder"] = Value::String(text);
                page.push(answer);
                if page.len() == body.page_size {
                    break;
                }
            }
        }
        let next = if remaining.next().is_some() {
            page.last()
                .and_then(|last| last["holder"].as_str().map(str::to_owned))
        } else {
            None
        };
        Ok(Json(json!({
            "holders": page,
            "revision": revision,
            "complete": next.is_none(),
            "next": next,
        })))
    })
}
