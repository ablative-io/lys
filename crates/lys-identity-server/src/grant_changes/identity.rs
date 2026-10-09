//! The grant log's stable identity and reset epoch (DIRECTORY-089 R1).
//!
//! The identity is recorded once, in a file beside the grant log's own
//! directory, so no signed leaf, operation or snapshot of the log is
//! rewritten, and a store migration that rebuilds the log's directory
//! keeps it. Opening an install that predates it records it then; every
//! later opening reads the same one back. It is published with a hard link
//! from a synced file, so two openings never record two identities and a
//! crash before the link leaves none: until the link is durable no identity
//! is answered. A file that cannot be read is refused by name and never
//! replaced by a fresh identity. An intentional replacement is [`reset`]:
//! it names the old identity and the new, and moves the epoch.

use std::io::Write;
use std::path::{Path, PathBuf};

use lys_identity::OperationId;
use lys_identity::grants::change_stream::LogName;
use serde::{Deserialize, Serialize};

use crate::error_grant_stream::GrantStreamError;

/// The version of the identity file this service writes and reads.
const IDENTITY_VERSION: u64 = 1;

/// The identity file, as it is kept.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Recorded {
    version: u64,
    /// The origin the grant log was created with, when this was recorded.
    origin: String,
    identity: String,
    epoch: u64,
    /// The identity an explicit reset replaced; absent for the first.
    replaces: Option<LogName>,
}

/// An explicit replacement of the log's identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reset {
    /// The identity replaced.
    pub old: LogName,
    /// The identity now served.
    pub new: LogName,
}

fn unavailable(reason: impl Into<String>) -> GrantStreamError {
    GrantStreamError::IdentityUnavailable {
        reason: reason.into(),
    }
}

/// The identity file of the grant log kept at `log_dir`: beside it, named
/// after it.
pub(crate) fn path_for(log_dir: &Path) -> Result<PathBuf, GrantStreamError> {
    let name = log_dir.file_name().ok_or_else(|| {
        unavailable("the grant log directory has no name to keep its identity by")
    })?;
    let mut file = name.to_os_string();
    file.push(".identity.json");
    Ok(log_dir.with_file_name(file))
}

fn read(path: &Path, bytes: &[u8]) -> Result<LogName, GrantStreamError> {
    let recorded: Recorded = serde_json::from_slice(bytes).map_err(|error| {
        unavailable(format!(
            "the grant log identity {} cannot be read: {error}",
            path.display()
        ))
    })?;
    if recorded.version != IDENTITY_VERSION || recorded.identity.is_empty() {
        return Err(unavailable(format!(
            "the grant log identity {} is version {} or empty; this service reads version {IDENTITY_VERSION}",
            path.display(),
            recorded.version
        )));
    }
    Ok(LogName {
        identity: recorded.identity,
        epoch: recorded.epoch,
    })
}

fn minted(origin: &str) -> Result<String, GrantStreamError> {
    let nonce = OperationId::generate()
        .map_err(|error| unavailable(format!("the secure random source failed: {error}")))?;
    Ok(format!("{origin}#{nonce}"))
}

/// `recorded`, written to a fresh synced file beside `path`, answered by
/// that file's path.
fn written(path: &Path, recorded: &Recorded) -> Result<PathBuf, GrantStreamError> {
    let bytes = serde_json::to_vec_pretty(recorded)
        .map_err(|error| unavailable(format!("the identity could not be encoded: {error}")))?;
    let mut name = path.as_os_str().to_os_string();
    name.push(format!(".{}.writing", minted("")?.trim_start_matches('#')));
    let temporary = PathBuf::from(name);
    let io = |error: std::io::Error| {
        unavailable(format!(
            "the grant log identity could not be written at {}: {error}",
            temporary.display()
        ))
    };
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(io)?;
    file.write_all(&bytes).map_err(io)?;
    file.sync_all().map_err(io)?;
    Ok(temporary)
}

fn sync_parent(path: &Path) -> Result<(), GrantStreamError> {
    let parent = path
        .parent()
        .ok_or_else(|| unavailable("the grant log identity has no folder to sync"))?;
    std::fs::File::open(parent)
        .and_then(|folder| folder.sync_all())
        .map_err(|error| {
            unavailable(format!(
                "the folder of {} could not be synced: {error}",
                path.display()
            ))
        })
}

/// The identity of the grant log kept at `log_dir`, recorded now, from the
/// log's `origin`, when it was never recorded.
///
/// # Errors
/// `grant_log_identity_unavailable` when the file cannot be read, or the
/// new identity cannot be made durable; no identity is answered then.
pub(crate) fn establish(log_dir: &Path, origin: &str) -> Result<LogName, GrantStreamError> {
    let path = path_for(log_dir)?;
    match std::fs::read(&path) {
        Ok(bytes) => return read(&path, &bytes),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(unavailable(format!(
                "the grant log identity {} cannot be read: {error}",
                path.display()
            )));
        }
    }
    let recorded = Recorded {
        version: IDENTITY_VERSION,
        origin: origin.to_owned(),
        identity: minted(origin)?,
        epoch: 0,
        replaces: None,
    };
    let temporary = written(&path, &recorded)?;
    let linked = std::fs::hard_link(&temporary, &path);
    let removed = std::fs::remove_file(&temporary);
    match linked {
        Ok(()) => {}
        // Another opening recorded it first: that one is the identity.
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            let bytes = std::fs::read(&path).map_err(|error| {
                unavailable(format!(
                    "the grant log identity {} cannot be read: {error}",
                    path.display()
                ))
            })?;
            return read(&path, &bytes);
        }
        Err(error) => {
            return Err(unavailable(format!(
                "the grant log identity {} could not be published: {error}",
                path.display()
            )));
        }
    }
    sync_parent(&path)?;
    if let Err(error) = removed {
        tracing::warn!(error = %error, "the grant log identity's written copy was not removed");
    }
    Ok(LogName {
        identity: recorded.identity,
        epoch: recorded.epoch,
    })
}

/// Replace the identity of the grant log kept at `log_dir` deliberately:
/// a new identity, in the next epoch, naming the one it replaces.
///
/// # Errors
/// `grant_log_identity_unavailable` when no identity is recorded to
/// replace, or the replacement cannot be made durable.
pub fn reset(log_dir: &Path, origin: &str) -> Result<Reset, GrantStreamError> {
    let path = path_for(log_dir)?;
    let bytes = std::fs::read(&path).map_err(|error| {
        unavailable(format!(
            "no grant log identity {} can be read to replace: {error}",
            path.display()
        ))
    })?;
    let old = read(&path, &bytes)?;
    let epoch = old
        .epoch
        .checked_add(1)
        .ok_or_else(|| unavailable("the reset epoch cannot move past its last value"))?;
    let recorded = Recorded {
        version: IDENTITY_VERSION,
        origin: origin.to_owned(),
        identity: minted(origin)?,
        epoch,
        replaces: Some(old.clone()),
    };
    let temporary = written(&path, &recorded)?;
    std::fs::rename(&temporary, &path).map_err(|error| {
        unavailable(format!(
            "the grant log identity {} could not be replaced: {error}",
            path.display()
        ))
    })?;
    sync_parent(&path)?;
    Ok(Reset {
        old,
        new: LogName {
            identity: recorded.identity,
            epoch,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::{establish, path_for, reset};

    /// An install with no identity records one once; reopening answers the
    /// same; a reset names the old and the new and moves the epoch; an
    /// unreadable file is refused and never replaced.
    #[test]
    fn an_identity_is_recorded_once_and_replaced_only_by_reset()
    -> Result<(), Box<dyn std::error::Error>> {
        let home = tempfile::tempdir()?;
        let log_dir = home.path().join("grants");
        std::fs::create_dir(&log_dir)?;
        let first = establish(&log_dir, "example.test/lys/grants")?;
        assert_eq!(first.epoch, 0);
        assert!(first.identity.starts_with("example.test/lys/grants#"));
        for _ in 0..2 {
            assert_eq!(establish(&log_dir, "example.test/lys/grants")?, first);
        }
        let leftovers = std::fs::read_dir(home.path())?.count();
        assert_eq!(
            leftovers, 2,
            "the log folder and its identity, nothing written beside"
        );

        let replaced = reset(&log_dir, "example.test/lys/grants")?;
        assert_eq!(replaced.old, first);
        assert_eq!(replaced.new.epoch, 1);
        assert_ne!(replaced.new.identity, first.identity);
        assert_eq!(
            establish(&log_dir, "example.test/lys/grants")?,
            replaced.new
        );

        std::fs::write(path_for(&log_dir)?, b"not json")?;
        let refused = establish(&log_dir, "example.test/lys/grants")
            .err()
            .ok_or("an unreadable identity was answered")?;
        assert_eq!(refused.name(), "grant_log_identity_unavailable");
        assert_eq!(std::fs::read(path_for(&log_dir)?)?, b"not json");
        Ok(())
    }
}
