//! Joining preserves the computer's key when an older layout kept it elsewhere.

use std::fs;
use std::path::Path;

use lys_core::Ed25519Identity;

use super::Layout;
use crate::identity::error::{ErrorKind, IdentityError, IdentityResult};
use crate::identity::private_files;

#[path = "runner_machine_record.rs"]
mod record;
pub use record::{Recorded, recorded, write_build_record};

fn failure(operation: &'static str, path: &Path, reason: impl ToString) -> IdentityError {
    IdentityError::new(
        ErrorKind::PrivateFileIo,
        operation,
        "runner machine key",
        reason.to_string(),
    )
    .at(path)
}

fn present(path: &Path) -> IdentityResult<bool> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_file() => {
            drop(private_files::read(path)?);
            Ok(true)
        }
        Ok(_) => Err(failure(
            "check",
            path,
            "runner machine key must be a regular private file",
        )),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(failure("check", path, error)),
    }
}

/// Load or create this computer's key, migrating an older join without rotation.
///
/// # Errors
/// Names unreadable files, open permissions and conflicting identities before changing them.
pub fn provide(layout: &Layout, say: &mut dyn FnMut(&str)) -> IdentityResult<Ed25519Identity> {
    let path = layout.runner_machine_key();
    let legacy = layout
        .data_dir()
        .join(crate::commands::runner_join::MACHINE_KEY);
    private_files::ensure_dir(&layout.root.join("keys"))?;
    let held = present(&path)?;
    if present(&legacy)? {
        let older =
            Ed25519Identity::load(&legacy).map_err(|error| failure("load", &legacy, error))?;
        if held {
            let current =
                Ed25519Identity::load(&path).map_err(|error| failure("load", &path, error))?;
            if current.public_key_bytes() != older.public_key_bytes() {
                return Err(failure(
                    "migrate",
                    &path,
                    "runner_machine_key_conflict: the old join and the installed key name different computers",
                ));
            }
            fs::remove_file(&legacy)
                .map_err(|error| failure("remove migrated key", &legacy, error))?;
        } else {
            fs::rename(&legacy, &path).map_err(|error| failure("move", &path, error))?;
        }
        for parent in [layout.root.join("keys"), layout.data_dir()] {
            fs::File::open(&parent)
                .and_then(|directory| directory.sync_all())
                .map_err(|error| failure("sync migrated key directory", &parent, error))?;
        }
        say(&format!("runner machine key moved to {}", path.display()));
        return Ok(older);
    }
    let key = if held {
        Ed25519Identity::load(&path)
    } else {
        Ed25519Identity::load_or_generate(&path)
    }
    .map_err(|error| failure("provide", &path, error))?;
    if !held {
        say(&format!("runner machine key created at {}", path.display()));
    }
    Ok(key)
}

#[cfg(test)]
#[path = "runner_machine_tests.rs"]
mod tests;
