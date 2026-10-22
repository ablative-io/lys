//! Reviewed version selection and retirement retain a seat's operation history.

use std::str::FromStr;
use std::sync::Arc;

use axum::Json;
use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use lys_identity::OperationId;
use serde::Serialize;

use crate::error::ServerError;
use crate::error_seat::SeatError;
use crate::provisioning_api::with_provisioning;
use crate::routes::{AppState, signed_in};
use crate::seats_api::{body, checked_name, shown, with_seats};
use crate::seats_state::{Line, Moved, Retired};
use crate::seats_views::{SeatOperationBody, SeatVersionBody, SeatView};
use crate::session::now;

/// The recorded retirement operation, independent of later name reuse.
#[derive(Serialize, utoipa::ToSchema)]
pub(crate) struct SeatRetired {
    /// The operation that retired the seat.
    pub operation: String,
    /// The name released by that operation.
    pub name: String,
}

pub(crate) async fn version(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(name): Path<String>,
    given: Result<Json<SeatVersionBody>, JsonRejection>,
) -> Result<Json<SeatView>, ServerError> {
    let given = body(given)?;
    checked_name(&name)?;
    OperationId::from_str(&given.operation)?;
    let actor = signed_in(&state, &headers)?;
    let held = crate::seats_api::seat(&state, &name)?;
    let by = crate::launch_api::start_caller(&state, &headers, &actor, &held.added.agent)?;
    // A review carries by content to the same person's agents (AGENTS-017 R3).
    let kin = crate::start_checks::kin(&state, &held.added.agent)?;
    let harness = with_provisioning(&state, |store| {
        let selected = store
            .version(&held.added.agent, given.profile_version)
            .ok_or(ServerError::ProfileVersionUnknown {
                version: given.profile_version,
            })?;
        crate::start_checks::reviewed_for(store, &kin, selected)?;
        selected
            .settings
            .harness
            .as_ref()
            .map(|harness| harness.name.clone())
            .ok_or(ServerError::HarnessUndeclared {
                version: given.profile_version,
            })
    })?;
    let updated = with_seats(&state, |store| {
        let current = store.named(&name)?;
        if current.added.operation != held.added.operation {
            return Err(SeatError::OperationReused {
                operation: given.operation,
            }
            .into());
        }
        store.keep(Line::Moved(Moved {
            operation: given.operation,
            name: name.clone(),
            profile_version: given.profile_version,
            harness,
            by,
            at: now(),
        }))?;
        store.named(&name)
    })?;
    Ok(Json(shown(&state, &updated).await?))
}

pub(crate) async fn retire(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(name): Path<String>,
    given: Result<Json<SeatOperationBody>, JsonRejection>,
) -> Result<Json<SeatRetired>, ServerError> {
    let given = body(given)?;
    checked_name(&name)?;
    OperationId::from_str(&given.operation)?;
    let actor = signed_in(&state, &headers)?;
    let held = with_seats(&state, |store| {
        store
            .seat(&name)
            .cloned()
            .ok_or_else(|| SeatError::Unknown { name: name.clone() }.into())
    })?;
    // An agent the door admitted holds seat.retire on the seat by its own
    // grant (AGENTS-019 R4), recorded by its own name.
    let by = match crate::routes::door_agent(&state, &actor)? {
        Some(agent) => agent.to_string(),
        None => crate::launch_api::start_caller(&state, &headers, &actor, &held.added.agent)?,
    };
    with_seats(&state, |store| {
        let current = store
            .seat(&name)
            .ok_or_else(|| SeatError::Unknown { name: name.clone() })?;
        if current.added.operation != held.added.operation {
            return Err(SeatError::OperationReused {
                operation: given.operation.clone(),
            }
            .into());
        }
        store.keep(Line::Retired(Retired {
            operation: given.operation.clone(),
            name: name.clone(),
            by,
            at: now(),
        }))?;
        Ok(())
    })?;
    Ok(Json(SeatRetired {
        operation: given.operation,
        name,
    }))
}
