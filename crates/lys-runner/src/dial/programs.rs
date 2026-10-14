//! Programme discovery belongs to the computer that will execute the harness.

use std::ffi::OsStr;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::{Command, Stdio};

use serde::{Deserialize, Serialize};

use crate::RunnerError;

/// An executable and the adapter that understands its protocol.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HarnessProgram {
    /// The command as the login PATH names it (`claude`, `codex`, `cdx`), so a
    /// launch always runs the copy installed at that moment, updates included.
    pub program: String,
    /// The version its own version command reported.
    pub version: String,
    /// The managed adapter matching this programme.
    pub adapter: String,
}

/// Discovery evidence, including the reason no executable was offered.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Programs {
    /// Each executable found, without repeats.
    pub programs: Vec<HarnessProgram>,
    /// Missing programmes and failed version probes, in words.
    pub reason: Option<String>,
}

fn failed(detail: impl Into<String>) -> RunnerError {
    RunnerError::refused("harness_programs_unreadable", detail)
}

/// Discover the runner login's harnesses without inspecting another computer.
///
/// # Errors
/// Reports an unreadable login PATH or a failed discovery operation by name.
pub fn discover() -> Result<Programs, RunnerError> {
    let shell =
        std::env::var_os("SHELL").ok_or_else(|| failed("the runner login names no SHELL"))?;
    if !Path::new(&shell).is_absolute() {
        return Err(failed("the runner login's SHELL is not absolute"));
    }
    let output = Command::new(shell)
        .args(["-l", "-c", "printf '%s' \"$PATH\""])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .map_err(|error| failed(format!("the login PATH could not be read: {error}")))?;
    if !output.status.success() {
        return Err(failed(format!(
            "the login PATH command exited {}",
            output.status
        )));
    }
    let path = String::from_utf8(output.stdout)
        .map_err(|error| failed(format!("the login PATH is not text: {error}")))?;
    discover_in(OsStr::new(&path))
}

/// Discover from an explicit PATH, also used by the real executable fixtures.
///
/// # Errors
/// Returns an unreadable filesystem entry or a failed executable resolution.
pub fn discover_in(path: &OsStr) -> Result<Programs, RunnerError> {
    let mut programs = Vec::<HarnessProgram>::new();
    let mut resolved = Vec::<std::path::PathBuf>::new();
    let mut reasons = Vec::new();
    for (command, adapter) in [
        ("claude", "claude-stream-json/1"),
        ("codex", "codex-app-server/1"),
        ("cdx", "codex-app-server/1"),
    ] {
        let mut selected = None;
        for directory in std::env::split_paths(path) {
            if !directory.is_absolute() {
                continue;
            }
            let candidate = directory.join(command);
            match std::fs::metadata(&candidate) {
                Ok(meta) if meta.is_file() && meta.permissions().mode() & 0o111 != 0 => {
                    selected = Some(candidate.canonicalize().map_err(|error| {
                        failed(format!("{command} cannot be resolved: {error}"))
                    })?);
                    break;
                }
                Ok(_) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => {
                    return Err(failed(format!("{command} cannot be inspected: {error}")));
                }
            }
        }
        let Some(selected) = selected else {
            continue;
        };
        if resolved.contains(&selected) {
            continue;
        }
        resolved.push(selected.clone());
        let program = command.to_owned();
        let output = Command::new(&selected)
            .arg("--version")
            .env("PATH", path)
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .output()
            .map_err(|error| failed(format!("{command} could not report its version: {error}")))?;
        if !output.status.success() {
            reasons.push(format!("{command} --version exited {}", output.status));
            continue;
        }
        let text = String::from_utf8(output.stdout)
            .map_err(|error| failed(format!("{command} version is not text: {error}")))?;
        match crate::tracking::version_in(&text) {
            Some(version) => programs.push(HarnessProgram {
                program,
                version,
                adapter: adapter.to_owned(),
            }),
            None => reasons.push(format!("{command} reported no version")),
        }
    }
    if programs.is_empty() && reasons.is_empty() {
        reasons.push("no_harness_programs: no Claude Code or Codex executable was found on this computer's login PATH".to_owned());
    }
    Ok(Programs {
        programs,
        reason: (!reasons.is_empty()).then(|| reasons.join("; ")),
    })
}

#[cfg(test)]
mod tests {
    use super::discover_in;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn programmes_answer_commands_versions_and_adapters_from_the_chosen_path()
    -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        for (command, report) in [
            ("claude", "2.1.300 (Claude Code)"),
            ("codex", "codex-cli 0.161.0-alpha.3"),
        ] {
            let program = dir.path().join(command);
            std::fs::write(&program, format!("#!/bin/sh\nprintf '%s\\n' '{report}'\n"))?;
            std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755))?;
        }
        let answer = discover_in(dir.path().as_os_str())?;
        assert_eq!(answer.programs.len(), 2);
        assert_eq!(answer.programs[0].program, "claude");
        assert_eq!(answer.programs[1].program, "codex");
        assert_eq!(answer.programs[0].version, "2.1.300");
        assert_eq!(answer.programs[0].adapter, "claude-stream-json/1");
        assert_eq!(answer.programs[1].version, "0.161.0-alpha.3");
        assert!(answer.reason.is_none());
        Ok(())
    }

    #[test]
    fn no_programmes_are_answered_as_an_empty_list_with_words()
    -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let answer = discover_in(dir.path().as_os_str())?;
        assert!(answer.programs.is_empty());
        assert!(
            answer
                .reason
                .as_deref()
                .is_some_and(|reason| reason.contains("no_harness_programs"))
        );
        Ok(())
    }
    #[test]
    fn an_unreadable_version_report_is_a_named_discovery_refusal()
    -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let program = dir.path().join("claude");
        std::fs::write(&program, "#!/bin/sh\nprintf '\\377'\n")?;
        std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755))?;
        let error = discover_in(dir.path().as_os_str())
            .err()
            .ok_or("invalid version accepted")?;
        assert_eq!(error.name(), "harness_programs_unreadable");
        Ok(())
    }
}
