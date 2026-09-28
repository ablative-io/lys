//! The read-only receipt and inclusion-verification endpoints (R2).
//!
//! A receipt is public evidence: it carries no secret, so reading one needs no
//! session. The endpoint answers the receipt, the signed message, the log's
//! current checkpoint and an inclusion proof of the leaf in it, which is all a
//! verifier needs beside the service's public key.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};
use serde_json::{Value, json};

use crate::error::ServerError;
use crate::routes::{AppState, hex, receipt_json, with_directory};

/// The receipt routes.
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/receipts/{index}", get(receipt))
        .route("/service-key", get(service_key))
}

async fn service_key(State(state): State<Arc<AppState>>) -> Result<Json<Value>, ServerError> {
    with_directory(&state, |directory| {
        Ok(Json(json!({ "ed25519": hex(&directory.service_key()) })))
    })
}

async fn receipt(
    State(state): State<Arc<AppState>>,
    Path(index): Path<u64>,
) -> Result<Json<Value>, ServerError> {
    with_directory(&state, |directory| {
        let log = directory.log()?;
        let message =
            log.leaf(index)?
                .as_deref()
                .map(hex)
                .ok_or_else(|| ServerError::RequestMalformed {
                    reason: format!("the log holds no leaf {index}"),
                })?;
        let (tree_size, root) = log.head()?;
        let proof = log.inclusion_proof(index)?;
        let receipt =
            directory
                .receipt_at(index)?
                .ok_or_else(|| ServerError::RequestMalformed {
                    reason: format!("the directory holds no receipt for leaf {index}"),
                })?;
        Ok(Json(json!({
            "receipt": receipt_json(&receipt),
            "message": message,
            "checkpoint": { "tree_size": tree_size, "root": hex(&root) },
            "inclusion_proof": hex(proof.as_bytes()),
        })))
    })
}
