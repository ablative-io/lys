//! What a session is launched as: its bound directory, the harness
//! executable and version checked before it runs, that start said in the
//! feed, and the leader a full plan window stops.

use std::path::Path;

use super::super::{Table, now_ms};
use super::append;
use crate::error::RunnerError;
use crate::peer::Leader;
use crate::protocol::Launch;
use crate::tracking::{Tracking, version_in};
use crate::tracking_store::{Body, Coverage};

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

/// The executable the launch names, without replacing a missing input.
fn executable(launch: &Launch) -> Result<String, RunnerError> {
    let unsupported = |words: String| RunnerError::refused("tracking_contract_unsupported", words);
    let program = Path::new(&launch.program);
    let found = if launch.program.contains('/') {
        program.to_owned()
    } else {
        let path = launch
            .environment
            .get("PATH")
            .map(std::ffi::OsString::from)
            .or_else(|| std::env::var_os("PATH"))
            .ok_or_else(|| unsupported("the launch and runner name no PATH".to_owned()))?;
        std::env::split_paths(&path)
            .map(|dir| dir.join(program))
            .find(|candidate| candidate.is_file())
            .ok_or_else(|| unsupported(format!("{} is not found in PATH", launch.program)))?
    };
    let canonical = std::fs::canonicalize(&found)
        .map_err(|error| unsupported(format!("{} does not resolve: {error}", found.display())))?;
    canonical
        .to_str()
        .map(str::to_owned)
        .ok_or_else(|| unsupported("the executable's resolved path is not UTF-8".to_owned()))
}

/// Find the executable, measuring its version only when tracking declares one.
pub(crate) fn launched(
    launch: &Launch,
    tracking: &Tracking,
) -> Result<(String, String), RunnerError> {
    let unsupported = |words: String| RunnerError::refused("tracking_contract_unsupported", words);
    let executable = executable(launch)?;
    if tracking.version.is_empty() {
        return Ok((executable, String::new()));
    }
    let output = std::process::Command::new(&executable)
        .arg("--version")
        .output()
        .map_err(|error| unsupported(format!("{executable} did not say its version: {error}")))?;
    if !output.status.success() {
        return Err(unsupported(format!(
            "{executable} --version exited {}",
            output.status
        )));
    }
    let said = String::from_utf8_lossy(&output.stdout);
    let version =
        version_in(&said).ok_or_else(|| unsupported(format!("{executable} names no version")))?;
    crate::tracking::measured(&tracking.adapter, &version)?;
    if version != tracking.version {
        return Err(unsupported(format!(
            "{executable} is version {version}, and the profile declares {}",
            tracking.version
        )));
    }
    Ok((executable, version))
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
    let words = if version.is_empty() {
        let tracking = tracking.ok_or_else(|| {
            RunnerError::refused(
                "tracking_start_missing",
                "an unmeasured start has no held harness tracking",
            )
        })?;
        let source = table
            .sessions
            .get(id)
            .and_then(|session| session.launch.as_ref())
            .and_then(|launch| launch.environment.get(crate::trust::CONFIG_HOME_SOURCE))
            .map_or("spec", String::as_str);
        serde_json::to_string(&serde_json::json!({
            "executable": executable,
            "harness": tracking.harness,
            "harness_version": null,
            "adapter": tracking.adapter,
            "context_window": null,
            "config_home": tracking.config_home,
            "config_home_source": source,
            "reason": "hook tracking has no measured version or declared window"
        }))
        .map_err(|error| RunnerError::refused("tracking_start_unrecordable", error.to_string()))?
    } else {
        format!("launched {executable}, which says it is version {version}")
    };
    let coverage = Coverage {
        state: "tracking_started".to_owned(),
        source: None,
        generation: 0,
        offset: None,
        words,
        executable: Some(executable.to_owned()),
        harness_version: (!version.is_empty()).then(|| version.to_owned()),
        adapter,
    };
    append(table, id, vec![Body::Coverage(coverage)], None)
}

/// Say, in the feed, that session `id`'s folder was recorded as trusted in
/// the harness's own configuration before the spawn, or found there already.
pub(crate) fn trust_recorded(
    table: &mut Table,
    id: &str,
    trust: &crate::trust::Trust,
) -> Result<(), RunnerError> {
    let coverage = Coverage {
        state: "trust_recorded".to_owned(),
        source: None,
        generation: 0,
        offset: None,
        words: trust.words(),
        executable: None,
        harness_version: None,
        adapter: None,
    };
    append(table, id, vec![Body::Coverage(coverage)], None)
}

/// Say, in the feed, what the watch of session `id`'s first screen found of
/// the trust dialog: `state` is `trust_answered` or
/// `trust_dialog_unanswered`, and `words` say which folder and what was done.
pub(crate) fn trust_dialog(
    table: &mut Table,
    id: &str,
    state: &str,
    words: String,
) -> Result<(), RunnerError> {
    let coverage = Coverage {
        state: state.to_owned(),
        source: None,
        generation: 0,
        offset: None,
        words,
        executable: None,
        harness_version: None,
        adapter: None,
    };
    append(table, id, vec![Body::Coverage(coverage)], None)
}

pub(crate) fn window_limit(table: &mut Table, id: &str, bodies: &[Body]) -> Option<Leader> {
    let session = table.sessions.get_mut(id)?;
    // A run's usage file is read on after its process ended; a figure read
    // then signals no process.
    if session.ending || session.ended.is_some() {
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
