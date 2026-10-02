//! The compiled screens, served from a directory beside the service's own
//! routes: the screens at `/`, the routes under `/api`, one origin, so the
//! session cookie is first-party and nothing else runs at runtime.
//!
//! A path with a file extension names a file and is answered 404 when the
//! file is absent. A path without one is a screen route the page itself
//! reads, so it is answered with the page. A path that steps outside the
//! directory is never read.
//!
//! Every path under `/api` is the API's, and one it has no route for is
//! refused by [`not_an_api_route`], never answered with the page, whether or
//! not the screens are served. Paths outside `/api` stay the page's (ADR-129).

use std::path::{Component, Path};
use std::sync::Arc;

use axum::extract::{OriginalUri, Path as UrlPath, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};

use crate::error::ServerError;

#[path = "surface_cache.rs"]
mod cache;
use cache::Screens;

/// The page every screen route is answered with.
pub const PAGE: &str = "index.html";

/// `api` under `/api`, and the screens in `dir` at the root.
pub fn serving(dir: &Path, api: Router) -> Result<Router, ServerError> {
    let installed = Arc::new(Screens::load(dir)?);
    let screens = Router::new()
        .route("/", get(page))
        .route("/{*path}", get(file))
        .with_state(installed);
    // Reserve the whole API mount, including unknown paths. A route-only nest
    // can otherwise lose an unknown API path to the public screen wildcard.
    Ok(Router::new().nest_service("/api", api).merge(screens))
}

/// The API's answer to a path it has no route for: 404, and a refusal whose
/// error names the path as not an API route.
pub async fn not_an_api_route(OriginalUri(uri): OriginalUri) -> Response {
    let body = serde_json::json!({
        "refusal": "NotAnApiRoute",
        "error": format!("{} is not an API route", uri.path()),
    });
    (StatusCode::NOT_FOUND, Json(body)).into_response()
}

async fn page(State(screens): State<Arc<Screens>>, headers: HeaderMap) -> Response {
    answer(&screens, Path::new(PAGE), &headers)
}

async fn file(
    State(screens): State<Arc<Screens>>,
    UrlPath(path): UrlPath<String>,
    headers: HeaderMap,
) -> Response {
    let relative = Path::new(&path);
    let inside = relative
        .components()
        .all(|part| matches!(part, Component::Normal(_)));
    if !inside {
        return StatusCode::NOT_FOUND.into_response();
    }
    if relative.extension().is_some() {
        return answer(&screens, relative, &headers);
    }
    answer(&screens, Path::new(PAGE), &headers)
}

fn answer(screens: &Screens, relative: &Path, headers: &HeaderMap) -> Response {
    match screens.get(relative) {
        Some(asset) => {
            let mut matched = false;
            for value in headers.get_all(header::IF_NONE_MATCH) {
                let Ok(value) = value.to_str() else {
                    return (
                        StatusCode::BAD_REQUEST,
                        "ScreenEtagMalformed: the conditional tag is not text",
                    )
                        .into_response();
                };
                matched |= value.split(',').any(|tag| {
                    let tag = tag.trim();
                    let tag = match tag.strip_prefix("W/") {
                        Some(tag) => tag,
                        None => tag,
                    };
                    tag == "*" || asset.etag == tag
                });
            }
            let mut answer = if matched {
                StatusCode::NOT_MODIFIED.into_response()
            } else {
                asset.bytes.clone().into_response()
            };
            answer.headers_mut().insert(
                header::CONTENT_TYPE,
                HeaderValue::from_static(content_type(relative)),
            );
            answer.headers_mut().insert(
                header::CACHE_CONTROL,
                HeaderValue::from_static(cache_control(relative)),
            );
            answer
                .headers_mut()
                .insert(header::ETAG, asset.etag.clone());
            answer
        }
        None if relative == Path::new(PAGE) => (
            StatusCode::NOT_FOUND,
            "the screens are not installed; run lys identity install with --surface",
        )
            .into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

/// The media type by the file's extension.
pub fn content_type(path: &Path) -> &'static str {
    match path.extension().and_then(|ext| ext.to_str()) {
        Some("html") => "text/html; charset=utf-8",
        Some("js" | "mjs") => "text/javascript; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("json" | "map") => "application/json",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("ico") => "image/x-icon",
        Some("woff2") => "font/woff2",
        Some("woff") => "font/woff",
        Some("txt") => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

/// Hashed assets are kept for a year; the page and everything else are
/// asked for again each time.
pub fn cache_control(path: &Path) -> &'static str {
    if path.starts_with("assets") {
        "public, max-age=31536000, immutable"
    } else {
        "no-cache"
    }
}

#[cfg(test)]
#[path = "surface_tests.rs"]
mod tests;
