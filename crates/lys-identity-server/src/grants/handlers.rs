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
use lys_identity::grants::{Action, ExerciseRequest, GrantError, Mode, owner_of};
use lys_identity::{IdentityError, IdentityId};
use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;

#[cfg(test)]
#[path = "model_tests.rs"]
mod model_tests;

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

/// The permission model from the approved apps' current schemas, so a
/// screen offers only the relations and agent actions they declare.
pub(super) async fn model(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<ModelView>, ServerError> {
    signed_in(&state, &headers)?;
    crate::apps_api::with_apps(&state, |apps, _| Ok(Json(ModelView::from(&apps.model()?))))
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
pub(super) fn refuse_held_on_hot(
    judged: &Judged<'_>,
    resource: &lys_identity::grants::Resource,
    relation: &lys_identity::grants::Relation,
    mode: Mode,
) -> Result<(), ServerError> {
    let kind = resource.kind();
    let Some(schema) = judged
        .apps
        .schema(owner_of(kind))
        .filter(|_| mode.is_held())
    else {
        return Ok(());
    };
    match schema.hot_action(kind, relation) {
        Some(action) => Err(ServerError::App(AppError::GrantModeOnHotAction {
            app: schema.app().to_owned(),
            action: action.to_string(),
            class: "hot",
            mode: mode.as_str(),
        })),
        None => Ok(()),
    }
}

/// The person a root grant names and what it lets them do, for the letter
/// written to them once the grant is recorded (DIRECTORY-110 R3).
pub(crate) struct Recipient {
    /// Their display name.
    pub(crate) name: String,
    /// Their subject at the issuer, whose account holds their email.
    pub(crate) subject: String,
    /// What they may now do, in the access screen's words.
    pub(crate) words: String,
}

/// Record a root grant as `body` asks, answering the record and, for an
/// outright grant to a person with a bound login, the recipient of the
/// access letter. The letter is not sent here: the one act that owns the
/// grant sends its one letter (POST /people/admit the added letter, POST
/// /grants/roots the access letter).
pub(crate) fn issue_root_recorded(
    state: &Arc<AppState>,
    headers: &HeaderMap,
    body: &RootBody,
) -> Result<(RecordedView, Option<Recipient>), ServerError> {
    with_grants(state, |judged| {
        let request = body.request(crate::service_account_grants::caller(
            state, headers, &judged,
        )?)?;
        judged.apps.admit_kind(None, request.resource.kind())?;
        refuse_held_on_hot(&judged, &request.resource, &request.relation, body.mode())?;
        let recorded =
            judged
                .grants
                .issue_root_in(judged.directory, &request, body.mode(), now())?;
        let recipient = (body.mode() == Mode::Outright)
            .then(|| {
                let holder = judged
                    .directory
                    .record(IdentityId::Person(request.holder))?;
                let parts = judged.grants.book().grant(recorded.receipt.grant)?.parts();
                Some(Recipient {
                    name: holder.profile().display_name().to_owned(),
                    subject: holder.bindings().first()?.subject().to_owned(),
                    words: crate::mail_letters::access_words(
                        parts.relation.as_str(),
                        parts.resource.kind(),
                        parts.resource.id(),
                        parts.actions.iter().map(Action::as_str),
                    ),
                })
            })
            .flatten();
        Ok((RecordedView::from(&recorded), recipient))
    })
}

/// The access letter for `recipient`, sent once for `operation`: unsent by
/// name when the install has no mail settings; `MailNotSent` when the
/// server refuses it, the grant staying recorded.
async fn access_letter(
    state: &Arc<AppState>,
    operation: &str,
    recipient: Option<Recipient>,
) -> Result<Option<crate::mail::LetterView>, ServerError> {
    let Some(recipient) = recipient else {
        return Ok(None);
    };
    let Some(mailer) = &state.mailer else {
        return Ok(Some(crate::mail::LetterView::unsent(
            "access_given",
            None,
            "mail_unconfigured",
        )));
    };
    let api = crate::sign_in_providers::api(state)?;
    let email = crate::accounts::email_of(api, &recipient.subject).await?;
    let letter = crate::mail_letters::Letter::AccessGiven {
        name: recipient.name,
        email,
        words: recipient.words,
        address: state.oidc.public_origin().to_owned(),
    };
    mailer.send_once(operation, &letter).await.map(Some)
}

pub(crate) async fn issue_root(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<RootBody>,
) -> Result<Json<RecordedView>, ServerError> {
    let (mut view, recipient) = issue_root_recorded(&state, &headers, &body)?;
    view.letter = access_letter(&state, &view.operation, recipient).await?;
    Ok(Json(view))
}

pub(crate) async fn delegate(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<DelegateBody>,
) -> Result<Json<RecordedView>, ServerError> {
    with_grants(&state, |judged| {
        if body.is_administrator() {
            let mut judged = judged;
            return super::administrator::delegate(&state, &headers, &body, &mut judged).map(Json);
        }
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
        // An agent takes back only a grant it passed on itself (AGENTS-015 R1).
        if matches!(caller, IdentityId::Agent(_))
            && judged
                .grants
                .book()
                .grant(id)
                .is_some_and(|grant| grant.parts().issuer != caller)
        {
            return Err(as_seen_by(
                &judged,
                caller,
                GrantError::RevokeRefused {
                    caller: caller.to_string(),
                    grant: id.to_string(),
                },
            ));
        }
        match judged.grants.revoke(judged.directory, &request, now()) {
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
    crate::grants_reads::with_read_grants(
        &state,
        None,
        now(),
        |judged| {
            let caller = crate::service_account_grants::caller(&state, &headers, judged)?;
            judged.apps.admit_kind(None, resource.kind())?;
            judged.apps.admit_action(resource.kind(), action.as_str())?;
            Ok(vec![ExerciseRequest {
                caller,
                route,
                resource: resource.clone(),
                action: action.clone(),
            }])
        },
        |mut judged| {
            let caller = crate::service_account_grants::caller(&state, &headers, &judged)?;
            judged.apps.admit_kind(None, resource.kind())?;
            judged.apps.admit_action(resource.kind(), action.as_str())?;
            let request = ExerciseRequest {
                caller,
                route,
                resource: resource.clone(),
                action: action.clone(),
            };
            match decide(&mut judged, &request, now(), None, Decision::Exercise) {
                Ok((permit, _)) => Ok(Json(PermitView::from(&permit))),
                Err(error) => Err(as_seen_by(&judged, caller, error)),
            }
        },
    )
}

pub(super) async fn why(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<ActionBody>,
) -> Result<Json<PermitView>, ServerError> {
    let (route, resource, action) = body.parts()?;
    crate::grants_reads::with_read_grants(
        &state,
        None,
        now(),
        |judged| {
            let caller = crate::service_account_grants::caller(&state, &headers, judged)?;
            judged.apps.admit_kind(None, resource.kind())?;
            judged.apps.admit_action(resource.kind(), action.as_str())?;
            Ok(vec![ExerciseRequest {
                caller,
                route,
                resource: resource.clone(),
                action: action.clone(),
            }])
        },
        |mut judged| {
            let caller = crate::service_account_grants::caller(&state, &headers, &judged)?;
            judged.apps.admit_kind(None, resource.kind())?;
            judged.apps.admit_action(resource.kind(), action.as_str())?;
            let request = ExerciseRequest {
                caller,
                route,
                resource: resource.clone(),
                action: action.clone(),
            };
            match decide(&mut judged, &request, now(), None, Decision::Explain) {
                Ok((permit, _)) => Ok(Json(PermitView::from(&permit))),
                Err(error) => Err(as_seen_by(&judged, caller, error)),
            }
        },
    )
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
    crate::grants_reads::with_read_grants(
        &state,
        None,
        at,
        |judged| {
            Ok(crate::grants_reads::resource_questions(
                judged, &resource, &action,
            ))
        },
        |judged| {
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
                    // A grant naming a role is judged as the role's actions now.
                    judged
                        .grants
                        .model()
                        .actions_of(record.grant())
                        .contains(&action)
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
                degraded: frame
                    .degradation()
                    .map(|held| crate::grants::DegradedView::from(held.as_ref())),
            }))
        },
    )
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
