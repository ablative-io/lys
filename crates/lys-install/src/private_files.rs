//! Restricted-mode durable files and the reconciliation of an interrupted
//! write.
//!
//! Every private file is written whole to a sibling temporary file created
//! owner-only, synced, renamed over its final name and followed by a sync of
//! the directory, so a reader sees the old bytes or the new ones and never a
//! torn file. A temporary file an interrupted run left behind is removed
//! before the next write; an existing file whose bytes already match is left
//! untouched and reported unchanged. A private file or directory that others
//! may read is refused by name rather than silently tightened.

use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use zeroize::Zeroizing;

use super::error::{ErrorKind, IdentityError, IdentityResult};

/// The mode every private file is created with.
pub const FILE_MODE: u32 = 0o600;

/// The mode the private state directory is created with.
pub const DIR_MODE: u32 = 0o700;

/// What a write did to the file on disk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// The file did not exist and now holds the bytes.
    Created,
    /// The file already held exactly these bytes and was not rewritten.
    Unchanged,
    /// The file held other bytes and now holds these.
    Replaced,
}

impl Outcome {
    /// The word printed for this outcome.
    pub fn word(self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Unchanged => "unchanged",
            Self::Replaced => "replaced",
        }
    }
}

fn io_error(operation: &'static str, path: &Path, source: &std::io::Error) -> IdentityError {
    IdentityError::new(
        ErrorKind::PrivateFileIo,
        operation,
        "private file",
        source.to_string(),
    )
    .at(path)
}

/// The permission bits of `path`, or `None` when it does not exist.
pub fn mode_of(path: &Path) -> IdentityResult<Option<u32>> {
    match fs::metadata(path) {
        Ok(metadata) => Ok(Some(permission_bits(&metadata))),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(io_error("stat", path, &error)),
    }
}

#[cfg(unix)]
fn permission_bits(metadata: &fs::Metadata) -> u32 {
    use std::os::unix::fs::PermissionsExt;
    metadata.permissions().mode() & 0o777
}

/// Without Unix modes, a writable file counts as readable by everyone, so a
/// private file is refused rather than trusted.
#[cfg(not(unix))]
fn permission_bits(metadata: &fs::Metadata) -> u32 {
    if metadata.permissions().readonly() {
        0o444
    } else {
        0o666
    }
}

fn refuse_open_mode(path: &Path, mode: u32) -> IdentityResult<()> {
    if mode.trailing_zeros() >= 6 {
        return Ok(());
    }
    Err(IdentityError::new(
        ErrorKind::PrivateFileModeOpen,
        "check mode",
        "private file",
        format!("mode {mode:03o} lets others read it; restrict it to the owner (chmod go-rwx)"),
    )
    .at(path))
}

/// Creates the private state directory owner-only, or checks that an
/// existing one is.
pub fn ensure_dir(path: &Path) -> IdentityResult<()> {
    if let Some(mode) = mode_of(path)? {
        return refuse_open_mode(path, mode);
    }
    create_dir(path)
}

#[cfg(unix)]
fn create_dir(path: &Path) -> IdentityResult<()> {
    use std::os::unix::fs::DirBuilderExt;
    fs::DirBuilder::new()
        .recursive(true)
        .mode(DIR_MODE)
        .create(path)
        .map_err(|error| io_error("create directory", path, &error))
}

#[cfg(not(unix))]
fn create_dir(path: &Path) -> IdentityResult<()> {
    Err(IdentityError::new(
        ErrorKind::PrivateFileIo,
        "create directory",
        "private state",
        "restricted file modes need a Unix host",
    )
    .at(path))
}

/// Reads a private file, refusing one others may read. `None` when it does
/// not exist.
pub fn read(path: &Path) -> IdentityResult<Option<Zeroizing<Vec<u8>>>> {
    let Some(mode) = mode_of(path)? else {
        return Ok(None);
    };
    refuse_open_mode(path, mode)?;
    let mut bytes = Zeroizing::new(Vec::new());
    File::open(path)
        .and_then(|mut file| file.read_to_end(&mut bytes))
        .map_err(|error| io_error("read", path, &error))?;
    Ok(Some(bytes))
}

fn temporary_path(path: &Path) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(".partial");
    path.with_file_name(name)
}

/// Writes `bytes` to `path` owner-only and durably, reconciling whatever an
/// earlier interrupted write left.
pub fn write(path: &Path, bytes: &[u8]) -> IdentityResult<Outcome> {
    let partial = temporary_path(path);
    match fs::remove_file(&partial) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(io_error("remove partial", &partial, &error)),
    }
    let existing = read(path)?;
    if let Some(current) = &existing
        && current.as_slice() == bytes
    {
        return Ok(Outcome::Unchanged);
    }
    let mut file = create_owner_only(&partial)?;
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|error| io_error("write", &partial, &error))?;
    drop(file);
    fs::rename(&partial, path).map_err(|error| io_error("rename", path, &error))?;
    if let Some(parent) = path.parent() {
        File::open(parent)
            .and_then(|directory| directory.sync_all())
            .map_err(|error| io_error("sync directory", parent, &error))?;
    }
    Ok(if existing.is_some() {
        Outcome::Replaced
    } else {
        Outcome::Created
    })
}

#[cfg(unix)]
fn create_owner_only(path: &Path) -> IdentityResult<File> {
    use std::os::unix::fs::OpenOptionsExt;
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(FILE_MODE)
        .open(path)
        .map_err(|error| io_error("create", path, &error))
}

#[cfg(not(unix))]
fn create_owner_only(path: &Path) -> IdentityResult<File> {
    Err(IdentityError::new(
        ErrorKind::PrivateFileIo,
        "create",
        "private file",
        "restricted file modes need a Unix host",
    )
    .at(path))
}
