//! Durable writes and key file custody checks.

use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use crate::error::SecretsError;

pub(crate) fn io(context: impl Into<String>) -> impl FnOnce(std::io::Error) -> SecretsError {
    let context = context.into();
    move |source| SecretsError::Io { context, source }
}

/// Writes `bytes` to `path` so that a reader sees either the old file or the
/// whole new one: a sibling temporary file is written and synced, renamed
/// over `path`, and the directory is synced.
pub(crate) fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), SecretsError> {
    let dir = path.parent().ok_or_else(|| SecretsError::Io {
        context: format!("{} has no parent directory", path.display()),
        source: std::io::Error::from(std::io::ErrorKind::InvalidInput),
    })?;
    let mut temporary = tempfile::NamedTempFile::new_in(dir).map_err(io(format!(
        "creating a temporary file in {}",
        dir.display()
    )))?;
    temporary.write_all(bytes).map_err(io(format!(
        "writing a temporary file for {}",
        path.display()
    )))?;
    temporary.as_file().sync_all().map_err(io(format!(
        "syncing a temporary file for {}",
        path.display()
    )))?;
    temporary.persist(path).map_err(|error| SecretsError::Io {
        context: format!("renaming into {}", path.display()),
        source: error.error,
    })?;
    fs::File::open(dir)
        .and_then(|handle| handle.sync_all())
        .map_err(io(format!("syncing directory {}", dir.display())))
}

/// `path` with every symbolic link resolved. A path that does not exist yet
/// resolves through its parent directory, which must exist.
pub(crate) fn canonical(path: &Path) -> Result<PathBuf, SecretsError> {
    if path.exists() {
        return fs::canonicalize(path).map_err(io(format!("resolving {}", path.display())));
    }
    let parent = match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    };
    let name = path.file_name().ok_or_else(|| SecretsError::Io {
        context: format!("{} names no file", path.display()),
        source: std::io::Error::from(std::io::ErrorKind::InvalidInput),
    })?;
    Ok(fs::canonicalize(parent)
        .map_err(io(format!("resolving {}", parent.display())))?
        .join(name))
}

/// Refuses a key file that lies inside any of `dirs`, compared on canonical
/// paths.
pub(crate) fn ensure_outside(key: &Path, dirs: &[&Path]) -> Result<(), SecretsError> {
    let key_canonical = canonical(key)?;
    for dir in dirs {
        let dir_canonical = canonical(dir)?;
        if key_canonical.starts_with(&dir_canonical) {
            return Err(SecretsError::KeyFileMisplaced {
                key: key_canonical,
                dir: dir_canonical,
            });
        }
    }
    Ok(())
}

/// Refuses a key file whose mode grants anything to its group or others.
pub(crate) fn ensure_owner_only(path: &Path) -> Result<(), SecretsError> {
    let mode = fs::metadata(path)
        .map_err(io(format!("reading the mode of {}", path.display())))?
        .permissions()
        .mode()
        & 0o777;
    if mode & 0o077 != 0 {
        return Err(SecretsError::KeyFilePermissions {
            path: path.to_path_buf(),
            mode,
        });
    }
    Ok(())
}
