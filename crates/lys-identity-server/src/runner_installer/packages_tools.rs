//! The Mac package tools run, and a build's staging folder removed, each
//! failure named by what failed.

use std::process::{Command, Stdio};

use super::refused;
use crate::error::ServerError;

pub(super) fn run_tool(mut command: Command, name: &str) -> Result<(), ServerError> {
    let output = command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .map_err(|error| {
            refused(
                &format!("installer_{name}_unavailable"),
                format!("the {name} package tool could not be started: {error}"),
            )
        })?;
    if !output.status.success() {
        // The tool's own last words say why; the connection code never reaches its arguments or output.
        let said = String::from_utf8_lossy(&output.stderr);
        let last: Vec<&str> = said
            .lines()
            .filter(|line| !line.trim().is_empty())
            .collect();
        let tail = last[last.len().saturating_sub(3)..].join(" | ");
        return Err(refused(
            &format!("installer_{name}_failed"),
            if tail.is_empty() {
                format!(
                    "the {name} package tool failed with {} and said nothing",
                    output.status
                )
            } else {
                format!(
                    "the {name} package tool failed with {}: {tail}",
                    output.status
                )
            },
        ));
    }
    Ok(())
}

/// A build's answer once its staging folder is removed: a folder that could
/// not be removed is named, beside the build's own failure when both failed.
pub(super) fn settled<T>(
    result: Result<T, ServerError>,
    cleanup: std::io::Result<()>,
) -> Result<T, ServerError> {
    let cleanup = cleanup.map_err(|error| refused("installer_cleanup_failed", error));
    match (result, cleanup) {
        (Ok(bytes), Ok(())) => Ok(bytes),
        (Err(error), Ok(())) | (Ok(_), Err(error)) => Err(error),
        (Err(build), Err(cleanup)) => Err(refused(
            "installer_cleanup_failed",
            format!("{build}; {cleanup}"),
        )),
    }
}
