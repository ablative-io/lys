//! Read back the placed build without changing its stored shape.

use super::super::error::{ErrorKind, IdentityResult};
use super::super::install::layout::Layout;
use super::super::install::{self, login, surface};
use super::super::private_files;
use super::{adopt, io, refuse};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

/// The build an install is running, as `install/build.json` records it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BuildRecord {
    /// Downloadable client releases, also kept by the configuration rollback.
    pub clients: BTreeMap<String, install::layout::ClientRelease>,
    /// Each binary in `bin/` by name, with the commit its `--version` names
    /// or [`adopt::UNSTAMPED`].
    pub binaries: BTreeMap<String, String>,
    /// The placed screens, when there are any.
    pub surface: Option<SurfaceBuild>,
    /// The environment the services were started with: the names kept
    /// from the login and the login shell's `PATH`.
    pub environment: login::EnvironmentRecord,
    /// The settings the service configuration runs with (ACCESS-002 R2).
    pub settings: install::settings::RecordedSettings,
    /// This install's runner key and its recorded join, when it has joined.
    pub runner_machine: install::layout::runner_machine::Recorded,
}

/// The placed screens package.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SurfaceBuild {
    /// The commit the manifest names.
    pub commit: String,
    /// The SHA-256 of the placed manifest, lowercase hex.
    pub manifest_sha256: String,
}

/// Reads what `bin/` and the placed screens are, writes it to
/// `install/build.json` and says it.
///
/// # Errors
/// Returns the named failure if build evidence cannot be read or recorded.
pub fn record_build(
    layout: &Layout,
    names: &[&str],
    say: &mut dyn FnMut(&str),
) -> IdentityResult<BuildRecord> {
    let mut binaries = BTreeMap::new();
    for name in names {
        let commit = adopt::commit_or_unstamped(&layout.binary(name), name)?;
        say(&format!("build {name} {commit}"));
        binaries.insert((*name).to_string(), commit);
    }
    let manifest_path = layout.surface_dir().join(surface::MANIFEST);
    let surface = match std::fs::read(&manifest_path) {
        Ok(bytes) => {
            let manifest: surface::Manifest = serde_json::from_slice(&bytes).map_err(|error| {
                refuse(
                    ErrorKind::ConfigInvalid,
                    "read screens",
                    "surface",
                    error.to_string(),
                )
                .at(&manifest_path)
            })?;
            let digest = format!("{:x}", Sha256::digest(&bytes));
            say(&format!(
                "build screens {} manifest sha256 {digest}",
                manifest.commit
            ));
            Some(SurfaceBuild {
                commit: manifest.commit,
                manifest_sha256: digest,
            })
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(io("read screens", &manifest_path, &error)),
    };
    let environment = login::login()?.record();
    say(&format!(
        "build environment keeps {} and PATH={}",
        environment.kept.join(","),
        environment.path
    ));
    let settings = install::settings::recorded(layout)?;
    let record = BuildRecord {
        clients: install::layout::client_releases(layout)?,
        binaries,
        surface,
        environment,
        settings,
        runner_machine: install::layout::runner_machine::recorded(layout, say)?,
    };
    let text = serde_json::to_vec_pretty(&record).map_err(|error| {
        refuse(
            ErrorKind::RenderFailed,
            "render",
            "build.json",
            error.to_string(),
        )
    })?;
    private_files::ensure_dir(&layout.install_dir())?;
    install::layout::runner_machine::write_build_record(layout, &text, say)?;
    Ok(record)
}
