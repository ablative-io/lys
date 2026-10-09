//! The served channel membership decision (ACCESS-006 R1).
//!
//! `POST /grants/membership` takes the shared [`MembershipRequest`] every
//! product sends and answers the shared [`MembershipDecision`]: the types
//! are lys-pass's, so the route, its OpenAPI schema and every product's
//! adapter have one owner. It is a question: nothing is recorded and no use
//! is written. It is open to the administrator and to an app, through its
//! credential, for its own kinds only, exactly as the batch check is.
//!
//! The request is judged in order, and each binding is refused by name
//! before the grants are asked: the contract version and grant log, every
//! member's shape, the subject's identity and the kind it is stated to be,
//! the resource and workspace kinds and the action as an approved app
//! declares them, and the resource's placement under the workspace. Only
//! then is the membership decided, by [`decide`], the one decision every
//! check makes, reaching from the resource to each parent its schema lets a
//! grant flow from and never through a restricted placement. A decision
//! the grants could not make (a revision ahead of them, a projection
//! pending, an unresolved operation, the log or the engine unavailable) is
//! answered by its name with no revision, never as a refusal or an allow.

use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::extract::rejection::JsonRejection;
use axum::http::HeaderMap;
use lys_identity::grants::{
    Action, ExerciseRequest, GrantError, Mode, Resource, Route, identity_kind, placed_within,
};
use lys_log_store::LeafStore;
use lys_pass::membership::{
    self, CONTRACT_VERSION, GrantLog, MembershipDecision, MembershipRequest, Verdict,
};

use crate::error::ServerError;
use crate::grants::{Decision, Judged, decide, with_grants};
use crate::grants_batch::{asker, mode_of, unanswered};
use crate::routes::{AppState, identity_id};
use crate::session::now;

/// The epoch of a grant log no reset has replaced. No reset contract exists
/// yet (DIRECTORY-089 R1 owns it), so every served log is in this epoch.
pub const ORIGINAL_EPOCH: u64 = 0;

/// The grant log this service decides from: the origin the log was created
/// with, which the store pins at creation and never changes, in its
/// original epoch.
pub(crate) fn served_log(state: &AppState) -> GrantLog {
    GrantLog {
        identity: state.grant_setup.log_origin.clone(),
        epoch: ORIGINAL_EPOCH,
    }
}

fn pass_mode(mode: Mode) -> lys_pass::Mode {
    match mode {
        Mode::Outright => lys_pass::Mode::Outright,
        Mode::ByDraft => lys_pass::Mode::ByDraft,
        Mode::ByTwo => lys_pass::Mode::ByTwo,
    }
}

fn contract_resource(resource: &Resource) -> lys_pass::rights::Resource {
    lys_pass::rights::Resource {
        kind: resource.kind().to_owned(),
        id: resource.id().to_owned(),
    }
}

/// A refusal's stable name and words.
type Named = (String, String);

fn named(error: impl Into<ServerError>) -> Named {
    let error = error.into();
    (error.name(), error.to_string())
}

/// The exercise `request` asks about, once every binding is judged, or the
/// first binding refused, by name.
fn bind<S: LeafStore>(
    judged: &Judged<'_, S>,
    acting_for: Option<&str>,
    request: &MembershipRequest,
) -> Result<ExerciseRequest, Named> {
    let caller = identity_id(&request.subject.id).map_err(named)?;
    let kind = identity_kind(&caller);
    if kind != request.subject.kind {
        return Err((
            membership::SUBJECT_KIND_MISMATCH.to_owned(),
            format!("the subject is a {kind}, not a {}", request.subject.kind),
        ));
    }
    for asked in [&request.workspace.kind, &request.resource.kind] {
        judged.apps.admit_kind(acting_for, asked).map_err(named)?;
    }
    judged
        .apps
        .admit_action(&request.resource.kind, &request.action)
        .map_err(named)?;
    let resource = Resource::new(&request.resource.kind, &request.resource.id).map_err(named)?;
    let workspace = Resource::new(&request.workspace.kind, &request.workspace.id).map_err(named)?;
    let held = judged.apps.held();
    let within = placed_within(&resource, &workspace, |child| {
        held.parent(child.kind(), child.id())
            .and_then(|placed| Resource::new(&placed.parent_kind, &placed.parent_id).ok())
    });
    if !within {
        return Err((
            membership::OUTSIDE_WORKSPACE.to_owned(),
            "the resource is not placed under the named workspace".to_owned(),
        ));
    }
    Ok(ExerciseRequest {
        caller,
        route: Route::Api,
        resource,
        action: Action::new(&request.action).map_err(named)?,
    })
}

/// The membership decision on `request`, made with `judged` at `at` for
/// the asker acting for `acting_for`, from the grant log `served`.
pub(crate) fn judge<S: LeafStore>(
    judged: &mut Judged<'_, S>,
    acting_for: Option<&str>,
    served: &GrantLog,
    request: &MembershipRequest,
    at: u64,
) -> MembershipDecision {
    let before =
        |(name, reason): Named| membership::refused_before_lookup(request, served, &name, &reason);
    if let Err(refused) = membership::validate(request, served) {
        return before((refused.name.to_owned(), refused.reason));
    }
    let asked = match bind(judged, acting_for, request) {
        Ok(asked) => asked,
        Err(refused) => return before(refused),
    };
    let decided = decide(
        judged,
        &asked,
        at,
        Some(request.at_least),
        Decision::Explain,
    );
    let verdict = match decided {
        Ok((permit, via)) => match mode_of(judged, permit.grant) {
            Some(Mode::Outright) => Verdict::Allowed {
                grant: permit.grant.to_string(),
                path: permit.path.iter().map(ToString::to_string).collect(),
                scope: contract_resource(&via),
            },
            Some(mode) => Verdict::Held {
                grant: permit.grant.to_string(),
                scope: contract_resource(&via),
                mode: pass_mode(mode),
            },
            // The book no longer names the grant it was decided on: not held.
            None => {
                let (refusal, reason) = named(GrantError::NotHeld {
                    identity: asked.caller.to_string(),
                    resource: asked.resource.to_string(),
                    action: asked.action.to_string(),
                });
                Verdict::Refused { refusal, reason }
            }
        },
        Err(error) if unanswered(&error) => return before(named(error)),
        Err(error) => {
            let (refusal, reason) = named(error);
            Verdict::Refused { refusal, reason }
        }
    };
    MembershipDecision {
        contract: CONTRACT_VERSION,
        log: served.clone(),
        request: request.clone(),
        revision: Some(judged.grants.revision()),
        verdict,
    }
}

/// Decide one channel membership question at the grants' current revision.
pub async fn membership(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Result<Json<MembershipRequest>, JsonRejection>,
) -> Result<Json<MembershipDecision>, ServerError> {
    let Json(request) = body.map_err(|refused| ServerError::RequestMalformed {
        reason: refused.body_text(),
    })?;
    let acting_for = asker(&state, &headers)?;
    let served = served_log(&state);
    let at = now();
    with_grants(&state, |mut judged| {
        Ok(Json(judge(
            &mut judged,
            acting_for.as_deref(),
            &served,
            &request,
            at,
        )))
    })
}
