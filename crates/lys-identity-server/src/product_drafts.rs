//! Product drafts (ACCESS-001 R3, route note option A): a product whose
//! grant holds an act by draft or by two records the act's exact words at
//! `POST /product-drafts`; people approve or refuse it; the app's connector
//! reads the approved drafts of its own kinds at `GET /product-drafts` and
//! closes each once, executed or refused on execution. Lys never executes
//! the act and never calls the product: the product reads Lys (ADR-116).
//!
//! The wire is lys-pass's own (`crates/lys-pass/src/drafts.rs`): the body
//! of a creation is its `DraftRequest`, the approved page its `ApprovedPage`.
//! The operator drafts at `/drafts` are a different thing and unchanged.

use std::str::FromStr;
use std::sync::Arc;

use axum::extract::rejection::JsonRejection;
use axum::extract::{Query, State};
use axum::http::{HeaderMap, header};
use axum::routing::{get, post};
use axum::{Json, Router};
use lys_identity::IdentityId;
use lys_identity::draft_event::Target;
use lys_identity::grants::{Action, ExerciseRequest, GrantId, Resource, Route};
use lys_identity::product_draft_event::{Created, ProductDraftEvent, draft_operation};
use lys_identity::projection::product_draft::ProductDraftRecord;
use lys_log_store::FileLeafStore;

use crate::apps_binding::{Acting, acting};
use crate::apps_error::AppError;
use crate::error::ServerError;
use crate::error_product_draft::ProductDraftError;
use crate::grants::{Judged, with_directory_grants};
use crate::routes::{AppState, hex, with_directory};
use crate::session::now;

#[path = "product_drafts_decide.rs"]
mod decide;
#[path = "product_drafts_list.rs"]
mod listing;
#[path = "product_drafts_wire.rs"]
pub mod wire;

use wire::{ApprovedDraftView, ApprovedPage, ApprovedQuery, ProductDraftBody, ProductDraftCreated};

/// The product draft routes.
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/product-drafts", get(list).post(create))
        .route("/product-drafts/{id}/approve", post(decide::approve))
        .route("/product-drafts/{id}/refuse", post(decide::refuse))
        .route("/product-drafts/{id}/executed", post(decide::executed))
        .route(
            "/product-drafts/{id}/refused-on-execution",
            post(decide::refused_on_execution),
        )
}

pub(crate) use wire::typed;

fn malformed(reason: impl Into<String>) -> ServerError {
    ServerError::RequestMalformed {
        reason: reason.into(),
    }
}

fn grant_refused(reason: impl Into<String>) -> ServerError {
    ProductDraftError::GrantRefused {
        reason: reason.into(),
    }
    .into()
}

/// A 32-byte digest from its 64 hex digits, either case.
pub(crate) fn digest32(text: &str, what: &str) -> Result<[u8; 32], ServerError> {
    let refused = || malformed(format!("{what} is a SHA-256 digest of 64 hex digits"));
    if text.len() != 64 || !text.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(refused());
    }
    let mut out = [0u8; 32];
    for (slot, pair) in out.iter_mut().zip(text.as_bytes().chunks_exact(2)) {
        let pair = std::str::from_utf8(pair).map_err(|_unreadable| refused())?;
        *slot = u8::from_str_radix(pair, 16).map_err(|_unreadable| refused())?;
    }
    Ok(out)
}

/// The app whose kind `kind` is: the part before its first dot.
pub(crate) fn app_of(kind: &str) -> Option<&str> {
    kind.split_once('.')
        .map(|(app, _)| app)
        .filter(|app| !app.is_empty())
}

/// The identity a product draft is asked by: the holder through the pass
/// Lys issued it for the target's app (`passed`, verified before any lock
/// is taken), an app's connector through the app's credential, or else the
/// signed-in caller (a person, or an agent through its run pass). A bearer
/// that does not verify is refused by name, never passed over for a session.
fn holder(
    state: &AppState,
    headers: &HeaderMap,
    judged: &Judged<'_>,
    passed: Option<IdentityId>,
) -> Result<IdentityId, ServerError> {
    if let Some(holder) = passed {
        return Ok(holder);
    }
    if !headers.contains_key(header::AUTHORIZATION) {
        return crate::grants::caller(state, headers, judged.directory);
    }
    match acting(state, judged.apps.held(), headers, judged.directory)? {
        Acting::App { app, .. } => connector_of(judged.apps.held(), &app),
        Acting::Administrator(_) | Acting::Person(_) | Acting::Registrar { .. } => {
            Err(ServerError::NotAdmitted {
                reason: "a product draft is asked by the grant's holder or an app's connector",
            })
        }
    }
}

fn connector_of(held: &crate::apps_state::Held, app: &str) -> Result<IdentityId, ServerError> {
    let line = held
        .app(app)
        .and_then(|found| found.connector.as_ref())
        .ok_or(ServerError::NotAdmitted {
            reason: "the app holds no connector, so it holds no grant",
        })?;
    Ok(IdentityId::Connector(lys_identity::ConnectorId::from_str(
        &line.connector,
    )?))
}

/// Whether grant `grant` alone lets `caller` take `action` on `resource` or
/// on a parent it is placed in: the grant is live, its holder is the caller
/// and its reach covers the target. Nothing is recorded.
pub(crate) fn reaches(
    judged: &mut Judged<'_>,
    caller: IdentityId,
    grant: GrantId,
    (resource, action): (&Resource, &Action),
    at: u64,
) -> bool {
    judged.apps.reach(resource).into_iter().any(|reached| {
        let asked = ExerciseRequest {
            caller,
            route: Route::Api,
            resource: reached,
            action: action.clone(),
        };
        judged
            .grants
            .explain_by(judged.directory, &asked, Some(grant), at, None)
            .is_ok_and(|permit| permit.grant == grant)
    })
}

/// The creation `body` asks for, judged against the caller's held grant.
fn judge(
    state: &AppState,
    headers: &HeaderMap,
    mut judged: Judged<'_>,
    body: ProductDraftBody,
    passed: Option<IdentityId>,
) -> Result<Created, ServerError> {
    let caller = holder(state, headers, &judged, passed)?;
    if body.operation.is_empty() || body.operation.chars().any(char::is_control) {
        return Err(malformed("operation is the product's own non-empty id"));
    }
    let grant_id = GrantId::from_str(&body.grant)?;
    let grant = judged
        .grants
        .book()
        .grant(grant_id)
        .cloned()
        .ok_or_else(|| grant_refused(format!("no grant `{grant_id}` is held")))?;
    if grant.holder() != caller {
        return Err(grant_refused(format!(
            "grant `{grant_id}` is held by another identity, not the caller"
        )));
    }
    if !grant.mode().is_held() {
        return Err(grant_refused(format!(
            "grant `{grant_id}` is outright, so its act is taken at once and never drafted"
        )));
    }
    let target = body.target;
    judged.apps.admit_kind(None, &target.kind)?;
    judged.apps.admit_action(&target.kind, &target.action)?;
    let app = app_of(&target.kind)
        .ok_or_else(|| malformed("the target names a kind of an app, `app.kind`"))?
        .to_owned();
    let resource = Resource::new(&target.kind, &target.id)?;
    let action = Action::new(&target.action)?;
    let at = now();
    if !reaches(&mut judged, caller, grant_id, (&resource, &action), at) {
        return Err(grant_refused(format!(
            "grant `{grant_id}` is not live or does not reach {}:{} for `{}`",
            target.kind, target.id, target.action
        )));
    }
    let request_digest = digest32(&body.request_digest, "request_digest")
        .map_err(|_malformed| ProductDraftError::DigestMismatch)?;
    if lys_identity::product_draft_event::words_digest(&body.words) != request_digest {
        return Err(ProductDraftError::DigestMismatch.into());
    }
    Ok(Created {
        operation: draft_operation(caller, &body.operation),
        client_operation: body.operation,
        holder: caller,
        responsible: grant.responsible(),
        recorded_at: at,
        app,
        grant: grant_id,
        mode: grant.mode(),
        target: Target {
            kind: target.kind,
            id: target.id,
            action: target.action,
        },
        request_digest,
        words: body.words,
    })
}

/// Record a product's held act: the same operation and request answer the
/// same draft; the operation asked again with another request is refused
/// `OperationReused`.
async fn create(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Result<Json<ProductDraftBody>, JsonRejection>,
) -> Result<Json<ProductDraftCreated>, ServerError> {
    let Json(body) = body.map_err(|refused| malformed(refused.body_text()))?;
    // A holder's pass is verified for the target's app before any lock.
    let passed = match crate::provider::presented_pass(&headers) {
        Some(pass) => {
            let app = app_of(&body.target.kind)
                .ok_or_else(|| malformed("the target names a kind of an app, `app.kind`"))?;
            Some(crate::provider::pass_holder(&state, pass, app)?)
        }
        None => None,
    };
    with_directory_grants(
        &state,
        |judged| judge(&state, &headers, judged, body, passed),
        |directory, created| {
            let draft = created.operation;
            directory.record_product_draft(ProductDraftEvent::Created(Arc::new(created)))?;
            Ok(Json(ProductDraftCreated {
                draft: draft.to_string(),
            }))
        },
    )
}

/// Run `act` as the app whose connector credential the request carries,
/// with the directory held; any other caller is refused `NotAdmitted`.
pub(crate) fn with_connector<T>(
    state: &AppState,
    headers: &HeaderMap,
    act: impl FnOnce(&str, &mut lys_identity::Directory<FileLeafStore>) -> Result<T, ServerError>,
) -> Result<T, ServerError> {
    with_directory(state, |directory| {
        let app = {
            let mut apps = state
                .apps
                .lock()
                .map_err(|error| AppError::AppsUnavailable {
                    reason: format!("the apps lock is poisoned: {error}"),
                })?;
            apps.settle()?;
            match acting(state, apps.held(), headers, directory.projection()?)? {
                Acting::App { app, .. } => app,
                Acting::Administrator(_) | Acting::Person(_) | Acting::Registrar { .. } => {
                    return Err(ServerError::NotAdmitted {
                        reason: "only an app's connector reads and closes its approved drafts",
                    });
                }
            }
        };
        act(&app, directory)
    })
}

/// The view of an approved draft lys-pass reads.
pub(crate) fn approved_view(record: &ProductDraftRecord) -> ApprovedDraftView {
    let created = &record.created;
    ApprovedDraftView {
        id: created.operation.to_string(),
        app: created.app.clone(),
        grant: created.grant.to_string(),
        target: wire::ProductTarget {
            kind: created.target.kind.clone(),
            id: created.target.id.clone(),
            action: created.target.action.clone(),
        },
        request_digest: hex(&created.request_digest),
        words: created.words.clone(),
    }
}

/// `GET /product-drafts`: a connector, carrying its app's credential,
/// reads the approved drafts of its own app that it has not closed, in
/// draft id order after `after`; a signed-in person reads every draft they
/// may see. The page is the whole population after the cursor: no page
/// size is configured for this route, so none is invented, and `next` is
/// always the end.
async fn list(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    query: Result<Query<ApprovedQuery>, axum::extract::rejection::QueryRejection>,
) -> Result<axum::response::Response, ServerError> {
    use axum::response::IntoResponse;
    let Query(query) = query.map_err(|refused| malformed(refused.body_text()))?;
    if !headers.contains_key(header::AUTHORIZATION) && query.app.is_none() {
        if query.state.is_some() {
            return Err(malformed(
                "a person reads every draft they may see, by no state",
            ));
        }
        let listed = listing::person_list(&state, &headers, query.after.as_deref())?;
        return Ok(Json(listed).into_response());
    }
    let (Some(asked), Some("approved")) = (query.app.as_deref(), query.state.as_deref()) else {
        return Err(malformed(
            "a connector reads its own app's drafts by app and state=approved",
        ));
    };
    let page = with_connector(&state, &headers, |app, directory| {
        if app != asked {
            return Err(ProductDraftError::NotYourApp {
                caller: app.to_owned(),
                app: asked.to_owned(),
            }
            .into());
        }
        let projection = directory.projection()?;
        let waiting: Vec<&ProductDraftRecord> = projection
            .product_drafts()
            .filter(|record| {
                record.created.app == app && record.is_approved() && record.closed.is_none()
            })
            .collect();
        let total = waiting.len();
        let drafts = waiting
            .into_iter()
            .filter(|record| {
                query
                    .after
                    .as_deref()
                    .is_none_or(|after| record.created.operation.to_string().as_str() > after)
            })
            .map(approved_view)
            .collect();
        Ok(ApprovedPage {
            drafts,
            next: None,
            total,
        })
    })?;
    Ok(Json(page).into_response())
}

/// How many product drafts the signed-in person may see that still wait on
/// someone, approved and unexecuted ones included: counted from the
/// person's own answer of `GET /product-drafts`, beside the operator drafts.
pub(crate) fn open_count(state: &AppState, headers: &HeaderMap) -> Result<usize, ServerError> {
    Ok(listing::person_list(state, headers, None)?
        .drafts
        .iter()
        .filter(|draft| matches!(draft.state.as_str(), "waiting" | "approved"))
        .count())
}
