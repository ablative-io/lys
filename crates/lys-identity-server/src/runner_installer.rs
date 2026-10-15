//! A computer's owner downloads an installer carrying one fresh join.

use std::path::Path;

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path as RoutePath, State};
use axum::http::{HeaderMap, header};
use axum::response::{IntoResponse, Response};
use axum::{Json, Router, routing::post};
use lys_identity::OperationId;
use serde::Deserialize;

use crate::clients_api::{executable, read, refused, root};
use crate::error::ServerError;
use crate::network_api::{computer_owner, with_network};
use crate::network_join::{dial_address, with_joins};
use crate::routes::{Shared, signed_in};

pub(crate) use packages::Pending;

/// Downloading an installer changes its connection code and needs an explicit act.
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = RunnerInstallerBody)]
pub(crate) struct Body {}

#[derive(Clone, Copy)]
enum Kind {
    Macos,
    Deb,
    Arch,
}

impl Kind {
    fn read(text: &str) -> Result<Self, ServerError> {
        match text {
            "macos-pkg" => Ok(Self::Macos),
            "deb" => Ok(Self::Deb),
            "arch" => Ok(Self::Arch),
            _ => Err(refused(
                "installer_kind_unknown",
                "choose macos-pkg, deb or arch for this computer's installer",
            )),
        }
    }

    const fn extension(self) -> &'static str {
        match self {
            Self::Macos => "pkg",
            Self::Deb => "deb",
            Self::Arch => "pkg.tar.gz",
        }
    }

    const fn content_type(self) -> &'static str {
        match self {
            Self::Macos => "application/octet-stream",
            Self::Deb => "application/vnd.debian.binary-package",
            Self::Arch => "application/gzip",
        }
    }
}

struct Binaries {
    lys: Vec<u8>,
    containment: Vec<u8>,
}

fn stage_client(
    folder: &Path,
    name: &str,
    bytes: &[u8],
) -> Result<std::path::PathBuf, ServerError> {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;

    let path = folder.join(name);
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o700)
        .open(&path)
        .map_err(|error| refused("installer_build_failed", format!("staging {name}: {error}")))?;
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|error| refused("installer_build_failed", format!("staging {name}: {error}")))?;
    Ok(path)
}

enum Held {
    Macos { arm: Binaries, commit: String },
    Linux { binaries: Binaries, commit: String },
}

fn held(root: &Path, kind: Kind) -> Result<Held, ServerError> {
    let clients = read(root)?;
    let load = |platform| -> Result<(Binaries, String), ServerError> {
        let lys = executable(root, &clients, platform, "lys")?;
        let containment = executable(root, &clients, platform, "lys-containment-entry")?;
        Ok((
            Binaries {
                lys: lys.bytes,
                containment: containment.bytes,
            },
            lys.commit,
        ))
    };
    // Pass checked bytes to the package builder so a changed install cannot
    // replace an executable between its digest check and packaging.
    match kind {
        Kind::Macos => {
            let (arm, commit) = load("aarch64-apple-darwin")?;
            Ok(Held::Macos { arm, commit })
        }
        Kind::Deb | Kind::Arch => {
            let (binaries, commit) = load("x86_64-unknown-linux-gnu")?;
            Ok(Held::Linux { binaries, commit })
        }
    }
}

impl Held {
    /// The checked bytes are written once into a private staging folder the package builders read from; the folder
    /// is removed when this answer is made, whatever happened.
    fn package(&self, kind: Kind, pending: &Pending) -> Result<Vec<u8>, ServerError> {
        let staged = tempfile::Builder::new()
            .prefix("lys-installer-clients-")
            .tempdir()
            .map_err(|error| {
                refused(
                    "installer_build_failed",
                    format!("staging the clients: {error}"),
                )
            })?;
        let write = |name: &str, bytes: &[u8]| -> Result<std::path::PathBuf, ServerError> {
            stage_client(staged.path(), name, bytes)
        };
        match (kind, self) {
            (Kind::Macos, Self::Macos { arm, commit }) => {
                let (lys, entry) = (
                    write("lys", &arm.lys)?,
                    write("lys-containment-entry", &arm.containment)?,
                );
                packages::macos_pkg(pending, &lys, &entry, commit)
            }
            (Kind::Deb, Self::Linux { binaries, commit }) => {
                let (lys, entry) = (
                    write("lys", &binaries.lys)?,
                    write("lys-containment-entry", &binaries.containment)?,
                );
                packages::deb(pending, &lys, &entry, commit)
            }
            (Kind::Arch, Self::Linux { binaries, commit }) => {
                let (lys, entry) = (
                    write("lys", &binaries.lys)?,
                    write("lys-containment-entry", &binaries.containment)?,
                );
                packages::arch(pending, &lys, &entry, commit)
            }
            _ => Err(refused(
                "installer_kind_unknown",
                "the installer kind does not match its prepared clients",
            )),
        }
    }
}

fn disposition(name: &str, kind: Kind) -> String {
    let safe: String = name
        .chars()
        .map(|character| match character {
            '/' | '\\' | '"' => '_',
            character if character.is_control() => '_',
            character => character,
        })
        .collect();
    let filename = format!("Lys for {safe}.{}", kind.extension());
    let ascii: String = filename
        .chars()
        .map(|character| if character.is_ascii() { character } else { '_' })
        .collect();
    let mut encoded = String::new();
    for byte in filename.bytes() {
        if byte.is_ascii_alphanumeric() || b"!#$&+-.^_`|~".contains(&byte) {
            encoded.push(char::from(byte));
        } else {
            let hex = b"0123456789ABCDEF";
            encoded.push('%');
            encoded.push(char::from(hex[usize::from(byte >> 4)]));
            encoded.push(char::from(hex[usize::from(byte & 15)]));
        }
    }
    format!("attachment; filename=\"{ascii}\"; filename*=UTF-8''{encoded}")
}

async fn install(
    State(state): State<Shared>,
    headers: HeaderMap,
    RoutePath((id, kind)): RoutePath<(String, String)>,
    given: Result<Json<Body>, JsonRejection>,
) -> Result<Response, ServerError> {
    let actor = signed_in(&state, &headers)?;
    computer_owner(&state, &actor, &id)?;
    let Json(Body {}) = given.map_err(|rejection| ServerError::RequestMalformed {
        reason: rejection.body_text(),
    })?;
    let kind = Kind::read(&kind)?;
    let server = dial_address(state.oidc.redirect_url())?;
    let root = root(&state)?;
    let held = tokio::task::spawn_blocking(move || held(&root, kind))
        .await
        .map_err(|_| refused("installer_build_failed", "the client preparation stopped"))??;
    // Preparation can take time; a retired computer must not receive a new join.
    let by = computer_owner(&state, &actor, &id)?;
    let operation = OperationId::generate()?.to_string();
    let (name, pending) = with_network(&state, |store| {
        let machine = store.machine(&id).ok_or(ServerError::MachineUnknown)?;
        if machine.retired.is_some() {
            return Err(ServerError::MachineRetired);
        }
        // Retirement and issuance share the network lock; a retirement
        // cannot pass the final admission check while a code is being saved.
        let code = with_joins(&state, |joins| {
            joins.issue(&operation, &id, &by, crate::session::now())
        })?;
        Ok((
            machine.name.clone(),
            Pending {
                code,
                server,
                machine: id,
            },
        ))
    })?;
    let bytes = tokio::task::spawn_blocking(move || held.package(kind, &pending))
        .await
        .map_err(|_| {
            refused(
                "installer_build_failed",
                "the installer preparation stopped",
            )
        })??;
    Ok((
        [
            (header::CONTENT_TYPE, kind.content_type().to_owned()),
            (header::CONTENT_DISPOSITION, disposition(&name, kind)),
            (header::CACHE_CONTROL, "private, no-store".to_owned()),
            (header::X_CONTENT_TYPE_OPTIONS, "nosniff".to_owned()),
        ],
        bytes,
    )
        .into_response())
}

pub(crate) fn routes() -> Router<Shared> {
    Router::new().route("/network/machines/{id}/installer/{kind}", post(install))
}

mod packages;

#[cfg(test)]
#[path = "runner_installer_tests.rs"]
mod tests;
