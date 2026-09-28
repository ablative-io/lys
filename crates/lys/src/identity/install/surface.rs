//! The compiled screens package: verified against its manifest, then placed
//! under the data root for the directory service to serve.
//!
//! A package is a directory holding `surface-manifest.json`, which names
//! every file with its size and SHA-256. A file that is missing, differs,
//! or points outside the package is refused by name; nothing is copied
//! until every file has been checked.

use std::path::{Component, Path, PathBuf};

use serde::Deserialize;
use sha2::{Digest, Sha256};

use super::super::error::{ErrorKind, IdentityError, IdentityResult};

/// The manifest's file name inside a package.
pub const MANIFEST: &str = "surface-manifest.json";

/// The format this install reads.
pub const FORMAT: &str = "lys-identity-surface/v1";

/// The manifest as the package step writes it.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    /// The package format.
    pub format: String,
    /// The source commit the screens were built from.
    pub commit: String,
    /// Every file in the package.
    pub files: Vec<ManifestFile>,
}

/// One packaged file.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManifestFile {
    /// The path inside the package, with forward slashes.
    pub path: String,
    /// The file's length.
    pub bytes: u64,
    /// The file's SHA-256, lowercase hex.
    pub sha256: String,
}

/// Every packaged file by its path inside the package, with its verified bytes.
pub type Verified = Vec<(PathBuf, Vec<u8>)>;

fn refuse(kind: ErrorKind, detail: impl Into<String>) -> IdentityError {
    IdentityError::new(kind, "verify surface package", "surface", detail)
}

/// The relative path `entry` names, refused when it leaves the package.
fn relative(entry: &str) -> IdentityResult<PathBuf> {
    let path = Path::new(entry);
    let plain = path
        .components()
        .all(|part| matches!(part, Component::Normal(_)));
    if entry.is_empty() || !plain {
        return Err(refuse(
            ErrorKind::ConfigInvalid,
            format!("{entry}: a package path names files inside the package only"),
        ));
    }
    Ok(path.to_path_buf())
}

/// Reads and checks the manifest at `package`, returning every file with
/// its verified bytes.
pub fn verify(package: &Path) -> IdentityResult<(Manifest, Verified)> {
    let manifest_path = package.join(MANIFEST);
    let text = std::fs::read_to_string(&manifest_path).map_err(|error| {
        refuse(ErrorKind::ConfigUnreadable, error.to_string()).at(&manifest_path)
    })?;
    let manifest: Manifest = serde_json::from_str(&text)
        .map_err(|error| refuse(ErrorKind::ConfigInvalid, error.to_string()).at(&manifest_path))?;
    if manifest.format != FORMAT {
        return Err(refuse(
            ErrorKind::ConfigInvalid,
            format!("format {} is not {FORMAT}", manifest.format),
        ));
    }
    if !manifest.files.iter().any(|file| file.path == "index.html") {
        return Err(refuse(
            ErrorKind::ConfigInvalid,
            "no index.html in the manifest",
        ));
    }
    let mut files = Vec::with_capacity(manifest.files.len());
    for entry in &manifest.files {
        let relative = relative(&entry.path)?;
        let path = package.join(&relative);
        let bytes = std::fs::read(&path)
            .map_err(|error| refuse(ErrorKind::ConfigUnreadable, error.to_string()).at(&path))?;
        let digest = format!("{:x}", Sha256::digest(&bytes));
        if bytes.len() as u64 != entry.bytes || digest != entry.sha256 {
            return Err(refuse(
                ErrorKind::ReadBackMismatch,
                format!("{}: the file differs from its manifest entry", entry.path),
            ));
        }
        files.push((relative, bytes));
    }
    Ok((manifest, files))
}

/// Verifies the package at `package` and places it at `destination`,
/// replacing whatever was there.
pub fn place(package: &Path, destination: &Path) -> IdentityResult<Manifest> {
    let (manifest, files) = verify(package)?;
    let fresh = destination.with_extension("placing");
    if fresh.exists() {
        std::fs::remove_dir_all(&fresh).map_err(|error| io(&fresh, &error))?;
    }
    for (relative, bytes) in &files {
        let target = fresh.join(relative);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).map_err(|error| io(parent, &error))?;
        }
        std::fs::write(&target, bytes).map_err(|error| io(&target, &error))?;
    }
    let manifest_text =
        std::fs::read(package.join(MANIFEST)).map_err(|error| io(package, &error))?;
    std::fs::write(fresh.join(MANIFEST), manifest_text).map_err(|error| io(&fresh, &error))?;
    if destination.exists() {
        std::fs::remove_dir_all(destination).map_err(|error| io(destination, &error))?;
    }
    std::fs::rename(&fresh, destination).map_err(|error| io(destination, &error))?;
    Ok(manifest)
}

fn io(path: &Path, error: &std::io::Error) -> IdentityError {
    IdentityError::new(
        ErrorKind::PrivateFileIo,
        "place surface package",
        "surface",
        error.to_string(),
    )
    .at(path)
}
