//! Scoped guest admission (ACCESS-006 R4), served at `POST
//! /grants/membership/admission`.
//!
//! A subject is admitted to a workspace when it holds at least one current
//! grant within it. The question is answered from the book's live holder
//! index: only the subject's own live grants are read, never its revoked
//! history, those on resources placed within the workspace are kept (a
//! grant held above the workspace is not within it and does not admit), in
//! resource order, and each is decided by
//! [`decide`] on one of its actions until one stands. A revoked, expired or
//! otherwise refused grant does not admit; a decision the grants could not
//! make is answered by its name with no revision. Admission opens nothing:
//! every act after it is its own membership decision. An agent is judged as
//! its own identity, so its responsible person's grants never admit it.

use std::collections::BTreeMap;
use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::extract::rejection::JsonRejection;
use axum::http::HeaderMap;
use lys_identity::grants::{Action, ExerciseRequest, Resource, Route, identity_kind};
use lys_log_store::LeafStore;
use lys_pass::membership::{self, CONTRACT_VERSION, GrantLog, Verdict};
use lys_pass::membership_admission::{self, AdmissionDecision, AdmissionRequest, NO_CURRENT_GRANT};

use crate::channel_membership::{Named, contract_resource, named, served_log, within};
use crate::channel_membership_counts::Counts;
use crate::error::ServerError;
use crate::grants::{Decision, Judged, decide, with_grants};
use crate::grants_batch::{asker, unanswered};
use crate::routes::{AppState, identity_id};
use crate::session::now;

/// The subject's grants within the workspace, one question per resource:
/// the resource and the first of the grant's actions.
fn questions<S: LeafStore>(
    judged: &Judged<'_, S>,
    (acting_for, counts): (Option<&str>, &Counts),
    request: &AdmissionRequest,
) -> Result<BTreeMap<Resource, ExerciseRequest>, Named> {
    let subject = identity_id(&request.subject.id).map_err(named)?;
    let kind = identity_kind(&subject);
    if kind != request.subject.kind {
        return Err((
            membership::SUBJECT_KIND_MISMATCH.to_owned(),
            format!("the subject is a {kind}, not a {}", request.subject.kind),
        ));
    }
    judged
        .apps
        .admit_kind(acting_for, &request.workspace.kind)
        .map_err(named)?;
    let workspace = Resource::new(&request.workspace.kind, &request.workspace.id).map_err(named)?;
    let mut asked = BTreeMap::new();
    for record in judged.grants.book().live_held_by(subject) {
        counts.probe(1);
        let grant = record.grant();
        let resource = grant.resource();
        if asked.contains_key(resource) || !within(judged, resource, &workspace, counts) {
            continue;
        }
        let Some(action) = grant.actions().iter().next() else {
            continue;
        };
        asked.insert(
            resource.clone(),
            ExerciseRequest {
                caller: subject,
                route: Route::Api,
                resource: resource.clone(),
                action: Action::clone(action),
            },
        );
    }
    Ok(asked)
}

/// The admission decision on `request`, made with `judged` at `at`, its
/// work counted in `counts`.
pub(crate) fn admission<S: LeafStore>(
    judged: &mut Judged<'_, S>,
    (acting_for, counts): (Option<&str>, &Counts),
    served: &GrantLog,
    request: &AdmissionRequest,
    at: u64,
) -> AdmissionDecision {
    let before = |(name, reason): Named| {
        membership_admission::refused_before_lookup(request, served, &name, &reason)
    };
    if let Err(refused) = membership_admission::validate(request, served) {
        return before((refused.name.to_owned(), refused.reason));
    }
    let asked = match questions(judged, (acting_for, counts), request) {
        Ok(asked) => asked,
        Err(refused) => return before(refused),
    };
    let mut verdict = Verdict::Refused {
        refusal: NO_CURRENT_GRANT.to_owned(),
        reason: "the subject holds no current grant within the workspace".to_owned(),
    };
    for question in asked.values() {
        counts.decision();
        match decide(
            judged,
            question,
            at,
            Some(request.at_least),
            Decision::Explain,
        ) {
            Ok((permit, via)) => {
                verdict = Verdict::Allowed {
                    grant: permit.grant.to_string(),
                    path: permit.path.iter().map(ToString::to_string).collect(),
                    scope: contract_resource(&via),
                };
                break;
            }
            Err(error) if unanswered(&error) => return before(named(error)),
            Err(_) => {}
        }
    }
    AdmissionDecision {
        contract: CONTRACT_VERSION,
        log: served.clone(),
        request: request.clone(),
        revision: Some(judged.grants.revision()),
        verdict,
    }
}

/// Whether a subject holds a current grant within a workspace.
pub async fn admit(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Result<Json<AdmissionRequest>, JsonRejection>,
) -> Result<Json<AdmissionDecision>, ServerError> {
    let Json(request) = body.map_err(|refused| ServerError::RequestMalformed {
        reason: refused.body_text(),
    })?;
    let acting_for = asker(&state, &headers)?;
    let served = served_log(&state);
    let at = now();
    let counts = &state.membership.counts;
    counts.call();
    with_grants(&state, |mut judged| {
        Ok(Json(admission(
            &mut judged,
            (acting_for.as_deref(), counts),
            &served,
            &request,
            at,
        )))
    })
}
