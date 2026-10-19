//! DIRECTORY-108: whether a resource of the asker's own kind has any holder of
//! an action, answered held or none from the whole book and naming no one.
//! `who` answers only the grants its caller sees (grant_sight.rs), so an app
//! could never read "nobody holds this" from it; `held` judges every grant on
//! the resource, as `who` does before its sight filter, and discloses only
//! whether one permits.

use super::Judged;
use crate::apps_binding::{Acting, acting};
use crate::error::ServerError;
use crate::grant_contract::ActionBody;
use crate::routes::AppState;
use crate::session::now;
use axum::Json;
use axum::extract::State;
use axum::http::HeaderMap;
use lys_identity::grants::ExerciseRequest;
use serde::Serialize;
use std::collections::BTreeSet;
use std::sync::Arc;

/// The held answer: whether any holder may take the action now, and the
/// revision it was judged at. It names no holder, grant or count.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct HeldAnswer {
    /// True when at least one holder's grant permits the action now.
    pub held: bool,
    /// The grants' revision the answer was judged at.
    pub revision: u64,
}

/// POST /grants/held: an approved app about a kind it owns, or the
/// administrator about any kind, asks whether anyone may take the action.
pub(super) async fn held(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<ActionBody>,
) -> Result<Json<HeldAnswer>, ServerError> {
    let (route, resource, action) = body.parts()?;
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
            admit(&state, &headers, &judged, resource.kind())?;
            judged.apps.admit_action(resource.kind(), action.as_str())?;
            let holders: BTreeSet<_> = judged
                .grants
                .book()
                .on_resource(&resource)
                .filter(|record| {
                    judged
                        .grants
                        .model()
                        .actions_of(record.grant())
                        .contains(&action)
                })
                .map(|record| record.grant().holder())
                .collect();
            let frame = judged.grants.frame(judged.directory, None)?;
            let revision = frame.revision();
            for holder in holders {
                let request = ExerciseRequest {
                    caller: holder,
                    route,
                    resource: resource.clone(),
                    action: action.clone(),
                };
                match judged.grants.explain_in(&frame, &request, at) {
                    Ok(_) => {
                        return Ok(Json(HeldAnswer {
                            held: true,
                            revision,
                        }));
                    }
                    // A decision that could not be made is the answer, never "none".
                    Err(error) if crate::grants_batch::unanswered(&error) => {
                        return Err(error.into());
                    }
                    Err(_) => {}
                }
            }
            Ok(Json(HeldAnswer {
                held: false,
                revision,
            }))
        },
    )
}

/// The asker: an approved app about a kind it owns (refused `not_your_app`
/// otherwise), or the administrator about any kind. Anyone else is refused.
fn admit(
    state: &AppState,
    headers: &HeaderMap,
    judged: &Judged<'_>,
    kind: &str,
) -> Result<(), ServerError> {
    match acting(state, judged.apps.held(), headers, judged.directory)? {
        Acting::App { app, .. } => judged
            .apps
            .admit_kind(Some(&app), kind)
            .map_err(ServerError::from),
        Acting::Administrator(_) => judged
            .apps
            .admit_kind(None, kind)
            .map_err(ServerError::from),
        Acting::Person(_) | Acting::Registrar { .. } => Err(ServerError::NotAdmitted {
            reason: "held is asked by an approved app about its own kinds, or by the administrator",
        }),
    }
}
