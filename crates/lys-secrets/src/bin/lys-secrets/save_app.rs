//! Prepare an approved app's credentials as the authenticated person, never
//! under a supplied owner. The broker makes the client secret itself and
//! seals it; no route takes or answers it (DIRECTORY-081).
use std::sync::Arc;

use axum::Json;
use axum::body::Bytes;
use axum::extract::{Request, State};
use axum::http::StatusCode;
use lys_secrets::SecretsError;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::callers::{caller, refused};
use crate::files::Route;
use crate::serve::{MAX_BODY, Shared, on_broker};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Prepare {
    app: String,
    upstream: String,
}

type Answer = Result<Json<Value>, (StatusCode, String)>;

/// Prepare is available only through a trusted screen service for a person.
/// The values it seals are never returned or logged; the answer carries
/// their references and the client secret's SHA-256.
pub async fn prepare(State(shared): State<Arc<Shared>>, request: Request) -> Answer {
    let (parts, body) = request.into_parts();
    // Guarded: read before the caller is known, as the caller's signature covers it.
    let body: Bytes = axum::body::to_bytes(body, MAX_BODY)
        .await
        .map_err(|_error| {
            (
                StatusCode::BAD_REQUEST,
                "RequestMalformed: unreadable body".to_owned(),
            )
        })?;
    let who = caller(&shared, &parts, &body).await?;
    if who.via.is_none() {
        return Err((
            StatusCode::FORBIDDEN,
            "NotAdmitted: a trusted screen service is required".to_owned(),
        ));
    }
    let asked: Prepare = serde_json::from_slice(&body).map_err(|_error| {
        (
            StatusCode::BAD_REQUEST,
            "RequestMalformed: invalid app credential preparation".to_owned(),
        )
    })?;
    if !valid_app(&asked.app) {
        return Err((
            StatusCode::BAD_REQUEST,
            "RequestMalformed: invalid app credential preparation".to_owned(),
        ));
    }
    let url = reqwest::Url::parse(&asked.upstream).map_err(|_error| {
        (
            StatusCode::BAD_REQUEST,
            "RequestMalformed: invalid identity upstream".to_owned(),
        )
    })?;
    if url.scheme() != "http"
        || !matches!(url.host_str(), Some("127.0.0.1" | "[::1]"))
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err((
            StatusCode::BAD_REQUEST,
            "RequestMalformed: identity upstream must be loopback".to_owned(),
        ));
    }
    let layout = shared.layout.clone();
    on_broker(&shared, move |broker| {
        let prefix = format!("lys-app-{}-{}", who.identity, asked.app);
        let client = format!("{prefix}-client");
        let api = format!("{prefix}-api");
        let digest = broker.prepare_app(&asked.app, &who.identity)?;
        layout.add_route(
            &api,
            Route {
                upstream: asked.upstream,
                header: "authorization".to_owned(),
                prefix: "Bearer ".to_owned(),
                spend_header: None,
            },
        )?;
        Ok::<_, SecretsError>(Json(json!({
            "app": asked.app,
            "client_secret_ref": client,
            "api_credential_ref": api,
            "client_secret_sha256": digest,
        })))
    })
    .await
    .map_err(|error| refused(&error))?
    .map_err(|error| refused(&error))
}

fn valid_app(app: &str) -> bool {
    (3..=40).contains(&app.len())
        && app.starts_with(|c: char| c.is_ascii_lowercase())
        && app
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}
