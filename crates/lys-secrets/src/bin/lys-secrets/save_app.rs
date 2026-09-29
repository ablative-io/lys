//! Store app credentials as the authenticated person, never under a supplied owner.
use std::sync::Arc;

use axum::Json;
use axum::body::Bytes;
use axum::extract::{Request, State};
use axum::http::StatusCode;
use lys_secrets::{EntryClass, Secret, SecretsError};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::callers::{caller, refused};
use crate::files::Route;
use crate::serve::{MAX_BODY, Shared, on_broker};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Save {
    app: String,
    client_secret: String,
    upstream: String,
}

type Answer = Result<Json<Value>, (StatusCode, String)>;

/// Save is available only through a trusted screen service for a person.
/// Values travel in the signed body and are never returned or logged.
pub async fn save(State(shared): State<Arc<Shared>>, request: Request) -> Answer {
    let (parts, body) = request.into_parts();
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
    let asked: Save = serde_json::from_slice(&body).map_err(|_error| {
        (
            StatusCode::BAD_REQUEST,
            "RequestMalformed: invalid app credential save".to_owned(),
        )
    })?;
    if !valid_app(&asked.app)
        || asked.client_secret.len() != 64
        || !asked.client_secret.bytes().all(|b| b.is_ascii_hexdigit())
    {
        return Err((
            StatusCode::BAD_REQUEST,
            "RequestMalformed: invalid app credential save".to_owned(),
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
        // Each entry reconciles independently after a partial or lost response.
        broker.seal_once(
            &client,
            EntryClass::Key,
            &who.identity,
            &Secret::from_slice(asked.client_secret.as_bytes()),
        )?;
        let credential =
            Secret::from_slice(format!("lys-app.{}.{}", asked.app, asked.client_secret).as_bytes());
        broker.seal_once(&api, EntryClass::Credential, &who.identity, &credential)?;
        layout.add_route(
            &api,
            Route {
                upstream: asked.upstream,
                header: "authorization".to_owned(),
                prefix: "Bearer ".to_owned(),
                spend_header: None,
            },
        )?;
        Ok::<_, SecretsError>(Json(
            json!({"app": asked.app, "client_secret_ref": client, "api_credential_ref": api}),
        ))
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
