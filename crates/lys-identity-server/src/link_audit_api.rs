//! The authenticated link-audit receiver endpoint over lys-identity's core (R4).

use std::str::FromStr;
use std::sync::Arc;

use axum::extract::State;
use axum::http::HeaderMap;
use axum::routing::post;
use axum::{Json, Router};
use lys_identity::{LinkChange, LinkObservation, LoginBinding, PersonId};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::error::ServerError;
use crate::routes::{AppState, receipt_json, signed_in, with_directory};
use crate::session::now;

/// The link-audit route.
pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/link-audit", post(deliver))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Delivery {
    person: String,
    source_operation_id: String,
    change: String,
    issuer: String,
    subject: String,
    observer: String,
    observed_at: u64,
}

async fn deliver(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<Delivery>,
) -> Result<Json<Value>, ServerError> {
    let source = signed_in(&state, &headers)?;
    state.admission.link_audit_source(&source)?;
    let change = match body.change.as_str() {
        "linked" => LinkChange::Linked,
        "unlinked" => LinkChange::Unlinked,
        other => {
            return Err(ServerError::RequestMalformed {
                reason: format!("{other} is not linked or unlinked"),
            });
        }
    };
    let person = PersonId::from_str(&body.person)?;
    let observation = LinkObservation::new(
        &body.source_operation_id,
        change,
        LoginBinding::new(&body.issuer, &body.subject)?,
        &body.observer,
        body.observed_at,
    )?;
    with_directory(&state, |directory| {
        let receipt = directory.accept_link_audit(source, person, observation, now())?;
        Ok(Json(json!({ "receipt": receipt_json(&receipt) })))
    })
}
