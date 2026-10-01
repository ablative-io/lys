//! The read-only receipt and inclusion-verification endpoints (R2).
//!
//! A receipt is public evidence: it carries no secret, so reading one needs no
//! session. The endpoint answers the receipt, the signed message, the log's
//! current checkpoint and an inclusion proof of the leaf in it, which is all a
//! verifier needs beside the service's public key.
//!
//! An act on a session through a runner leaves a receipt of its own, read
//! at `/runner-receipts/{index}`: the act as its log keeps it, naming the
//! caller and the act and never the text typed, with the leaf's hash.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};

use crate::directory_views::{CheckpointView, ReceiptPage, ServiceKeyView, receipt_view};
use crate::error::ServerError;
use crate::routes::{AppState, hex, with_directory};
use crate::runner_acts::ActReceipt;

/// The receipt routes.
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/receipts/{index}", get(receipt))
        .route("/service-key", get(service_key))
        .route("/runner-receipts/{index}", get(act_receipt))
}

async fn service_key(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ServiceKeyView>, ServerError> {
    with_directory(&state, |directory| {
        Ok(Json(ServiceKeyView {
            ed25519: hex(&directory.service_key()),
        }))
    })
}

async fn receipt(
    State(state): State<Arc<AppState>>,
    Path(index): Path<u64>,
) -> Result<Json<ReceiptPage>, ServerError> {
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
        Ok(Json(ReceiptPage {
            receipt: receipt_view(&receipt),
            message,
            checkpoint: CheckpointView {
                tree_size,
                root: hex(&root),
            },
            inclusion_proof: hex(proof.as_bytes()),
        }))
    })
}

async fn act_receipt(
    State(state): State<Arc<AppState>>,
    Path(index): Path<u64>,
) -> Result<Json<ActReceipt>, ServerError> {
    let acts = state
        .acts
        .lock()
        .map_err(|error| ServerError::RuntimeUnavailable {
            reason: format!("the runner acts lock is poisoned: {error}"),
        })?;
    acts.receipt(index)?
        .map(Json)
        .ok_or_else(|| ServerError::RequestMalformed {
            reason: format!("the runner acts' log holds no act {index}"),
        })
}
