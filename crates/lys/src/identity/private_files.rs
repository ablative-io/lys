//! Restricted-mode durable file creation, and the reconciliation of its
//! outcome: a private file is written whole beside its place and renamed
//! into it, created owner-only, and read back so the caller learns whether
//! it was created, replaced, or already held these bytes.

use std::fs::{self, OpenOptions};
use std::io::Write as _;
use std::path::Path;

use super::error::{IdentityError, IdentityResult};

/// What a private write found and did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// The file did not exist and now holds the bytes.
    Created,
    /// The file held other bytes and now holds these.
    Replaced,
    /// The file already held exactly these bytes; nothing was written.
    Unchanged,
}

impl Outcome {
    /// The word the report prints.
    #[must_use]
    pub fn word(self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Replaced => "replaced",
            Self::Unchanged => "unchanged",
        }
    }
}

/// Create `dir` and every parent, owner-only, and tighten it if it exists.
pub fn ensure_private_dir(dir: &Path) -> IdentityResult<()> {
    fs::create_dir_all(dir).map_err(|source| IdentityError::Io {
        operation: "create the venue directory",
        path: dir.to_path_buf(),
        source,
    })?;
    set_mode(dir, 0o700, "restrict the venue directory")
}

/// Write `bytes` to `path` with mode `0600`, atomically, and say what changed.
pub fn write_private(path: &Path, bytes: &[u8]) -> IdentityResult<Outcome> {
    let existing = match fs::read(path) {
        Ok(held) => Some(held),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(source) => {
            return Err(IdentityError::Io {
                operation: "read the private file before writing it",
                path: path.to_path_buf(),
                source,
            });
        }
    };
    if existing.as_deref() == Some(bytes) {
        set_mode(path, 0o600, "restrict the private file")?;
        return Ok(Outcome::Unchanged);
    }
    if let Some(parent) = path.parent() {
        ensure_private_dir(parent)?;
    }
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let staging = path.with_file_name(format!(".{name}.writing"));
    {
        let mut options = OpenOptions::new();
        options.write(true).create(true).truncate(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt as _;
            options.mode(0o600);
        }
        let mut file = options.open(&staging).map_err(|source| IdentityError::Io {
            operation: "create the private file",
            path: staging.clone(),
            source,
        })?;
        file.write_all(bytes)
            .and_then(|()| file.sync_all())
            .map_err(|source| IdentityError::Io {
                operation: "write the private file",
                path: staging.clone(),
                source,
            })?;
    }
    set_mode(&staging, 0o600, "restrict the private file")?;
    fs::rename(&staging, path).map_err(|source| IdentityError::Io {
        operation: "move the private file into place",
        path: path.to_path_buf(),
        source,
    })?;
    let held = fs::read(path).map_err(|source| IdentityError::Io {
        operation: "read the private file back",
        path: path.to_path_buf(),
        source,
    })?;
    if held != bytes {
        return Err(IdentityError::Io {
            operation: "read the private file back",
            path: path.to_path_buf(),
            source: std::io::Error::other("the bytes read back differ from the bytes written"),
        });
    }
    Ok(if existing.is_some() {
        Outcome::Replaced
    } else {
        Outcome::Created
    })
}

/// The mode bits of `path`, for the report and the tests.
pub fn mode_of(path: &Path) -> IdentityResult<u32> {
    let meta = fs::metadata(path).map_err(|source| IdentityError::Io {
        operation: "read the private file's mode",
        path: path.to_path_buf(),
        source,
    })?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        Ok(meta.permissions().mode() & 0o777)
    }
    #[cfg(not(unix))]
    {
        let _ = meta;
        Ok(0)
    }
}

fn set_mode(path: &Path, mode: u32, operation: &'static str) -> IdentityResult<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        fs::set_permissions(path, fs::Permissions::from_mode(mode)).map_err(|source| {
            IdentityError::Io {
                operation,
                path: path.to_path_buf(),
                source,
            }
        })
    }
    #[cfg(not(unix))]
    {
        let _ = (path, mode, operation);
        Ok(())
    }
}
