//! Lys reads an app's manifest itself for the approve page (DIRECTORY-102
//! R3), so no browser rule (CORS, mixed content) stands between the page and
//! the app: `GET /apps/connect/manifest?address=` answers the manifest, or
//! one named reason, each with what the page says next.
//!
//! - `manifest_not_answering`: nothing answered, with the connection
//!   error's words.
//! - `manifest_answered_status`: the app answered a status that is not a
//!   success, with the status. A redirect is one: Lys follows no redirect,
//!   so no app can send this read to any other address, Lys's own loopback
//!   or private ones among them.
//! - `manifest_not_a_manifest`: the answer is not a manifest, described by
//!   its content type and size and never repeated.
//! - `manifest_address_refused`: the address is not an app's manifest
//!   address (http or https, its path ending `/identity/lys/manifest`), so
//!   the read cannot be pointed at any other endpoint.
//!
//! Only the administrator is answered. No bound in seconds is put on the
//! read: a person who stops waiting leaves the page, which ends it.

use std::sync::Arc;

use axum::Json;
use axum::extract::rejection::QueryRejection;
use axum::extract::{Query, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::{Manifest, malformed, unavailable, with_apps};
use crate::error::ServerError;
use crate::routes::AppState;

/// The path every app serves its manifest at, under its own address.
const MANIFEST_PATH: &str = "/identity/lys/manifest";
/// The most of an answer Lys reads; a manifest is a few kilobytes.
const MOST_BYTES: usize = 256 * 1024;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ManifestQuery {
    address: String,
}

/// The manifest Lys read, with the origin it read it from.
#[derive(Serialize, utoipa::ToSchema)]
pub(crate) struct ManifestRead {
    manifest: Manifest,
    origin: String,
}

/// A named reason the manifest was not read, with the fields that say it.
fn unread(status: StatusCode, refusal: &str, reason: &str, more: Value) -> Response {
    let mut body = json!({"refusal": refusal, "reason": reason});
    if let (Some(body), Value::Object(more)) = (body.as_object_mut(), more) {
        body.extend(more);
    }
    (status, Json(body)).into_response()
}

/// The address, when it is an app's manifest address, or why not.
fn admitted(address: &str) -> Result<reqwest::Url, String> {
    let url = reqwest::Url::parse(address)
        .map_err(|error| format!("`{address}` is not a web address ({error})"))?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err(format!(
            "`{address}` is not an http or https address, and Lys reads a manifest only over those"
        ));
    }
    if !url.username().is_empty() || url.password().is_some() || url.fragment().is_some() {
        return Err(format!(
            "`{address}` carries a sign-in or a fragment, which a manifest address never does"
        ));
    }
    if !url.path().ends_with(MANIFEST_PATH) || url.query().is_some() {
        return Err(format!(
            "`{address}` is not an app's manifest address, which ends {MANIFEST_PATH}"
        ));
    }
    Ok(url)
}

/// An error's words with every cause under it, as the connection said them.
fn words(error: &reqwest::Error) -> String {
    let mut said = error.to_string();
    let mut cause = std::error::Error::source(error);
    while let Some(under) = cause {
        said.push_str(": ");
        said.push_str(&under.to_string());
        cause = under.source();
    }
    said
}

/// What an answer that is not a manifest was, by its type and size.
fn found(kind: Option<&str>, bytes: usize, whole: bool) -> String {
    let size = if whole {
        format!("{bytes} bytes")
    } else {
        format!("over {} KiB", MOST_BYTES / 1024)
    };
    match kind {
        Some("application/json") => {
            format!(
                "JSON of {size} that is not a manifest (it has no id, name, return addresses and schema)"
            )
        }
        Some(kind) => format!("a {kind} answer of {size}"),
        None => format!("an answer of {size} with no content type"),
    }
}

pub(super) async fn read(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    query: Result<Query<ManifestQuery>, QueryRejection>,
) -> Result<Response, ServerError> {
    let Query(query) = query.map_err(|refused| malformed(refused.body_text()))?;
    with_apps(&state, |apps, directory| {
        crate::apps_api::administrator(&state, apps, &headers, directory)
    })?;
    let url = match admitted(&query.address) {
        Ok(url) => url,
        Err(reason) => {
            return Ok(unread(
                StatusCode::BAD_REQUEST,
                "manifest_address_refused",
                &reason,
                json!({}),
            ));
        }
    };
    let origin = url.origin().ascii_serialization();
    let mut call = crate::waits::Call::begin("app", &origin, "manifest", &query.address)?;
    let reading = async {
    let client = crate::outbound::http()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|error| unavailable(&format!("the manifest reader could not start: {error}")))?;
    let mut answer = match client
        .get(url)
        .header(header::ACCEPT, "application/json")
        .send()
        .await
    {
        Ok(answer) => answer,
        Err(error) => {
            let said = words(&error);
            return Ok(unread(
                StatusCode::BAD_GATEWAY,
                "manifest_not_answering",
                &format!("Lys could not reach {origin}: {said}"),
                json!({"error": said}),
            ));
        }
    };
    let status = answer.status();
    if !status.is_success() {
        return Ok(unread(
            StatusCode::BAD_GATEWAY,
            "manifest_answered_status",
            &format!(
                "the app at {origin} answered {} for its manifest",
                status.as_u16()
            ),
            json!({"status": status.as_u16()}),
        ));
    }
    let kind = answer
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .map(|value| value.trim().to_ascii_lowercase());
    let mut bytes = Vec::new();
    let mut whole = true;
    loop {
        match answer.chunk().await {
            Ok(Some(chunk)) if bytes.len() + chunk.len() <= MOST_BYTES => {
                bytes.extend_from_slice(&chunk);
            }
            Ok(Some(_over)) => {
                whole = false;
                break;
            }
            Ok(None) => break,
            Err(error) => {
                let said = words(&error);
                return Ok(unread(
                    StatusCode::BAD_GATEWAY,
                    "manifest_not_answering",
                    &format!("{origin} stopped answering partway through its manifest: {said}"),
                    json!({"error": said}),
                ));
            }
        }
    }
    let manifest = if whole {
        serde_json::from_slice::<Manifest>(&bytes).ok()
    } else {
        None
    };
    if let Some(manifest) = manifest {
        return Ok(Json(ManifestRead { manifest, origin }).into_response());
    }
    let found = found(kind.as_deref(), bytes.len(), whole);
    Ok(unread(
        StatusCode::BAD_GATEWAY,
        "manifest_not_a_manifest",
        &format!("the app at {origin} answered with {found}, not a manifest"),
        json!({"found": found}),
    ))
    };
    match call.until(reading).await {
        Ok(answer) => answer,
        Err(reason) => Ok(unread(StatusCode::SERVICE_UNAVAILABLE, "manifest_not_answering", &reason, json!({}))),
    }
}

#[cfg(test)]
#[path = "apps_connect_manifest_tests.rs"]
mod tests;
