//! Read-only routes a screen reads: the sealed entries without their values,
//! the grants, and the checked audit log. None of them carries a secret
//! byte, a handle or a digest.

use std::sync::{Arc, PoisonError};

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use serde_json::{Value, json};

use crate::serve::Shared;

type Answer = Result<Json<Value>, (StatusCode, String)>;

fn failed(error: &lys_secrets::SecretsError) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}\n"))
}

pub async fn secrets(State(shared): State<Arc<Shared>>) -> Answer {
    let routes = shared.layout.routes().map_err(|error| failed(&error))?;
    let broker = shared.broker.lock().unwrap_or_else(PoisonError::into_inner);
    let entries: Vec<Value> = broker
        .store()
        .entries()
        .map(|entry| {
            let route = routes.get(&entry.name);
            json!({
                "name": entry.name,
                "class": entry.class,
                "owner": entry.owner,
                "sequence": entry.sequence,
                "upstream": route.map(|route| route.upstream.clone()),
                "header": route.map(|route| route.header.clone()),
            })
        })
        .collect();
    Ok(Json(json!({ "secrets": entries })))
}

pub async fn grants(State(shared): State<Arc<Shared>>) -> Answer {
    let broker = shared.broker.lock().unwrap_or_else(PoisonError::into_inner);
    let rows = broker
        .permissions()
        .list()
        .map_err(|error| failed(&error))?;
    let grants: Vec<Value> = rows
        .into_iter()
        .map(|(identity, secret, granted_by)| json!({ "identity": identity, "secret": secret, "granted_by": granted_by }))
        .collect();
    Ok(Json(json!({ "grants": grants })))
}

pub async fn audit(State(shared): State<Arc<Shared>>) -> Answer {
    let broker = shared.broker.lock().unwrap_or_else(PoisonError::into_inner);
    let lines = broker.audit().replay().map_err(|error| failed(&error))?;
    let lines: Vec<Value> = lines
        .into_iter()
        .map(|recorded| {
            let line = recorded.line;
            json!({
                "index": recorded.index,
                "kind": line.kind.label(),
                "at_ms": line.at_ms,
                "handle": line.handle,
                "identity": line.identity,
                "secret": line.secret,
                "operation": line.operation,
                "uses": line.uses,
                "outcome": line.outcome,
            })
        })
        .collect();
    Ok(Json(
        json!({ "verified_by": lys_secrets::to_hex(&broker.audit().verifying_key()), "lines": lines }),
    ))
}
