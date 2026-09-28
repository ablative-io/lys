//! Every refusal a test meets is one the `OpenAPI` document lists for the
//! route that answered it.
//!
//! The harness serves the service behind [`listed_only`]. For every answer
//! that is not a success, it reads the route the request was matched to and
//! the refusal's name, and when the document does not list that name for that
//! route it replaces the answer with the refusal `RefusalUndocumented`, at
//! 500, naming the route and the refusal. A test that expected the refusal
//! then fails naming what the document leaves out. With
//! `tests/openapi.rs`, which fails on a listed refusal no test produces, the
//! lists are held complete from both sides: nothing listed that no test
//! meets, nothing met that is not listed.
//!
//! The lists are the service's, read from `lys_identity_server::openapi::api`,
//! and the refusal is the one the service answered: neither is written here.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use axum::Json;
use axum::body::Body;
use axum::extract::{MatchedPath, Request};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use serde_json::{Value, json};

/// Each documented route's refusals, by method and path template.
pub type Listed = Arc<BTreeMap<(String, String), BTreeSet<String>>>;

/// The refusals the service's document lists, by route.
pub fn listed() -> Listed {
    let routes = lys_identity_server::openapi::api()
        .routes()
        .iter()
        .map(|route| {
            (
                (route.method.word().to_owned(), route.path.to_owned()),
                route
                    .refusals
                    .iter()
                    .map(|name| (*name).to_owned())
                    .collect(),
            )
        })
        .collect();
    Arc::new(routes)
}

fn undocumented(reason: &str) -> Response {
    let body = json!({
        "refusal": "RefusalUndocumented",
        "reason": format!("RefusalUndocumented: {reason}"),
        "fields": [],
    });
    (StatusCode::INTERNAL_SERVER_ERROR, Json(body)).into_response()
}

/// Answer the service's answer to `request`, unless it is a refusal the
/// document does not list for the route that answered it.
pub async fn listed_only(listed: Listed, request: Request, next: Next) -> Response {
    let method = request.method().as_str().to_ascii_lowercase();
    let path = request
        .extensions()
        .get::<MatchedPath>()
        .map(|matched| matched.as_str().to_owned());
    let response = next.run(request).await;
    if response.status().is_success() {
        return response;
    }
    let Some(allowed) = path.and_then(|path| {
        let key = (method, path);
        listed.get(&key).map(|allowed| (key, allowed))
    }) else {
        return response;
    };
    let ((method, path), allowed) = allowed;
    let (parts, body) = response.into_parts();
    let bytes = match axum::body::to_bytes(body, usize::MAX).await {
        Ok(bytes) => bytes,
        Err(error) => {
            return undocumented(&format!(
                "{method} {path} answered a body that could not be read: {error}"
            ));
        }
    };
    let name = serde_json::from_slice::<Value>(&bytes)
        .ok()
        .and_then(|answer| answer["refusal"].as_str().map(str::to_owned));
    match name {
        Some(name) if !allowed.contains(&name) => undocumented(&format!(
            "{method} {path} answered the refusal {name}, which the OpenAPI document does not list for it"
        )),
        _ => Response::from_parts(parts, Body::from(bytes)),
    }
}
