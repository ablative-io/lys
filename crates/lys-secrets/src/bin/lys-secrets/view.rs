//! Read-only routes a screen reads: the sealed entries without their values,
//! the grants, and the checked audit log. None of them carries a secret
//! byte, a handle or a digest. Each is asked by a handle's holder or by a
//! trusted screen service for a person (see `callers`), and answers only
//! with what that identity may discover; a secret outside its scope is left
//! out, as one not sealed.

use std::sync::{Arc, PoisonError};

use axum::Json;
use axum::extract::{Request, State};
use axum::http::StatusCode;
use serde_json::{Value, json};

use crate::callers::{self, refused};
use crate::serve::Shared;

type Answer = Result<Json<Value>, (StatusCode, String)>;

fn failed(error: &lys_secrets::SecretsError) -> (StatusCode, String) {
    refused(error)
}

/// The identity the request's caller speaks for. A screen route carries no
/// body, so the signature covers an empty one.
fn caller(shared: &Shared, request: Request) -> Result<String, (StatusCode, String)> {
    let (parts, _body) = request.into_parts();
    callers::caller(shared, &parts, &[]).map(|who| who.identity)
}

pub async fn secrets(State(shared): State<Arc<Shared>>, request: Request) -> Answer {
    let identity = caller(&shared, request)?;
    let routes = shared.layout.routes().map_err(|error| failed(&error))?;
    let broker = shared.broker.lock().unwrap_or_else(PoisonError::into_inner);
    let entries: Vec<Value> = broker
        .listing(&identity)
        .into_iter()
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

pub async fn grants(State(shared): State<Arc<Shared>>, request: Request) -> Answer {
    let identity = caller(&shared, request)?;
    let broker = shared.broker.lock().unwrap_or_else(PoisonError::into_inner);
    let rows = broker
        .permissions()
        .list()
        .map_err(|error| failed(&error))?;
    let grants: Vec<Value> = rows
        .into_iter()
        .filter(|(_identity, secret, _relation, _by)| broker.discovers(&identity, secret))
        .map(|(identity, secret, relation, granted_by)| {
            json!({ "identity": identity, "secret": secret, "relation": relation, "granted_by": granted_by })
        })
        .collect();
    Ok(Json(json!({ "grants": grants })))
}

pub async fn audit(State(shared): State<Arc<Shared>>, request: Request) -> Answer {
    let identity = caller(&shared, request)?;
    let broker = shared.broker.lock().unwrap_or_else(PoisonError::into_inner);
    let lines = broker.audit().replay().map_err(|error| failed(&error))?;
    let lines: Vec<Value> = lines
        .into_iter()
        .filter(|recorded| {
            recorded
                .line
                .secret
                .as_deref()
                .is_some_and(|secret| broker.discovers(&identity, secret))
        })
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
