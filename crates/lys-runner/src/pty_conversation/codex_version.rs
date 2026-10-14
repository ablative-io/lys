//! Native resume is enabled only for builds proved to refuse missing UUIDs.

use crate::error::RunnerError;
use crate::protocol::Launch;

#[cfg(test)]
#[path = "codex_version_tests.rs"]
mod tests;

fn unsupported(reason: impl std::fmt::Display) -> RunnerError {
    RunnerError::refused(
        "pty_resume_unsupported",
        format!("{reason}; choose a reviewed Codex command or build"),
    )
}

fn reported(launch: &Launch) -> Result<String, RunnerError> {
    let selected =
        crate::program_path::selected(&launch.program, &launch.environment).map_err(|error| {
            unsupported(format!(
                "the native Codex executable cannot be selected: {error}"
            ))
        })?;
    let (status, bytes) =
        crate::harness_control::process::report_version(&selected, &launch.environment).map_err(
            |error| {
                unsupported(format!(
                    "the native Codex version cannot be proved: {error}"
                ))
            },
        )?;
    if !status.success() {
        return Err(unsupported(format!(
            "the native Codex version probe exited {status}"
        )));
    }
    let report = String::from_utf8(bytes).map_err(|error| {
        unsupported(format!(
            "the native Codex version report is not UTF-8: {error}"
        ))
    })?;
    let version = crate::tracking::version_in(&report)
        .ok_or_else(|| unsupported("the native Codex command reported no semantic version"))?;
    Ok(version)
}

fn reviewed(version: &str) -> bool {
    matches!(version, "0.161.0-alpha.3" | "0.161.0-channels.6")
}

pub(super) fn fresh_note(launch: &Launch) -> Result<Option<String>, RunnerError> {
    let version = reported(launch)?;
    Ok((!reviewed(&version))
        .then(|| format!("resume not proven for codex {version}, started fresh")))
}

pub(super) fn prove(launch: &Launch) -> Result<(), RunnerError> {
    let version = reported(launch)?;
    if !reviewed(&version) {
        return Err(unsupported(format!(
            "Codex {version} has no reviewed proof that native resume refuses a missing UUID"
        )));
    }
    Ok(())
}
