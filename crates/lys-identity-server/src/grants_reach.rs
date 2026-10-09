//! Who may act on many resources at once, for the permission graph.
//!
//! `POST /grants/reach` takes one or more resources, each with the
//! actions asked about, and answers for each resource every holder the
//! caller may see with the actions it may take, in holder order. It is
//! `/grants/who` asked of every resource and action together: the same
//! holders, the same decisions, all made at the one revision the answer
//! names, from one reading of the permission engine, so the graph costs one
//! read rather than one per holder, action and resource. Each action is
//! answered with the mode of the grant it rests on, so a screen names a held
//! right as held (ACCESS-001 R4). It is a question: nothing is recorded.
//!
//! A grant naming a role is counted as the role's actions under the schema
//! version current now (ACCESS-004 R1). A resource placed restricted
//! (ACCESS-004 R2) is answered with `restricted` true, so the reach view
//! shows it apart: nothing held on its parent reaches it.

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::http::HeaderMap;
use lys_identity::IdentityId;
use lys_identity::grants::{Action, ExerciseRequest, Grant, Resource};
use serde::{Deserialize, Serialize};

use crate::error::ServerError;
use crate::grant_contract::RouteWire;
use crate::grant_sight::sees_with;
use crate::grants::with_grants;
use crate::grants_batch::unanswered;
use crate::session::now;

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
    /// Each action's mode, beside it: `outright`, `by_draft` or `by_two`,
    /// the mode of the grant the action rests on.
    pub modes: Vec<&'static str>,
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
    /// Whether the resource is placed in its parent restricted, so only a
    /// grant on it reaches it (ACCESS-004 R2).
    pub restricted: bool,
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
    if body.resources.is_empty() {
        return Err(ServerError::RequestMalformed {
            reason: "resources names at least one resource".to_owned(),
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
        let frame = judged.grants.frame(judged.directory, None)?;
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
                let held = judged.grants.model().actions_of(record.grant());
                for action in &actions {
                    if held.contains(action) && !entry.1.contains(&action) {
                        entry.1.push(action);
                    }
                }
            }
            let mut permitted = Vec::new();
            for (text, (holder, held)) in holders {
                let mut may = Vec::new();
                let mut modes = Vec::new();
                for action in actions.iter().filter(|action| held.contains(action)) {
                    let request = ExerciseRequest {
                        caller: holder,
                        route,
                        resource: resource.clone(),
                        action: action.clone(),
                    };
                    match judged.grants.explain_in(&frame, &request, at) {
                        // A permit naming a grant the book no longer holds is
                        // answered as not permitted, as `which` answers it.
                        Ok(permit) => {
                            if let Some(grant) = judged.grants.book().grant(permit.grant) {
                                may.push(action.to_string());
                                modes.push(Grant::mode(grant).as_str());
                            }
                        }
                        Err(error) if unanswered(&error) => return Err(error.into()),
                        Err(_) => {}
                    }
                }
                if !may.is_empty() {
                    permitted.push(ReachHolder {
                        holder: text,
                        actions: may,
                        modes,
                    });
                }
            }
            resources.push(ReachView {
                kind: wire.kind.clone(),
                id: wire.id.clone(),
                holders: permitted,
                restricted: judged.apps.is_restricted(&resource),
            });
        }
        Ok(Json(ReachAnswer {
            revision: frame.revision(),
            resources,
        }))
    })
}
