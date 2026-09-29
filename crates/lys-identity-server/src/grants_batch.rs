//! Many questions at once: the batch check and the question of which
//! resources a subject may act on.
//!
//! `POST /grants/check/batch` takes up to [`BATCH_MAX`] checks and answers
//! each allowed or refused with its reason, in the order sent, every one at
//! the one revision the answer names, since all are decided under one hold
//! of the grants. `POST /grants/which` answers, a page at a time after a
//! cursor, the ids of a kind a subject may take an action on: each id the
//! subject holds a grant on, and each placed under a resource it holds one
//! on, kept only when the same decision the batch makes allows it.
//!
//! Both are questions: nothing is recorded and no use is written. Both are
//! open to the administrator and to an app, through its credential, for its
//! own kinds only; an app naming another app's kind is refused
//! `not_your_app`. `at_least` is the revision a caller last wrote, and a
//! decision is refused `StaleDecision` rather than made before it.

use std::collections::BTreeSet;
use std::sync::{Arc, PoisonError};

use axum::Json;
use axum::extract::State;
use axum::extract::rejection::JsonRejection;
use axum::http::HeaderMap;
use lys_identity::IdentityId;
use lys_identity::grants::{Action, ExerciseRequest, GrantError, Resource, Route};
use serde::{Deserialize, Serialize};

use crate::apps_binding::{Acting, acting};
use crate::apps_error::AppError;
use crate::error::ServerError;
use crate::grants::{Decision, Judged, decide, with_grants};
use crate::routes::{AppState, identity_id};
use crate::session::now;

/// The most checks one batch takes.
pub const BATCH_MAX: usize = 500;

/// One check of a batch.
#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CheckWire {
    /// The person or agent asked about.
    pub subject: String,
    /// The resource's kind.
    pub kind: String,
    /// The resource's id.
    pub id: String,
    /// The action.
    pub action: String,
}

/// A batch of checks.
#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct BatchBody {
    /// The checks, answered in this order.
    pub checks: Vec<CheckWire>,
    /// The revision the caller last wrote, which every decision reflects.
    #[serde(default)]
    pub at_least: Option<u64>,
}

/// One check's answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
pub struct CheckAnswer {
    /// Whether the subject may take the action.
    pub allowed: bool,
    /// The grant that allows it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grant: Option<String>,
    /// The grant and every ancestor, from the grant to its root.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub path: Vec<String>,
    /// The resource whose grant allows it, `kind:id`: the one asked about,
    /// or a parent it is placed in.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub via: Option<String>,
    /// The refusal's name, when refused.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refusal: Option<String>,
    /// The refusal's words, when refused.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// A batch's answers.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct BatchAnswer {
    /// The permission revision every answer was decided at.
    pub revision: u64,
    /// Each check's answer, in the order sent.
    pub results: Vec<CheckAnswer>,
}

/// The question of which ids a subject may act on.
#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct WhichBody {
    /// The person or agent asked about.
    pub subject: String,
    /// The kind.
    pub kind: String,
    /// The action.
    pub action: String,
    /// The last id of the previous page; absent for the first.
    #[serde(default)]
    pub after: Option<String>,
    /// How many ids one page carries, at least one.
    pub page_size: usize,
    /// The revision the caller last wrote, which every decision reflects.
    #[serde(default)]
    pub at_least: Option<u64>,
}

/// A page of ids.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct WhichPage {
    /// The ids, in order.
    pub ids: Vec<String>,
    /// The cursor of the next page; absent on the last.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next: Option<String>,
    /// The permission revision every answer was decided at.
    pub revision: u64,
}

fn malformed(reason: impl Into<String>) -> ServerError {
    ServerError::RequestMalformed {
        reason: reason.into(),
    }
}

/// Whether `error` says the question could not be answered, rather than
/// answering it no: `which` answers such a failure by its name instead of
/// leaving the id out, so a list is never short because the engine, the log
/// or the revision asked for was not there.
pub(crate) fn unanswered(error: &GrantError) -> bool {
    matches!(
        error,
        GrantError::StaleDecision { .. }
            | GrantError::PermissionEngineUnavailable { .. }
            | GrantError::ProjectionPending { .. }
            | GrantError::LogUnavailable { .. }
    )
}

/// Who asks: the administrator, or an app for its own kinds, as the app
/// they act for.
fn asker(state: &AppState, headers: &HeaderMap) -> Result<Option<String>, ServerError> {
    let apps = state.apps.lock().unwrap_or_else(PoisonError::into_inner);
    let who = acting(state, apps.held(), headers)?;
    match who {
        Acting::Administrator(_) => Ok(None),
        Acting::App { app, .. } => Ok(Some(app)),
        Acting::Person(_) | Acting::Registrar { .. } => Err(ServerError::NotAdmitted {
            reason: "only the administrator and an app, for its own kinds, ask many questions at once",
        }),
    }
}

/// The answer to one check, decided as the batch decides every one.
pub(crate) fn one(
    judged: &mut Judged<'_>,
    acting_for: Option<&str>,
    check: &CheckWire,
    at: u64,
    at_least: Option<u64>,
) -> CheckAnswer {
    let refused = |error: ServerError| CheckAnswer {
        allowed: false,
        grant: None,
        path: Vec::new(),
        via: None,
        refusal: Some(error.name()),
        reason: Some(error.to_string()),
    };
    let asked = (|| -> Result<ExerciseRequest, ServerError> {
        judged.apps.admit_kind(acting_for, &check.kind)?;
        judged.apps.admit_action(&check.kind, &check.action)?;
        Ok(ExerciseRequest {
            caller: identity_id(&check.subject)?,
            route: Route::Api,
            resource: Resource::new(&check.kind, &check.id)?,
            action: Action::new(&check.action)?,
        })
    })();
    let request = match asked {
        Ok(request) => request,
        Err(error) => return refused(error),
    };
    match decide(judged, &request, at, at_least, Decision::Explain) {
        Ok((permit, via)) => CheckAnswer {
            allowed: true,
            grant: Some(permit.grant.to_string()),
            path: permit.path.iter().map(ToString::to_string).collect(),
            via: Some(via.to_string()),
            refusal: None,
            reason: None,
        },
        Err(error) => refused(error.into()),
    }
}

/// Many checks at once, each answered in order at one named revision.
pub async fn batch(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Result<Json<BatchBody>, JsonRejection>,
) -> Result<Json<BatchAnswer>, ServerError> {
    let Json(body) = body.map_err(|refused| malformed(refused.body_text()))?;
    if body.checks.len() > BATCH_MAX {
        return Err(AppError::BatchTooLarge {
            count: body.checks.len(),
            most: BATCH_MAX,
        }
        .into());
    }
    let acting_for = asker(&state, &headers)?;
    let at = now();
    with_grants(&state, |mut judged| {
        let results = body
            .checks
            .iter()
            .map(|check| one(&mut judged, acting_for.as_deref(), check, at, body.at_least))
            .collect();
        Ok(Json(BatchAnswer {
            revision: judged.grants.revision(),
            results,
        }))
    })
}

/// The ids of `kind` a subject may take `action` on, a page at a time.
pub async fn which(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Result<Json<WhichBody>, JsonRejection>,
) -> Result<Json<WhichPage>, ServerError> {
    let Json(body) = body.map_err(|refused| malformed(refused.body_text()))?;
    if body.page_size == 0 {
        return Err(malformed("page_size is at least 1"));
    }
    let acting_for = asker(&state, &headers)?;
    let subject: IdentityId = identity_id(&body.subject)?;
    let action = Action::new(&body.action)?;
    let at = now();
    with_grants(&state, |mut judged| {
        judged.apps.admit_kind(acting_for.as_deref(), &body.kind)?;
        judged.apps.admit_action(&body.kind, action.as_str())?;
        let mut candidates: BTreeSet<String> = judged
            .grants
            .book()
            .held_by(subject)
            .map(|record| record.grant().resource())
            .filter(|resource| resource.kind() == body.kind)
            .map(|resource| resource.id().to_owned())
            .collect();
        candidates.extend(
            judged
                .apps
                .held()
                .placements
                .iter()
                .filter(|placed| placed.child_kind == body.kind)
                .map(|placed| placed.child_id.clone()),
        );
        let after = body.after.as_deref();
        let mut ids = Vec::new();
        let mut more = false;
        for id in candidates
            .into_iter()
            .filter(|id| after.is_none_or(|after| id.as_str() > after))
        {
            let request = ExerciseRequest {
                caller: subject,
                route: Route::Api,
                resource: Resource::new(&body.kind, &id)?,
                action: action.clone(),
            };
            match decide(&mut judged, &request, at, body.at_least, Decision::Explain) {
                Ok(_) => {}
                Err(error) if unanswered(&error) => return Err(error.into()),
                Err(_) => continue,
            }
            if ids.len() == body.page_size {
                more = true;
                break;
            }
            ids.push(id);
        }
        let next = if more { ids.last().cloned() } else { None };
        Ok(Json(WhichPage {
            ids,
            next,
            revision: judged.grants.revision(),
        }))
    })
}
