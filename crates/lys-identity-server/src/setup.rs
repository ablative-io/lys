//! First-run setup for the configured administrator; the verified session supplies the login.

use std::str::FromStr;
use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::extract::rejection::JsonRejection;
use axum::http::HeaderMap;
use lys_identity::{OperationId, Profile};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::error::ServerError;
use crate::routes::{AppState, receipt_json, signed_in, with_directory};
use crate::session::now;

/// Browser-owned operation id and the name to display; identity claims are forbidden.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetupRequest {
    /// Retained across retries of this setup act.
    pub operation: String,
    /// The name shown in the directory, including single names.
    pub display_name: String,
}

/// Complete setup in one signed log write, with no implied resource grants.
///
/// The caller is admitted before the body is read, so a body the route does
/// not take is refused `RequestMalformed` only to the administrator.
pub async fn finish(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Result<Json<SetupRequest>, JsonRejection>,
) -> Result<Json<Value>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    state.admission.administrator(&actor)?;
    let Json(body) = body.map_err(|refused| ServerError::RequestMalformed {
        reason: refused.body_text(),
    })?;
    let operation = OperationId::from_str(&body.operation)?;
    let profile = Profile::new(&body.display_name)?;
    with_directory(&state, |directory| {
        let (person, receipt) = directory.setup_person(actor, operation, profile, now())?;
        Ok(Json(
            json!({ "person": person.to_string(), "receipt": receipt_json(&receipt) }),
        ))
    })
}
