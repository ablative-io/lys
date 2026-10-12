//! Build metadata reports a join receipt only when it names this install's key.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::{Layout, provide};
use crate::identity::error::{ErrorKind, IdentityError, IdentityResult};
use crate::identity::private_files;

/// The installed key and the join receipt belonging to that key, when one exists.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Recorded {
    /// The private key's location; the key's seed is never recorded here.
    pub key_file: PathBuf,
    /// The Ed25519 public key, in hexadecimal.
    pub key: String,
    /// The computer named by its most recent join, or null while unjoined.
    pub computer: Option<String>,
    /// The identity made at that join, or null for a join predating machine identities.
    pub identity: Option<String>,
}

#[derive(Deserialize)]
struct Joins {
    format: String,
    codes: Vec<Join>,
}

#[derive(Deserialize)]
struct Join {
    machine: String,
    standing: Standing,
}

#[derive(Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
enum Standing {
    Waiting,
    Replaced,
    Used {
        key: String,
        identity: Option<String>,
    },
}

fn failed(path: &Path, reason: impl ToString) -> IdentityError {
    IdentityError::new(
        ErrorKind::PrivateFileIo,
        "record",
        "runner machine record",
        reason.to_string(),
    )
    .at(path)
}

fn identity(text: &str, prefix: &str) -> bool {
    text.strip_prefix(prefix)
        .is_some_and(|hex| hex.len() == 32 && hex.bytes().all(|byte| byte.is_ascii_hexdigit()))
}

/// Read the key's join receipt without rotating or inventing a computer identity.
///
/// # Errors
/// Names invalid keys, unreadable joins and a key recorded against several computers.
pub fn recorded(layout: &Layout, say: &mut dyn FnMut(&str)) -> IdentityResult<Recorded> {
    let key = lys_runner::protocol::hex(&provide(layout, say)?.public_key_bytes());
    let path = layout.data_dir().join("network.joins.json");
    let mut recorded = Recorded {
        key_file: layout.runner_machine_key(),
        key,
        computer: None,
        identity: None,
    };
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(recorded),
        Err(error) => return Err(failed(&path, error)),
    };
    let joins: Joins = serde_json::from_slice(&bytes).map_err(|error| failed(&path, error))?;
    if joins.format != "lys-runner-joins/v1" {
        return Err(failed(&path, "runner_machine_joins_format_unknown"));
    }
    for join in joins.codes {
        let Standing::Used {
            key,
            identity: joined,
        } = join.standing
        else {
            continue;
        };
        if !key.eq_ignore_ascii_case(&recorded.key) {
            continue;
        }
        if !identity(&join.machine, "op-")
            || joined
                .as_deref()
                .is_some_and(|id| !identity(id, "machine-"))
        {
            return Err(failed(&path, "runner_machine_join_identity_invalid"));
        }
        if recorded
            .computer
            .as_ref()
            .is_some_and(|held| held != &join.machine)
        {
            return Err(failed(
                &path,
                "runner_machine_join_conflict: this key names several computers",
            ));
        }
        recorded.computer = Some(join.machine);
        recorded.identity = joined;
    }
    Ok(recorded)
}

/// Atomically replace build metadata, preserving the old shape before its first migration.
///
/// # Errors
/// Names malformed old records, conflicting backups and every failed durable write.
pub fn write_build_record(
    layout: &Layout,
    bytes: &[u8],
    say: &mut dyn FnMut(&str),
) -> IdentityResult<()> {
    let path = layout.build_record();
    let old = match fs::read(&path) {
        Ok(bytes) => Some(bytes),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(failed(&path, error)),
    };
    if let Some(old) = old {
        let value: serde_json::Value =
            serde_json::from_slice(&old).map_err(|error| failed(&path, error))?;
        if value.get("runner_machine").is_none() {
            let backup = layout
                .install_dir()
                .join("build.before-runner-machine.json");
            match fs::read(&backup) {
                Ok(held) if held != old => {
                    return Err(failed(&backup, "runner_machine_build_backup_conflict"));
                }
                Ok(_) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    private_files::write(&backup, &old)?;
                }
                Err(error) => return Err(failed(&backup, error)),
            }
            atomic(&path, bytes)?;
            say(&format!(
                "build.json gained runner machine metadata; previous record kept at {}",
                backup.display()
            ));
            return Ok(());
        }
    }
    atomic(&path, bytes)
}

fn atomic(path: &Path, bytes: &[u8]) -> IdentityResult<()> {
    let parent = path
        .parent()
        .ok_or_else(|| failed(path, "the build record has no directory"))?;
    let temporary = parent.join(format!(
        ".build-machine-{}-{}",
        std::process::id(),
        rand::random::<u64>()
    ));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&temporary)
        .map_err(|error| failed(&temporary, error))?;
    let result = (|| {
        file.write_all(bytes)?;
        file.sync_all()?;
        fs::rename(&temporary, path)?;
        fs::File::open(parent)?.sync_all()
    })();
    drop(file);
    if let Err(error) = result {
        match fs::remove_file(&temporary) {
            Ok(()) => {}
            Err(cleanup) if cleanup.kind() == std::io::ErrorKind::NotFound => {}
            Err(cleanup) => {
                return Err(failed(
                    path,
                    format!("{error}; removing the temporary record: {cleanup}"),
                ));
            }
        }
        return Err(failed(path, error));
    }
    Ok(())
}
