//! Rebinding a replacement runner to the owners a dead one started
//! (AGENTS-004 R2).
//!
//! A runner that starts an owner writes the owned seat into the owner's
//! directory once the owner is ready. A runner opening its state reads one
//! such record per owner directory, never a journal, never a session's
//! history, and proves each owner at the kernel: the pid is alive and its
//! start identity is the recorded one. An owner the kernel does not confirm
//! is unreachable and named so, with its record kept: a dead owner is
//! unknown authority, not proof that the seat is offline, and nothing is
//! restarted or replayed for it here.

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::protocol::OWNERS_DIR;
use super::sessions::OwnedSeat;
use crate::error::RunnerError;
use crate::peer;

/// The owned-seat record in an owner's directory.
pub const ENDPOINT: &str = "endpoint.json";

/// The most owner directories a runner reads at open.
pub const MAX_OWNERS_FOUND: usize = 4096;

/// Why an owner on record could not be reached.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "why", rename_all = "snake_case", deny_unknown_fields)]
pub enum Unreachable {
    /// No process has the owner's pid.
    Exited,
    /// A process has the pid, with another start identity: the pid was
    /// reused, and that process is not the owner.
    PidReused {
        /// What the kernel reports for that pid now.
        found: String,
    },
    /// The kernel could not be asked.
    Unproved {
        /// The kernel's refusal, in words.
        reason: String,
    },
}

/// One owner found at open.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Found {
    /// The seat as recorded when its owner became ready.
    pub seat: OwnedSeat,
    /// Why the owner cannot be reached, when it cannot.
    pub unreachable: Option<Unreachable>,
}

impl Found {
    /// Whether the owner was proved live.
    #[must_use]
    pub fn live(&self) -> bool {
        self.unreachable.is_none()
    }
}

/// What a recovery read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RecoveryCounts {
    /// Owner directories visited.
    pub record_visits: u64,
    /// Bytes read.
    pub bytes_copied: u64,
    /// Kernel proofs asked.
    pub proofs: u64,
}

/// Writes the owned seat into its owner's directory, replaced whole.
///
/// # Errors
///
/// `seat_owner_start_failed` when it cannot be written durably.
pub fn record_endpoint(seat: &OwnedSeat) -> Result<(), RunnerError> {
    let path = seat.endpoint.dir.join(ENDPOINT);
    let bytes = serde_json::to_vec_pretty(seat)
        .map_err(|error| RunnerError::refused("seat_owner_start_failed", error.to_string()))?;
    crate::state::replace(&path, &bytes).map_err(|error| {
        RunnerError::refused(
            "seat_owner_start_failed",
            format!("writing {}: {error}", path.display()),
        )
    })
}

/// Proves `seat`'s owner at the kernel.
#[must_use]
pub fn prove(seat: &OwnedSeat) -> Option<Unreachable> {
    let owner = &seat.endpoint.owner;
    match peer::start_identity(owner.pid) {
        Ok(start) if start == owner.start => None,
        Ok(start) => Some(Unreachable::PidReused { found: start.0 }),
        Err(error) => {
            let words = error.to_string();
            if words.contains("cannot be read") || words.contains("not a process") {
                Some(Unreachable::Exited)
            } else {
                Some(Unreachable::Unproved { reason: words })
            }
        }
    }
}

/// Whether `recorded` is the directory `found`, by the kernel's identity
/// (device and inode) rather than by spelling: the runner records its
/// canonical state path and a reader may reach the same directory through a
/// symbolic link (macOS's `/var` is `/private/var`). A recorded directory
/// that does not exist is another directory.
///
/// # Errors
///
/// `seat_owner_store_unavailable` when either directory's identity cannot be
/// read for another reason than its absence.
fn same_directory(recorded: &Path, found: &Path) -> Result<bool, RunnerError> {
    use std::os::unix::fs::MetadataExt;
    let identity = |path: &Path| match fs::metadata(path) {
        Ok(metadata) => Ok(Some((metadata.dev(), metadata.ino()))),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(RunnerError::refused(
            "seat_owner_store_unavailable",
            format!("reading {}: {error}", path.display()),
        )),
    };
    let (Some(recorded), Some(found)) = (identity(recorded)?, identity(found)?) else {
        return Ok(false);
    };
    Ok(recorded == found)
}

/// Reads every owner directory under `state` and proves each owner.
///
/// # Errors
///
/// `seat_owner_store_unavailable` when the owners directory cannot be
/// listed; `seat_owner_record_invalid` for an endpoint record that does
/// not read or names another directory, which is never skipped;
/// `seat_owner_bound_exceeded` over [`MAX_OWNERS_FOUND`].
pub fn recover(state: &Path) -> Result<(Vec<Found>, RecoveryCounts), RunnerError> {
    let owners = state.join(OWNERS_DIR);
    let mut counts = RecoveryCounts::default();
    let entries = match fs::read_dir(&owners) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok((Vec::new(), counts));
        }
        Err(error) => {
            return Err(RunnerError::refused(
                "seat_owner_store_unavailable",
                format!("listing {}: {error}", owners.display()),
            ));
        }
    };
    let mut found = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| {
            RunnerError::refused(
                "seat_owner_store_unavailable",
                format!("listing {}: {error}", owners.display()),
            )
        })?;
        let dir = entry.path();
        if !dir.is_dir() {
            continue;
        }
        if found.len() >= MAX_OWNERS_FOUND {
            return Err(RunnerError::refused(
                "seat_owner_bound_exceeded",
                format!("{MAX_OWNERS_FOUND} owner directories are the bound"),
            ));
        }
        counts.record_visits = counts.record_visits.saturating_add(1);
        let path = dir.join(ENDPOINT);
        let bytes = match fs::read(&path) {
            Ok(bytes) => bytes,
            // An owner directory with no endpoint yet is one whose start
            // never reached ready: nothing to bind, nothing to replay.
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => {
                return Err(RunnerError::refused(
                    "seat_owner_store_unavailable",
                    format!("reading {}: {error}", path.display()),
                ));
            }
        };
        counts.bytes_copied = counts.bytes_copied.saturating_add(bytes.len() as u64);
        let seat: OwnedSeat = serde_json::from_slice(&bytes).map_err(|error| {
            RunnerError::refused(
                "seat_owner_record_invalid",
                format!("{} does not read: {error}", path.display()),
            )
        })?;
        if dir
            .file_name()
            .is_none_or(|name| name != seat.binding.session.as_str())
            || !same_directory(&seat.endpoint.dir, &dir)?
        {
            return Err(RunnerError::refused(
                "seat_owner_record_invalid",
                format!("{} names another owner directory", path.display()),
            ));
        }
        counts.proofs = counts.proofs.saturating_add(1);
        let unreachable = prove(&seat);
        found.push(Found { seat, unreachable });
    }
    found.sort_by(|a, b| a.seat.binding.session.cmp(&b.seat.binding.session));
    Ok((found, counts))
}
