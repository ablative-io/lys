//! Restricted-mode durable file creation, and reconciliation of its outcome.
//!
//! # Invariants
//!
//! - A private directory is `0700` and a private file `0600`, set at
//!   creation and checked afterwards. A file or directory found with any
//!   group or other permission is refused (`private_mode_unrestricted`)
//!   rather than trusted or silently tightened: something other than this
//!   CLI put it there.
//! - A write is durable on return: the bytes go to a temporary sibling that
//!   is fsynced, renamed over the target, and the directory is fsynced.
//! - A write's outcome is reconciled by reading it back. What is on disk is
//!   the outcome; a read-back that differs is `private_write_unconfirmed`,
//!   never assumed to have worked.
//! - A temporary sibling left by an interrupted write is truncated and
//!   rewritten, never read as a value.
//! - Off Unix there are no file modes to restrict with, so every operation
//!   here fails as `io_failed` with the `Unsupported` kind instead of
//!   writing a credential readable by other users.

use std::path::Path;

use zeroize::Zeroizing;

use super::error::IdentityError;

/// Permission bits of a private directory.
pub const DIR_MODE: u32 = 0o700;

/// Permission bits of a private file.
pub const FILE_MODE: u32 = 0o600;

/// The permission bits that must be clear on anything private.
const GROUP_OTHER: u32 = 0o077;

/// An error builder for one operation on one path.
fn io(operation: &'static str, path: &Path) -> impl FnOnce(std::io::Error) -> IdentityError {
    let path = path.to_path_buf();
    move |source| IdentityError::Io {
        operation,
        path,
        source,
    }
}

/// Creates `path` (and any missing parents) as `0700`, or checks an
/// existing directory is no more open than that.
pub fn ensure_private_dir(path: &Path) -> Result<(), IdentityError> {
    create_dir(path)?;
    refuse_open(path, DIR_MODE)
}

/// Writes `contents` to `path` as a `0600` file, durably, and confirms the
/// outcome by reading it back.
pub fn write_private(path: &Path, contents: &[u8]) -> Result<(), IdentityError> {
    use std::io::Write;

    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let temp = dir.join(format!(".{name}.tmp"));
    let mut file = open_restricted(&temp)?;
    file.write_all(contents)
        .map_err(io("write private file", &temp))?;
    file.sync_all().map_err(io("sync private file", &temp))?;
    drop(file);
    std::fs::rename(&temp, path).map_err(io("install private file", path))?;
    std::fs::File::open(dir)
        .and_then(|handle| handle.sync_all())
        .map_err(io("sync private directory", dir))?;

    let written = read_private(path)?.ok_or_else(|| IdentityError::PrivateWriteUnconfirmed {
        path: path.to_path_buf(),
    })?;
    if written.as_slice() != contents {
        return Err(IdentityError::PrivateWriteUnconfirmed {
            path: path.to_path_buf(),
        });
    }
    let mode = mode_of(path)?;
    if mode != FILE_MODE {
        return Err(IdentityError::PrivateModeUnrestricted {
            path: path.to_path_buf(),
            mode,
            expected: FILE_MODE,
        });
    }
    Ok(())
}

/// Creates a directory and its missing parents with [`DIR_MODE`].
#[cfg(unix)]
fn create_dir(path: &Path) -> Result<(), IdentityError> {
    use std::os::unix::fs::DirBuilderExt;

    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(DIR_MODE)
        .create(path)
        .map_err(io("create private directory", path))
}

/// Opens `path` for writing as a truncated [`FILE_MODE`] file. A leftover
/// file keeps the mode it was created with, so the mode is set again.
#[cfg(unix)]
fn open_restricted(path: &Path) -> Result<std::fs::File, IdentityError> {
    use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

    let file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(FILE_MODE)
        .open(path)
        .map_err(io("create private file", path))?;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(FILE_MODE))
        .map_err(io("restrict private file", path))?;
    Ok(file)
}

/// The permission bits of `path`.
#[cfg(unix)]
pub fn mode_of(path: &Path) -> Result<u32, IdentityError> {
    use std::os::unix::fs::PermissionsExt;

    std::fs::metadata(path)
        .map(|metadata| metadata.permissions().mode() & 0o7777)
        .map_err(io("read mode of", path))
}

/// Refuses on this platform: there is no file mode to restrict with.
#[cfg(not(unix))]
fn create_dir(path: &Path) -> Result<(), IdentityError> {
    Err(unsupported("create private directory", path))
}

/// Refuses on this platform: there is no file mode to restrict with.
#[cfg(not(unix))]
fn open_restricted(path: &Path) -> Result<std::fs::File, IdentityError> {
    Err(unsupported("create private file", path))
}

/// Refuses on this platform: there is no file mode to read.
#[cfg(not(unix))]
pub fn mode_of(path: &Path) -> Result<u32, IdentityError> {
    Err(unsupported("read mode of", path))
}

#[cfg(not(unix))]
fn unsupported(operation: &'static str, path: &Path) -> IdentityError {
    io(operation, path)(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "restricted file modes need Unix",
    ))
}

/// Reads a private file, or `None` when it does not exist. A file that
/// grants any group or other permission is refused.
pub fn read_private(path: &Path) -> Result<Option<Zeroizing<Vec<u8>>>, IdentityError> {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => Zeroizing::new(bytes),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(source) => return Err(io("read private file", path)(source)),
    };
    refuse_open(path, FILE_MODE)?;
    Ok(Some(bytes))
}

/// Refuses `path` when its mode grants any group or other permission.
fn refuse_open(path: &Path, expected: u32) -> Result<(), IdentityError> {
    let mode = mode_of(path)?;
    if mode & GROUP_OTHER == 0 {
        Ok(())
    } else {
        Err(IdentityError::PrivateModeUnrestricted {
            path: path.to_path_buf(),
            mode,
            expected,
        })
    }
}
