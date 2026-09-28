//! Where this copy of Lys.app is, what it carries, and what is installed.
//!
//! The app's executable sits in `Contents/MacOS` beside every Lys binary it
//! carries, which is where the install looks for the binaries it places in
//! its own `bin/`; the screens package is in `Contents/Resources/surface`.
//! Once placed, the install runs from `bin/`, independent of where the app
//! sits.
//!
//! Invariants: an app on a read-only disk image, or one the system has
//! moved aside to a translocated path because it was opened where it was
//! downloaded, installs nothing and says to move Lys to Applications; the
//! build the app carries is read from its binary's own `--version`, and the
//! build installed from the install's own record, never guessed.

use std::path::{Path, PathBuf};

use lys_install::install::layout::Layout;
use lys_install::upgrade;

use crate::refusal::Refusal;

/// The directory service, whose build names the build of the whole install.
pub const SERVICE: &str = "lys-identity-server";

/// The app's own binary, which the install keeps in its `bin/` for the
/// login item and the uninstall helper.
pub const APP: &str = "lys-app";

/// This copy of the app.
#[derive(Debug, Clone)]
pub struct Bundle {
    /// The folder the app's executable and every binary it carries are in.
    pub binaries: PathBuf,
}

impl Bundle {
    /// The bundle the running executable is in.
    pub fn current() -> Result<Self, Refusal> {
        let own = std::env::current_exe().map_err(|error| {
            Refusal::app("app_path_unknown", "Lys could not find where it is.", error)
        })?;
        let binaries = own.parent().map(Path::to_path_buf).ok_or_else(|| {
            Refusal::app(
                "app_path_unknown",
                "Lys could not find where it is.",
                format!("{} has no folder", own.display()),
            )
        })?;
        Ok(Self { binaries })
    }

    /// The binary `name` the app carries.
    pub fn binary(&self, name: &str) -> PathBuf {
        self.binaries.join(name)
    }

    /// The screens package the app carries, when the app is a bundle.
    pub fn surface(&self) -> PathBuf {
        self.binaries.with_file_name("Resources").join("surface")
    }

    /// Refuses, saying to move Lys to Applications, when the app runs from
    /// somewhere nothing it installs could stay.
    pub fn require_placed(&self) -> Result<(), Refusal> {
        if is_translocated(&self.binaries) {
            return Err(Refusal::translocated(&self.binaries.display().to_string()));
        }
        Ok(())
    }

    /// The build this app carries: the commit its directory service's
    /// `--version` names.
    pub fn build(&self) -> Result<String, Refusal> {
        upgrade::version(&self.binary(SERVICE), SERVICE).map_err(|error| {
            Refusal::app(
                "app_build_unreadable",
                "Lys could not read which version this copy of Lys is.",
                error,
            )
        })
    }
}

/// Whether `path` is where the system runs an app it has moved aside
/// (`/private/var/folders/…/AppTranslocation/…`) or a mounted disk image
/// (`/Volumes/…`).
pub fn is_translocated(path: &Path) -> bool {
    path.components()
        .any(|part| part.as_os_str() == "AppTranslocation")
        || path.starts_with("/Volumes")
}

/// What the data root holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Installed {
    /// No install, or one that stopped before its service was configured.
    Nothing,
    /// An install whose build record names `build`, or `None` when it has
    /// no record yet.
    Build(Option<String>),
}

/// What `layout` holds: an install once its deployment configuration, its
/// service's configuration and its service binary are all in place.
pub fn installed(layout: &Layout) -> Result<Installed, Refusal> {
    let placed = [
        layout.deployment_config(),
        layout.service_config(),
        layout.binary(SERVICE),
    ];
    if !placed.iter().all(|path| path.is_file()) {
        return Ok(Installed::Nothing);
    }
    let record = layout.build_record();
    let text = match std::fs::read_to_string(&record) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Installed::Build(None));
        }
        Err(error) => {
            return Err(Refusal::app(
                "build_record_unreadable",
                "Lys could not read which version is installed.",
                format!("{}: {error}", record.display()),
            ));
        }
    };
    let build = recorded_build(&text).map_err(|error| {
        Refusal::app(
            "build_record_unreadable",
            "Lys could not read which version is installed.",
            format!("{}: {error}", record.display()),
        )
    })?;
    Ok(Installed::Build(build))
}

/// The directory service's commit a build record names, `None` when it
/// names none.
pub fn recorded_build(text: &str) -> Result<Option<String>, serde_json::Error> {
    let record: serde_json::Value = serde_json::from_str(text)?;
    Ok(record
        .get("binaries")
        .and_then(|binaries| binaries.get(SERVICE))
        .and_then(serde_json::Value::as_str)
        .map(str::to_string))
}

#[cfg(test)]
#[path = "bundle_tests.rs"]
mod tests;
