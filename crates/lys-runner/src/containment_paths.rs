//! Writable roots are held directory objects, opened without following links.
//! Every ancestor is opened relative to an already-held directory. Launch
//! rechecks the named object's identity; the backend consumes its descriptor.

use std::os::fd::{AsFd, BorrowedFd, OwnedFd};
use std::os::unix::fs::MetadataExt;
use std::path::{Component, Path, PathBuf};

use rustix::fs::{Mode, OFlags, open, openat};
use serde::Serialize;

use crate::error::RunnerError;

/// Native identity of an opened filesystem object, not a pathname prefix.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Identity {
    /// Filesystem device number.
    pub device: u64,
    /// Inode number on that device.
    pub inode: u64,
}

/// A directory kept open from preparation until the native rules are applied.
/// Its descriptor is close-on-exec, never an agent's inherited write channel.
#[derive(Debug)]
pub struct Directory {
    path: PathBuf,
    file: std::fs::File,
    identity: Identity,
}

fn refused(path: &Path, reason: impl std::fmt::Display) -> RunnerError {
    RunnerError::refused(
        "containment_directory_unavailable",
        format!("{}: {reason}", path.display()),
    )
}

fn identity(file: &std::fs::File, path: &Path) -> Result<Identity, RunnerError> {
    let metadata = file.metadata().map_err(|error| refused(path, error))?;
    Ok(Identity {
        device: metadata.dev(),
        inode: metadata.ino(),
    })
}

fn open_directory(path: &Path) -> Result<OwnedFd, RunnerError> {
    let text = path
        .to_str()
        .ok_or_else(|| refused(path, "path is not UTF-8"))?;
    if !path.is_absolute()
        || text.contains('\0')
        || text.contains("//")
        || text.contains("/./")
        || text.ends_with("/.")
        || (text != "/" && text.ends_with('/'))
    {
        return Err(refused(path, "path is not absolute and plain"));
    }
    let flags = OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC;
    let mut held = open("/", flags, Mode::empty()).map_err(|error| refused(path, error))?;
    for component in path.components() {
        match component {
            Component::RootDir => {}
            Component::Normal(name) => {
                held = openat(&held, name, flags, Mode::empty())
                    .map_err(|error| refused(path, error))?;
            }
            Component::CurDir | Component::ParentDir | Component::Prefix(_) => {
                return Err(refused(path, "path includes a relative component"));
            }
        }
    }
    Ok(held)
}

impl Directory {
    /// Open the exact directory without following a symlink in any component.
    pub fn open(path: &Path) -> Result<Self, RunnerError> {
        let file = std::fs::File::from(open_directory(path)?);
        let identity = identity(&file, path)?;
        Ok(Self {
            path: path.to_owned(),
            file,
            identity,
        })
    }

    /// The recorded device and inode of the held directory.
    pub fn identity(&self) -> Identity {
        self.identity
    }

    /// Verify the name still resolves without links to the same held object.
    /// A rename/replacement between preparation and launch is refused by name.
    pub fn verify(&self) -> Result<(), RunnerError> {
        let current = std::fs::File::from(open_directory(&self.path)?);
        if identity(&current, &self.path)? != self.identity {
            return Err(refused(
                &self.path,
                "directory identity changed after preparation",
            ));
        }
        Ok(())
    }
}

impl AsFd for Directory {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.file.as_fd()
    }
}
