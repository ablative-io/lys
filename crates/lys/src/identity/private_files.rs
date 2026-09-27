//! Restricted-mode durable file creation, and reconciliation of its outcome.
//!
//! Invariants:
//!
//! - A private file is created mode `0600` inside a directory of mode `0700`
//!   and is never observable with looser permissions: content is written to a
//!   fresh `0600` temporary file beside the target, synced, then renamed over
//!   it, and the directory is synced so the rename is durable on return.
//! - A write reports what actually happened ([`WriteOutcome`]): an existing
//!   file with identical content is left alone and reported `Unchanged`, so a
//!   repeated run is observably idempotent.
//! - [`create_once`] never replaces an existing file. A credential that
//!   already initialised a database is reused, not rotated; if another writer
//!   created the file first, its content wins and is read back.
//! - A private file found readable or writable by group or other is refused
//!   by name rather than silently used.

use std::fs::{File, OpenOptions};
use std::io::{ErrorKind, Read, Write};
use std::path::{Path, PathBuf};

use zeroize::Zeroizing;

use crate::identity::error::{IdentityError, IdentityResult};

/// Mode of every private file.
pub const PRIVATE_FILE_MODE: u32 = 0o600;

/// Mode of every private directory.
pub const PRIVATE_DIR_MODE: u32 = 0o700;

/// What a private write did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteOutcome {
    /// The file did not exist and was created.
    Created,
    /// The file existed with the same content and was left as it was.
    Unchanged,
    /// The file existed with other content and was replaced.
    Replaced,
    /// The file existed and was kept; nothing was written.
    Reused,
}

impl WriteOutcome {
    /// Stable lower-case name for output.
    pub fn as_str(self) -> &'static str {
        match self {
            WriteOutcome::Created => "created",
            WriteOutcome::Unchanged => "unchanged",
            WriteOutcome::Replaced => "replaced",
            WriteOutcome::Reused => "reused",
        }
    }
}

/// Create `dir` (and its parents) and restrict it to mode `0700`.
pub fn ensure_private_dir(dir: &Path, resource: &str) -> IdentityResult<()> {
    std::fs::create_dir_all(dir).map_err(|source| io("create directory", resource, dir, source))?;
    set_mode(dir, PRIVATE_DIR_MODE, resource)
}

/// Write `contents` to `path` as a private file, replacing any other content.
pub fn write_private(path: &Path, contents: &[u8], resource: &str) -> IdentityResult<WriteOutcome> {
    let existed = match read_private(path, resource) {
        Ok(existing) if existing.as_slice() == contents => return Ok(WriteOutcome::Unchanged),
        Ok(_) => true,
        Err(IdentityError::SecretMissing { .. }) => false,
        Err(error) => return Err(error),
    };
    let temporary = temporary_path(path);
    let written = write_new(&temporary, contents, resource).and_then(|()| {
        std::fs::rename(&temporary, path).map_err(|source| io("rename", resource, path, source))
    });
    if let Err(error) = written {
        // The temporary file holds the same private bytes; leave none behind.
        // A failure to remove it is reported beside the original failure.
        match std::fs::remove_file(&temporary) {
            Err(cleanup) if cleanup.kind() != ErrorKind::NotFound => eprintln!(
                "warning: io_failed: remove temporary {resource} at {}: {cleanup}",
                temporary.display()
            ),
            _ => {}
        }
        return Err(error);
    }
    sync_parent(path, resource)?;
    Ok(if existed {
        WriteOutcome::Replaced
    } else {
        WriteOutcome::Created
    })
}

/// Create `path` with `contents` only if it does not exist.
///
/// Returns the bytes the file holds afterwards and what happened: `Created`
/// when this call wrote it, `Reused` when it already existed (including when
/// another writer won a race to create it).
pub fn create_once(
    path: &Path,
    contents: &[u8],
    resource: &str,
) -> IdentityResult<(Zeroizing<Vec<u8>>, WriteOutcome)> {
    match write_new(path, contents, resource) {
        Ok(()) => {
            sync_parent(path, resource)?;
            Ok((Zeroizing::new(contents.to_vec()), WriteOutcome::Created))
        }
        Err(IdentityError::Io { source, .. }) if source.kind() == ErrorKind::AlreadyExists => {
            Ok((read_private(path, resource)?, WriteOutcome::Reused))
        }
        Err(error) => Err(error),
    }
}

/// Read a private file, refusing one that grants group or other access.
///
/// A missing file is [`IdentityError::SecretMissing`], naming `resource`.
pub fn read_private(path: &Path, resource: &str) -> IdentityResult<Zeroizing<Vec<u8>>> {
    let mut file = match File::open(path) {
        Ok(file) => file,
        Err(source) if source.kind() == ErrorKind::NotFound => {
            return Err(IdentityError::SecretMissing {
                resource: resource.to_string(),
                path: path.to_path_buf(),
            });
        }
        Err(source) => return Err(io("open", resource, path, source)),
    };
    let mode = mode_of(path, resource)?;
    if mode & 0o077 != 0 {
        return Err(IdentityError::PrivateFileTooOpen {
            resource: resource.to_string(),
            path: path.to_path_buf(),
            mode,
        });
    }
    let mut contents = Zeroizing::new(Vec::new());
    file.read_to_end(&mut contents)
        .map_err(|source| io("read", resource, path, source))?;
    Ok(contents)
}

/// The permission bits of `path` (always `0o600` on non-Unix platforms,
/// which have no such bits to read).
pub fn mode_of(path: &Path, resource: &str) -> IdentityResult<u32> {
    let metadata =
        std::fs::metadata(path).map_err(|source| io("read metadata of", resource, path, source))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        Ok(metadata.permissions().mode() & 0o777)
    }
    #[cfg(not(unix))]
    {
        let _ = metadata;
        Ok(PRIVATE_FILE_MODE)
    }
}

fn write_new(path: &Path, contents: &[u8], resource: &str) -> IdentityResult<()> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(PRIVATE_FILE_MODE);
    }
    let mut file = options
        .open(path)
        .map_err(|source| io("create", resource, path, source))?;
    file.write_all(contents)
        .map_err(|source| io("write", resource, path, source))?;
    file.sync_all()
        .map_err(|source| io("sync", resource, path, source))?;
    // The creation mode is filtered by the umask; set it exactly.
    set_mode(path, PRIVATE_FILE_MODE, resource)
}

fn set_mode(path: &Path, mode: u32, resource: &str) -> IdentityResult<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))
            .map_err(|source| io("restrict permissions of", resource, path, source))
    }
    #[cfg(not(unix))]
    {
        let _ = (path, mode, resource);
        Ok(())
    }
}

fn sync_parent(path: &Path, resource: &str) -> IdentityResult<()> {
    #[cfg(unix)]
    {
        let parent = match path.parent() {
            Some(parent) if !parent.as_os_str().is_empty() => parent,
            _ => Path::new("."),
        };
        File::open(parent)
            .and_then(|directory| directory.sync_all())
            .map_err(|source| io("sync directory of", resource, parent, source))
    }
    #[cfg(not(unix))]
    {
        let _ = (path, resource);
        Ok(())
    }
}

fn temporary_path(path: &Path) -> PathBuf {
    let mut name = path.file_name().map(std::ffi::OsStr::to_os_string).unwrap_or_default();
    name.push(format!(".tmp-{}", std::process::id()));
    path.with_file_name(name)
}

fn io(operation: &'static str, resource: &str, path: &Path, source: std::io::Error) -> IdentityError {
    IdentityError::Io {
        operation,
        resource: resource.to_string(),
        path: path.to_path_buf(),
        source,
    }
}
