//! What a session is launched as: its bound directory, the harness
//! executable and version checked before it runs, that start said in the
//! feed, and the leader a full plan window stops.

use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, mpsc};

use portable_pty::Child;
use serde_json::Value;

use super::super::{Live, Session, Sessions, Starting, Table, now_ms, unknown};
use super::{Wake, accounts, append, plan, stop_follower, transcript_parent, window_limit};
use crate::error::RunnerError;
use crate::peer::Leader;
use crate::protocol::{Ended, EndedHow, Launch};
use crate::tracking::{Accounts, Harness, Reading, Tracking, version_in};
use crate::tracking_store::{Body, Commit, Coverage, SourceState};

/// The directory a session is bound to: `directory` resolved, or as given
/// when it cannot be.
pub(crate) fn bound_directory(directory: &str) -> String {
    let given = if directory.is_empty() { "." } else { directory };
    std::fs::canonicalize(given).map_or_else(
        |error| {
            crate::error::said(&format!(
                "{given} does not resolve, and is kept as given: {error}"
            ));
            given.to_owned()
        },
        |path| path.display().to_string(),
    )
}

/// The executable `launch` runs, found as its environment's `PATH` finds
/// it, and the version it reports, refused `tracking_contract_unsupported`
/// unless it is the version `tracking` declares.
pub(crate) fn launched(
    launch: &Launch,
    tracking: &Tracking,
) -> Result<(String, String), RunnerError> {
    let unsupported = |words: String| RunnerError::refused("tracking_contract_unsupported", words);
    let program = Path::new(&launch.program);
    let found = if launch.program.contains('/') {
        Some(program.to_owned())
    } else {
        let path = launch
            .environment
            .get("PATH")
            .cloned()
            .or_else(|| std::env::var("PATH").ok())
            .unwrap_or_default();
        std::env::split_paths(&path)
            .map(|dir| dir.join(program))
            .find(|candidate| candidate.is_file())
    };
    let executable = found
        .and_then(|found| std::fs::canonicalize(found).ok())
        .ok_or_else(|| {
            unsupported(format!(
                "{} is not found to ask its version",
                launch.program
            ))
        })?;
    let output = std::process::Command::new(&executable)
        .arg("--version")
        .output()
        .map_err(|error| {
            unsupported(format!(
                "{} did not say its version: {error}",
                executable.display()
            ))
        })?;
    let said = String::from_utf8_lossy(&output.stdout);
    let version = version_in(&said)
        .ok_or_else(|| unsupported(format!("{} names no version", executable.display())))?;
    crate::tracking::measured(&tracking.adapter, &version)?;
    if version != tracking.version {
        return Err(unsupported(format!(
            "{} is version {version}, and the profile declares {}",
            executable.display(),
            tracking.version
        )));
    }
    Ok((executable.display().to_string(), version))
}

/// Say, in the feed, the executable and version launched for session `id`.
pub(crate) fn tracking_started(
    table: &mut Table,
    id: &str,
    executable: &str,
    version: &str,
) -> Result<(), RunnerError> {
    let tracking = table
        .sessions
        .get(id)
        .and_then(|session| session.guard.tracking.as_ref());
    let adapter = tracking.map(|tracking| tracking.adapter.clone());
    let coverage = Coverage {
        state: "tracking_started".to_owned(),
        source: None,
        generation: 0,
        offset: None,
        words: format!("launched {executable}, which says it is version {version}"),
        executable: Some(executable.to_owned()),
        harness_version: Some(version.to_owned()),
        adapter,
    };
    append(table, id, vec![Body::Coverage(coverage)], None)
}

pub(crate) fn window_limit(table: &mut Table, id: &str, bodies: &[Body]) -> Option<Leader> {
    let session = table.sessions.get_mut(id)?;
    if session.ending {
        return None;
    }
    let rotation = session.rotation.as_mut()?;
    if rotation.tripped() {
        return None;
    }
    let reached = bodies.iter().any(|body| match body {
        Body::Usage(record) => {
            record.account.as_deref() == Some(rotation.account())
                && rotation.windows_in(&record.figures.plan_windows, now_ms())
        }
        _ => false,
    });
    if reached {
        rotation.trip();
        if session.guard.leader.is_none() {
            crate::error::said("rotation_signal_failed: the process's leader is unproved");
        }
        return session.guard.leader.clone();
    }
    None
}
