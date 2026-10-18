//! `GET /operations/{operation}`: whether Lys recorded a change the signed-in
//! caller sent under `operation`, and what it made.
//!
//! The answer is read from the records that already carry operation ids; no
//! store is written for it:
//!
//! - `grant`: the grant the grants' log issued under the operation, made by
//!   its issuer, unless that grant was an approval's;
//! - `decision`: an approval, whose operation is the one its grant was issued
//!   under, made by the person who decided; its id is the request's;
//! - `request`: the request kept under the operation, made by its asker;
//! - `role_change`: a job made or revised (a version) or a holder assigned
//!   (a holding) under the operation, made by who made or assigned it; its id
//!   is the job's.
//!
//! A record made by the caller is answered `{recorded: true, kind, id}`. No
//! record under the operation is `{recorded: false}`; an event the grants
//! refused made no record. A record made by anyone else is refused as
//! `NotAdmitted` and says nothing of the record, so no one learns another
//! person's operations. A decline carries no operation of its own and is not
//! answered here. A store the configuration does not name holds nothing; a
//! store that cannot be read is refused by its own name, never read as nothing.

use std::str::FromStr;
use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::routing::get;
use axum::{Json, Router};
use lys_identity::{IdentityId, OperationId};
use serde::Serialize;

use crate::error::ServerError;
use crate::grants::{Judged, with_grants};
use crate::routes::AppState;

/// The operations route.
pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/operations/{operation}", get(operation))
}

pub(crate) fn types(api: &mut lys_openapi::Api) -> Vec<crate::openapi_types::Entry> {
    vec![(
        lys_openapi::Method::Get,
        "/operations/{operation}",
        None,
        Some(api.schema::<OperationView>()),
    )]
}

/// Whether an operation of the caller's was recorded, and what it made.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
pub struct OperationView {
    /// Whether a record carrying the operation exists.
    pub recorded: bool,
    /// `grant`, `request`, `decision` or `role_change`; absent when not recorded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<&'static str>,
    /// The grant's, the request's or the job's id; absent when not recorded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
}

/// A record kept under an operation: what it is, its id, and who made it.
struct Made {
    kind: &'static str,
    id: String,
    by: String,
}

async fn operation(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(operation): Path<String>,
) -> Result<Json<OperationView>, ServerError> {
    let operation = OperationId::from_str(&operation)?;
    with_grants(&state, |judged| {
        let caller = crate::grants::caller(&state, &headers, judged.directory)?;
        let made = match granted(&state, &judged, operation)? {
            Some(made) => Some(made),
            None => match asked(&state, operation)? {
                Some(made) => Some(made),
                None => role_change(&state, operation)?,
            },
        };
        match made {
            None => Ok(Json(OperationView {
                recorded: false,
                kind: None,
                id: None,
            })),
            Some(made) if made.by == caller.to_string() => Ok(Json(OperationView {
                recorded: true,
                kind: Some(made.kind),
                id: Some(made.id),
            })),
            Some(_) => Err(ServerError::NotAdmitted {
                reason: "this operation was not sent by the signed-in caller",
            }),
        }
    })
}

/// The grant issued under `operation`, as an approval's decision when a
/// request's approval issued it.
fn granted(
    state: &AppState,
    judged: &Judged<'_>,
    operation: OperationId,
) -> Result<Option<Made>, ServerError> {
    let book = judged.grants.book();
    let Some(index) = book.operation(operation) else {
        return Ok(None);
    };
    let Some(grant) = book
        .records()
        .find(|record| record.index() == index)
        .map(lys_identity::grants::GrantRecord::grant)
    else {
        return Ok(None);
    };
    let id = grant.id().to_string();
    let decision = with_store(state.requests.as_ref(), |store| {
        Ok(store
            .requests()
            .filter_map(|(_, decided)| decided)
            .find(|decided| decided.approved && decided.grant.as_deref() == Some(id.as_str()))
            .map(|decided| Made {
                kind: "decision",
                id: decided.id.clone(),
                by: decided.by.clone(),
            }))
    })?;
    Ok(Some(decision.flatten().unwrap_or(Made {
        kind: "grant",
        id,
        by: IdentityId::to_string(&grant.parts().issuer),
    })))
}

/// The request kept under `operation`: a request's id is its operation.
fn asked(state: &AppState, operation: OperationId) -> Result<Option<Made>, ServerError> {
    let id = operation.to_string();
    Ok(with_store(state.requests.as_ref(), |store| {
        Ok(store.request(&id).map(|(asked, _)| Made {
            kind: "request",
            id: asked.id.clone(),
            by: asked.asked_by.clone(),
        }))
    })?
    .flatten())
}

/// The job made, revised or assigned under `operation`.
fn role_change(state: &AppState, operation: OperationId) -> Result<Option<Made>, ServerError> {
    let operation = operation.to_string();
    Ok(with_store(state.roles.as_ref(), |store| {
        Ok(store.roles().iter().find_map(|role| {
            let made = |by: &str| Made {
                kind: "role_change",
                id: role.id.clone(),
                by: by.to_owned(),
            };
            role.versions
                .iter()
                .find(|version| version.operation == operation)
                .map(|version| made(&version.made_by))
                .or_else(|| {
                    role.holdings
                        .iter()
                        .find(|holding| holding.operation == operation)
                        .map(|holding| made(&holding.assigned_by))
                })
        }))
    })?
    .flatten())
}

/// `act` over a settled store, or nothing when the configuration names none.
fn with_store<S: Settled, T>(
    store: Option<&std::sync::Mutex<S>>,
    act: impl FnOnce(&S) -> Result<T, ServerError>,
) -> Result<Option<T>, ServerError> {
    let Some(store) = store else {
        return Ok(None);
    };
    let mut store = store
        .lock()
        .map_err(|error| S::unavailable(&error.to_string()))?;
    store.settle()?;
    act(&store).map(Some)
}

/// A store read here: settled before it is read, refused by its own name.
trait Settled {
    fn settle(&mut self) -> Result<(), ServerError>;
    fn unavailable(reason: &str) -> ServerError;
}

impl Settled for crate::requests_store::RequestStore {
    fn settle(&mut self) -> Result<(), ServerError> {
        crate::requests_store::RequestStore::settle(self)
    }
    fn unavailable(reason: &str) -> ServerError {
        ServerError::RequestsUnavailable {
            reason: format!("the requests lock is poisoned: {reason}"),
        }
    }
}

impl Settled for crate::roles_store::RolesStore {
    fn settle(&mut self) -> Result<(), ServerError> {
        crate::roles_store::RolesStore::settle(self)
    }
    fn unavailable(reason: &str) -> ServerError {
        ServerError::RolesUnavailable {
            reason: format!("the roles lock is poisoned: {reason}"),
        }
    }
}
