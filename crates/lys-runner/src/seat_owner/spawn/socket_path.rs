//! Old endpoints are inspected before an owner can take a new socket.

use std::fs;
use std::io::{ErrorKind, Write};
use std::os::unix::fs::{FileTypeExt, OpenOptionsExt, PermissionsExt};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};

use super::{OwnerPlan, start_failed};
use crate::error::RunnerError;
use crate::seat_owner::protocol::{LOG, PLAN, SOCKET, fits_socket, owner_socket};

impl OwnerPlan {
    /// The short endpoint; an old plan's directory identifies its install root.
    #[must_use]
    pub fn socket_in(&self, dir: &Path) -> PathBuf {
        if let Some(socket) = &self.socket
            && socket.file_name().is_some_and(|name| name != SOCKET)
        {
            let name = owner_socket(Path::new(""), &self.binding.session);
            return socket.with_file_name(name.as_os_str());
        }
        let mut root = dir.to_path_buf();
        for _ in 0..4 {
            root.pop();
        }
        owner_socket(&root.join("run/owners"), &self.binding.session)
    }

    /// Reads a plan back from an owner's directory.
    ///
    /// # Errors
    ///
    /// `seat_owner_start_failed` when it cannot be read or does not validate.
    pub fn read(dir: &Path) -> Result<Self, RunnerError> {
        let path = dir.join(PLAN);
        let bytes = fs::read(&path)
            .map_err(|error| start_failed(format!("reading {}: {error}", path.display())))?;
        let plan: Self = serde_json::from_slice(&bytes)
            .map_err(|error| start_failed(format!("{} does not read: {error}", path.display())))?;
        plan.validate()?;
        prepare(&plan, dir)?;
        Ok(plan)
    }
}

fn legacy_failed(path: &Path, error: impl std::fmt::Display) -> RunnerError {
    RunnerError::refused(
        "seat_owner_legacy_unproved",
        format!(
            "the legacy socket {} could not be proved inactive: {error}",
            path.display()
        ),
    )
}

pub(super) fn prepare(plan: &OwnerPlan, dir: &Path) -> Result<(), RunnerError> {
    let old_plan = plan
        .socket
        .as_ref()
        .is_none_or(|path| path.file_name().is_some_and(|name| name == SOCKET));
    if old_plan
        && (!dir.ends_with(Path::new("data/runner/owners").join(&plan.binding.session))
            || plan
                .socket
                .as_ref()
                .is_some_and(|path| path != &dir.join(SOCKET)))
    {
        return Err(start_failed(format!(
            "{} is not an owner plan directory",
            dir.display()
        )));
    }
    if plan
        .socket
        .as_ref()
        .is_some_and(|path| !path.is_absolute() || path.file_name().is_none())
    {
        return Err(start_failed(
            "the owner socket must be an absolute file path",
        ));
    }
    let socket = plan.socket_in(dir);
    fits_socket(&socket)?;
    let legacy = dir.join(SOCKET);
    match fs::symlink_metadata(&legacy) {
        Err(error) if error.kind() == ErrorKind::NotFound => {}
        Err(error) => return Err(legacy_failed(&legacy, error)),
        Ok(metadata) => {
            if !metadata.file_type().is_socket() {
                return Err(legacy_failed(&legacy, "the path is not a socket file"));
            }
            match UnixStream::connect(&legacy) {
                Ok(_) => {
                    return Err(RunnerError::refused(
                        "seat_owner_legacy_listening",
                        format!("a legacy owner still listens on {}", legacy.display()),
                    ));
                }
                Err(error) if error.kind() == ErrorKind::ConnectionRefused => {}
                Err(error) => return Err(legacy_failed(&legacy, error)),
            }
            let mut log = fs::OpenOptions::new()
                .create(true)
                .append(true)
                .mode(0o600)
                .open(dir.join(LOG))
                .map_err(|error| legacy_failed(&legacy, error))?;
            fs::remove_file(&legacy).map_err(|error| legacy_failed(&legacy, error))?;
            let words = format!("seat_owner_legacy_removed: {}", legacy.display());
            writeln!(log, "{words}")
                .and_then(|()| log.sync_all())
                .map_err(|error| legacy_failed(&legacy, error))?;
        }
    }
    let parent = socket
        .parent()
        .ok_or_else(|| start_failed("the owner socket has no directory"))?;
    fs::create_dir_all(parent)
        .and_then(|()| fs::set_permissions(parent, fs::Permissions::from_mode(0o700)))
        .map_err(|error| start_failed(format!("making {}: {error}", parent.display())))?;
    Ok(())
}
