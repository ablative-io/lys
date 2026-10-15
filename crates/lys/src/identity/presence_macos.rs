//! Execute only the authentication helper embedded by this build.

use std::io::Write;
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt};
use std::process::{Command, Stdio};

use super::{answer, refused};
use crate::identity::error::{ErrorKind, IdentityResult};

const HELPER: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/lys-owner-presence"));

pub(super) fn authenticate() -> IdentityResult<()> {
    let id = lys_identity::OperationId::generate()
        .map_err(|error| refused(ErrorKind::PresenceUnavailable, error.to_string()))?;
    let dir = std::env::temp_dir().join(format!("lys-presence-{id}"));
    std::fs::DirBuilder::new()
        .mode(0o700)
        .create(&dir)
        .map_err(|error| refused(ErrorKind::PresenceUnavailable, error.to_string()))?;
    let path = dir.join("owner-presence");
    let result = (|| {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&path)?;
        file.write_all(HELPER)?;
        file.sync_all()?;
        file.set_permissions(std::fs::Permissions::from_mode(0o500))?;
        Command::new(&path)
            .env_clear()
            .current_dir("/")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()
    })();
    let cleanup = std::fs::remove_file(&path)
        .or_else(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                Ok(())
            } else {
                Err(error)
            }
        })
        .and_then(|()| std::fs::remove_dir(&dir));
    cleanup.map_err(|error| {
        refused(
            ErrorKind::PresenceUnavailable,
            format!("the authentication helper could not be removed: {error}"),
        )
    })?;
    let output = result.map_err(|error| {
        refused(
            ErrorKind::PresenceUnavailable,
            format!("the authentication helper could not run: {error}"),
        )
    })?;
    answer(output.status.success(), &output.stdout)
}
