//! The compiled screens, served from a directory beside the service's own
//! routes: the screens at `/`, the routes under `/api`, one origin, so the
//! session cookie is first-party and nothing else runs at runtime.
//!
//! A path with a file extension names a file and is answered 404 when the
//! file is absent. A path without one is a screen route the page itself
//! reads, so it is answered with the page. A path that steps outside the
//! directory is never read.

use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use axum::Router;
use axum::extract::{Path as UrlPath, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;

/// The page every screen route is answered with.
pub const PAGE: &str = "index.html";

/// `api` under `/api`, and the screens in `dir` at the root.
pub fn serving(dir: PathBuf, api: Router) -> Router {
    let screens = Router::new()
        .route("/", get(page))
        .route("/{*path}", get(file))
        .with_state(Arc::<Path>::from(dir));
    // Reserve the whole API mount, including unknown paths. A route-only nest
    // can otherwise lose an unknown API path to the public screen wildcard.
    Router::new().nest_service("/api", api).merge(screens)
}

async fn page(State(dir): State<Arc<Path>>) -> Response {
    answer(&dir, Path::new(PAGE))
}

async fn file(State(dir): State<Arc<Path>>, UrlPath(path): UrlPath<String>) -> Response {
    let relative = Path::new(&path);
    let inside = relative
        .components()
        .all(|part| matches!(part, Component::Normal(_)));
    if !inside {
        return StatusCode::NOT_FOUND.into_response();
    }
    if relative.extension().is_some() {
        return answer(&dir, relative);
    }
    answer(&dir, Path::new(PAGE))
}

fn answer(dir: &Path, relative: &Path) -> Response {
    match std::fs::read(dir.join(relative)) {
        Ok(bytes) => (
            [
                (header::CONTENT_TYPE, content_type(relative)),
                (header::CACHE_CONTROL, cache_control(relative)),
            ],
            bytes,
        )
            .into_response(),
        Err(_) if relative == Path::new(PAGE) => (
            StatusCode::NOT_FOUND,
            "the screens are not installed; run lys identity install with --surface",
        )
            .into_response(),
        Err(_) => StatusCode::NOT_FOUND.into_response(),
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
