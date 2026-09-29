//! Who may act on many resources at once, for the permission graph.
//!
//! `POST /grants/reach` takes up to [`REACH_MAX`] resources, each with the
//! actions asked about, and answers for each resource every holder the
//! caller may see with the actions it may take, in holder order. It is
//! `/grants/who` asked of every resource and action together: the same
//! holders, the same decisions, all made at the one revision the answer
//! names, from one reading of the permission engine, so the graph costs one
//! read rather than one per holder, action and resource. It is a question:
//! nothing is recorded.

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::http::HeaderMap;
use lys_identity::IdentityId;
use lys_identity::grants::{Action, ExerciseRequest, Resource};
use serde::{Deserialize, Serialize};

use crate::error::ServerError;
use crate::grant_contract::RouteWire;
use crate::grant_sight::sees_with;
use crate::grants::with_grants;
use crate::session::now;

/// The most resources one reach question names.
pub const REACH_MAX: usize = 500;

/// One resource and the actions asked about it.
#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ReachResource {
    /// The resource's kind.
    pub kind: String,
    /// The resource's id.
    pub id: String,
    /// The actions asked about, each answered for every holder.
    pub actions: Vec<String>,
}

/// Who may act on each of these resources.
#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ReachBody {
    /// How the request says it arrived; recorded nowhere, deciding nothing.
    pub route: RouteWire,
    /// The resources, answered in this order.
    pub resources: Vec<ReachResource>,
}

/// One holder and the actions it may take.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
pub struct ReachHolder {
    /// The holder.
    pub holder: String,
    /// The actions it may take, in the order asked.
    pub actions: Vec<String>,
}

/// Who may act on one resource.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
pub struct ReachView {
    /// The resource's kind.
    pub kind: String,
    /// The resource's id.
    pub id: String,
    /// Every holder the caller may see that may take an action asked about.
    pub holders: Vec<ReachHolder>,
}

/// The reach answer.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct ReachAnswer {
    /// The revision every decision was made at.
    pub revision: u64,
    /// Each resource's holders, in the order asked.
    pub resources: Vec<ReachView>,
}

/// `POST /grants/reach`.
pub(crate) async fn reach(
    State(state): State<Arc<crate::routes::AppState>>,
    headers: HeaderMap,
    Json(body): Json<ReachBody>,
) -> Result<Json<ReachAnswer>, ServerError> {
    if body.resources.is_empty() || body.resources.len() > REACH_MAX {
        return Err(ServerError::RequestMalformed {
            reason: format!("resources names 1 to {REACH_MAX} resources"),
        });
    }
    let route = body.route.into();
    let at = now();
    with_grants(&state, |judged| {
        let caller = crate::service_account_grants::caller(&state, &headers, &judged)?;
        let mut asked = Vec::with_capacity(body.resources.len());
        for wire in &body.resources {
            judged.apps.admit_kind(None, &wire.kind)?;
            let mut actions = Vec::with_capacity(wire.actions.len());
            for action in &wire.actions {
                judged.apps.admit_action(&wire.kind, action)?;
                actions.push(Action::new(action)?);
            }
            asked.push((wire, Resource::new(&wire.kind, &wire.id)?, actions));
        }
        let frame = judged.grants.frame(None)?;
        let mut known = HashMap::new();
        let mut resources = Vec::with_capacity(asked.len());
        for (wire, resource, actions) in asked {
            let mut holders: BTreeMap<String, (IdentityId, Vec<&Action>)> = BTreeMap::new();
            for record in judged.grants.book().on_resource(&resource) {
                if !sees_with(&judged, caller, record, &mut known) {
                    continue;
                }
                let holder = record.grant().holder();
                let entry = holders
                    .entry(holder.to_string())
                    .or_insert_with(|| (holder, Vec::new()));
                for action in &actions {
                    if record.grant().actions().contains(action) && !entry.1.contains(&action) {
                        entry.1.push(action);
                    }
                }
            }
            let mut permitted = Vec::new();
            for (text, (holder, held)) in holders {
                let may: Vec<String> = actions
                    .iter()
                    .filter(|action| held.contains(action))
                    .filter(|action| {
                        let request = ExerciseRequest {
                            caller: holder,
                            route,
                            resource: resource.clone(),
                            action: (*action).clone(),
                        };
                        judged
                            .grants
                            .explain_in(&frame, judged.directory, &request, at)
                            .is_ok()
                    })
                    .map(ToString::to_string)
                    .collect();
                if !may.is_empty() {
                    permitted.push(ReachHolder {
                        holder: text,
                        actions: may,
                    });
                }
            }
            resources.push(ReachView {
                kind: wire.kind.clone(),
                id: wire.id.clone(),
                holders: permitted,
            });
        }
        Ok(Json(ReachAnswer {
            revision: frame.revision(),
            resources,
        }))
    })
}
