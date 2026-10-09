//! The grant routes' handlers: list, read, issue, pass on, revoke, check,
//! and the questions why, who and what cannot be given.

use super::{Decision, Judged, decide, grant_view, with_grants};
use crate::apps_error::AppError;
use crate::error::ServerError;
use crate::grant_contract::{
    ActionBody, CannotGiveAnswer, CannotGiveBody, DelegateBody, GrantList, GrantView, HolderView,
    ModelView, PAGE_MAX, PermitView, RecordedView, RevokeBody, RootBody, WhoBody, WhoPage,
    grant_id,
};
use crate::grant_sight::{as_seen_by, sees, sees_identity, sees_with, visible_or};
use crate::routes::{AppState, signed_in};
use crate::session::now;
use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use lys_identity::grants::{ExerciseRequest, GrantError, Mode, RootRequest, owner_of};
use lys_identity::{IdentityError, IdentityId};
use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;

pub(super) async fn list(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<GrantList>, ServerError> {
    with_grants(&state, |judged| {
        let caller = crate::service_account_grants::caller(&state, &headers, &judged)?;
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
pub(super) async fn model(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<ModelView>, ServerError> {
    signed_in(&state, &headers)?;
    Ok(Json(ModelView::from(&state.grant_setup.model()?)))
}

pub(super) async fn read(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<GrantView>, ServerError> {
    let id = grant_id(&id)?;
    with_grants(&state, |judged| {
        let caller = crate::service_account_grants::caller(&state, &headers, &judged)?;
        let record = judged
            .grants
            .book()
            .record(id)
            .filter(|record| sees(&judged, caller, record))
            .ok_or(ServerError::GrantNotVisible)?;
        Ok(Json(grant_view(&judged, caller, record, now())))
    })
}

/// Refused `grant_mode_on_hot_action` when `mode` holds the grant for a
/// draft and its relation carries an action the app's schema marks hot: a hot
/// action is decided from the pass alone, so it is never held (D4).
fn refuse_held_on_hot(
    judged: &Judged<'_>,
    request: &RootRequest,
    mode: Mode,
) -> Result<(), ServerError> {
    let kind = request.resource.kind();
    let Some(schema) = judged.apps.schema(owner_of(kind)).filter(|_| mode.is_held()) else {
        return Ok(());
    };
    match schema.hot_action(kind, &request.relation) {
        Some(action) => Err(ServerError::App(AppError::GrantModeOnHotAction {
            app: schema.app().to_owned(),
            action: action.to_string(),
            class: "hot",
            mode: mode.as_str(),
        })),
        None => Ok(()),
    }
}

pub(crate) async fn issue_root(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<RootBody>,
) -> Result<Json<RecordedView>, ServerError> {
    with_grants(&state, |judged| {
        let request = body.request(crate::service_account_grants::caller(
            &state, &headers, &judged,
        )?)?;
        judged.apps.admit_kind(None, request.resource.kind())?;
        refuse_held_on_hot(&judged, &request, body.mode())?;
        let recorded = judged
            .grants
            .issue_root_in(judged.directory, &request, body.mode(), now())?;
        Ok(Json(RecordedView::from(&recorded)))
    })
}

pub(crate) async fn delegate(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<DelegateBody>,
) -> Result<Json<RecordedView>, ServerError> {
    with_grants(&state, |judged| {
        let caller = crate::service_account_grants::caller(&state, &headers, &judged)?;
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
        let source_holder = judged
            .grants
            .book()
            .grant(request.source)
            .filter(|source| source.holder() != caller)
            .map(|source| {
                lys_identity::grants::admission::delegation_authority(
                    judged.directory,
                    source,
                    caller,
                    request.recipient,
                )?;
                let pending = crate::operator::upgrade_pending(&state).map_err(|error| {
                    ServerError::DirectoryUnavailable {
                        reason: format!("boss delegation cannot read upgrade intent: {error}"),
                    }
                })?;
                if pending {
                    return Err(ServerError::NotAdmitted {
                        reason: "upgrade_pending: boss delegation waits until the upgrade commits",
                    });
                }
                Ok(source.holder().to_string())
            })
            .transpose()?;
        match judged.grants.delegate(judged.directory, &request, now()) {
            Ok(recorded) => {
                let mut answer = RecordedView::from(&recorded);
                answer.receipt.source_holder = source_holder;
                Ok(Json(answer))
            }
            Err(error) => Err(as_seen_by(&judged, caller, error)),
        }
    })
}

pub(super) async fn revoke(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<RevokeBody>,
) -> Result<Json<RecordedView>, ServerError> {
    let id = grant_id(&id)?;
    with_grants(&state, |judged| {
        let caller = crate::service_account_grants::caller(&state, &headers, &judged)?;
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
pub(super) async fn check(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<ActionBody>,
) -> Result<Json<PermitView>, ServerError> {
    let (route, resource, action) = body.parts()?;
    with_grants(&state, |mut judged| {
        let caller = crate::service_account_grants::caller(&state, &headers, &judged)?;
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

pub(super) async fn why(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<ActionBody>,
) -> Result<Json<PermitView>, ServerError> {
    let (route, resource, action) = body.parts()?;
    with_grants(&state, |mut judged| {
        let caller = crate::service_account_grants::caller(&state, &headers, &judged)?;
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

pub(super) async fn who(
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
        let caller = crate::service_account_grants::caller(&state, &headers, &judged)?;
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
        let frame = judged.grants.frame(judged.directory, None)?;
        // A decision that could not be made is the answer, never a holder
        // left off the page.
        let mut page = Vec::new();
        let mut more = false;
        for (text, holder) in holders {
            let request = ExerciseRequest {
                caller: holder,
                route,
                resource: resource.clone(),
                action: action.clone(),
            };
            match judged.grants.explain_in(&frame, &request, at) {
                Ok(_) if page.len() == body.page_size => {
                    more = true;
                    break;
                }
                Ok(permit) => page.push(HolderView {
                    holder: text,
                    permit: PermitView::from(&permit),
                }),
                Err(error) if crate::grants_batch::unanswered(&error) => {
                    return Err(error.into());
                }
                Err(_) => {}
            }
        }
        let next = if more {
            page.last().map(|last| last.holder.clone())
        } else {
            None
        };
        Ok(Json(WhoPage {
            holders: page,
            revision: frame.revision(),
            complete: next.is_none(),
            next,
        }))
    })
}

/// Whether `caller` may name `recipient` on the delegation form: a person the
/// directory records, since the policy admits people as recipients, or an
/// agent the caller may see.
pub(super) fn names_recipient(
    judged: &Judged<'_>,
    caller: IdentityId,
    recipient: IdentityId,
) -> bool {
    judged.directory.record(recipient).is_some()
        && (matches!(recipient, IdentityId::Person(_)) || sees_identity(judged, caller, recipient))
}

/// What the caller cannot give the recipient from the source grant, each
/// item with its one reason, as the grants' one authority owner lists it.
/// The question is a read, asked in the query: `route`, `source` and `recipient`.
pub(super) async fn cannot_give(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(body): Query<CannotGiveBody>,
) -> Result<Json<CannotGiveAnswer>, ServerError> {
    with_grants(&state, |judged| {
        let caller = crate::service_account_grants::caller(&state, &headers, &judged)?;
        let request = body.request(caller)?;
        judged
            .grants
            .book()
            .record(request.source)
            .filter(|record| sees(&judged, caller, record))
            .ok_or(ServerError::GrantNotVisible)?;
        if !names_recipient(&judged, caller, request.recipient) {
            let unknown = ServerError::from(IdentityError::IdentityUnknown {
                identity: request.recipient.to_string(),
            });
            return Err(ServerError::Withheld {
                refusal: unknown.name(),
            });
        }
        let at = now();
        match judged.grants.cannot_give(judged.directory, &request, at) {
            Ok(list) => Ok(Json(CannotGiveAnswer::from(&list))),
            Err(error) => Err(as_seen_by(&judged, caller, error)),
        }
    })
}
