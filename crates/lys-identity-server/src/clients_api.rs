//! Signed-in people download the same client release the install records.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use axum::extract::{Path as RoutePath, State};
use axum::http::{HeaderMap, header};
use axum::response::{IntoResponse, Response};
use axum::{Json, Router, routing::get};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::ServerError;
use crate::routes::{AppState, Shared, signed_in};

/// One platform's binaries, bound to the commit and to their file digests.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ClientRelease {
    /// The source commit the release was built from.
    pub commit: String,
    /// Each download's SHA-256, keyed by its executable name.
    pub files: BTreeMap<String, String>,
}

/// Every platform the current install holds.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct Clients {
    /// Releases keyed by Rust target triple.
    pub platforms: BTreeMap<String, ClientRelease>,
}

fn refused(name: &str, words: impl Into<String>) -> ServerError {
    ServerError::Runner {
        refusal: name.to_owned(),
        words: words.into(),
    }
}

fn segment(text: &str) -> bool {
    !text.is_empty()
        && text
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

fn root(state: &AppState) -> Result<PathBuf, ServerError> {
    let data = state.estate_plan_file.parent().ok_or_else(|| {
        refused(
            "clients_unreadable",
            "the estate plan has no containing folder",
        )
    })?;
    let root = if data.file_name().is_some_and(|name| name == "data") {
        data.parent().ok_or_else(|| {
            refused(
                "clients_unreadable",
                "the install's data folder has no root",
            )
        })?
    } else {
        data
    };
    Ok(root.to_path_buf())
}

fn read(root: &Path) -> Result<Clients, ServerError> {
    let path = root.join("install/build.json");
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Clients {
                platforms: BTreeMap::new(),
            });
        }
        Err(error) => {
            return Err(refused(
                "clients_unreadable",
                format!("{}: {error}", path.display()),
            ));
        }
    };
    let build: serde_json::Value = serde_json::from_slice(&bytes).map_err(|error| {
        refused(
            "clients_unreadable",
            format!("the build record does not read: {error}"),
        )
    })?;
    let Some(clients) = build.get("clients") else {
        return Ok(Clients {
            platforms: BTreeMap::new(),
        });
    };
    let platforms: BTreeMap<String, ClientRelease> = serde_json::from_value(clients.clone())
        .map_err(|error| {
            refused(
                "clients_unreadable",
                format!("the client releases do not read: {error}"),
            )
        })?;
    for (platform, release) in &platforms {
        if !segment(platform)
            || !segment(&release.commit)
            || release.files.len() != 2
            || !["lys", "lys-containment-entry"]
                .iter()
                .all(|name| release.files.contains_key(*name))
            || release.files.values().any(|digest| {
                digest.len() != 64
                    || !digest
                        .bytes()
                        .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
            })
        {
            return Err(refused(
                "clients_unreadable",
                "a client release has an invalid platform, commit, executable set or digest",
            ));
        }
    }
    Ok(Clients { platforms })
}

/// Downloads require the same signed-in authority as the directory's reads.
pub(crate) fn routes() -> Router<Shared> {
    Router::new()
        .route("/clients", get(list))
        .route("/clients/{platform}/{name}", get(download))
}

async fn list(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Clients>, ServerError> {
    signed_in(&state, &headers)?;
    Ok(Json(read(&root(&state)?)?))
}

async fn download(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    RoutePath((platform, name)): RoutePath<(String, String)>,
) -> Result<Response, ServerError> {
    signed_in(&state, &headers)?;
    let root = root(&state)?;
    let clients = read(&root)?;
    let release = clients.platforms.get(&platform).ok_or_else(|| {
        refused(
            "client_platform_unknown",
            format!("Lys holds no client for platform {platform}"),
        )
    })?;
    let expected = release.files.get(&name).ok_or_else(|| {
        refused(
            "client_file_unknown",
            "this platform holds lys and lys-containment-entry only",
        )
    })?;
    let path = root
        .join("clients")
        .join(&platform)
        .join(&release.commit)
        .join(&name);
    let bytes = std::fs::read(&path)
        .map_err(|error| refused("clients_unreadable", format!("{}: {error}", path.display())))?;
    let digest = format!("{:x}", Sha256::digest(&bytes));
    if digest != *expected {
        return Err(refused(
            "client_digest_mismatch",
            "the held executable differs from the install's recorded SHA-256",
        ));
    }
    Ok((
        [
            (
                header::CONTENT_TYPE.as_str(),
                "application/octet-stream".to_owned(),
            ),
            (
                header::CONTENT_DISPOSITION.as_str(),
                format!("attachment; filename=\"{name}\""),
            ),
            (
                header::CACHE_CONTROL.as_str(),
                "private, no-store".to_owned(),
            ),
            ("x-lys-sha256", digest),
            ("x-lys-commit", release.commit.clone()),
        ],
        bytes,
    )
        .into_response())
}

#[cfg(test)]
#[path = "clients_api_tests.rs"]
mod tests;

/// Describe the installer-owned byte response without claiming it is JSON.
pub(crate) fn describe_download(document: &mut serde_json::Value) {
    document["paths"]["/clients/{platform}/{name}"]["get"]["responses"]["200"]["content"] = serde_json::json!({
        "application/octet-stream": {"schema": {"type":"string", "format":"binary"}}
    });
}
