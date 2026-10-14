//! Programme discovery belongs to the computer that will execute the harness.

use std::ffi::OsStr;
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
    let path = crate::program_path::login_path().map_err(failed)?;
    discover_in(&path)
}

/// Discover from an explicit PATH, also used by the real executable fixtures.
///
/// # Errors
/// Returns an unreadable filesystem entry or a failed executable resolution.
pub fn discover_in(path: &OsStr) -> Result<Programs, RunnerError> {
    let mut programs = Vec::<HarnessProgram>::new();
    let mut reasons = Vec::new();
    for (command, adapter) in [
        ("claude", "claude-stream-json/1"),
        ("codex", "codex-app-server/1"),
        ("cdx", "codex-app-server/1"),
    ] {
        let Some(selected) = crate::program_path::resolve_in(command, path).map_err(failed)? else {
            continue;
        };
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
    #[test]
    fn codex_and_cdx_aliases_are_both_reported() -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        let codex = directory.path().join("codex");
        std::fs::write(&codex, "#!/bin/sh\nprintf 'codex-cli 0.161.0\\n'\n")?;
        std::fs::set_permissions(&codex, std::fs::Permissions::from_mode(0o755))?;
        std::os::unix::fs::symlink(&codex, directory.path().join("cdx"))?;
        let answer = discover_in(directory.path().as_os_str())?;
        assert_eq!(
            answer
                .programs
                .iter()
                .map(|program| program.program.as_str())
                .collect::<Vec<_>>(),
            ["codex", "cdx"]
        );
        Ok(())
    }

    #[test]
    fn the_version_probe_extracts_semver_and_keeps_the_selected_symlinked_path()
    -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        let installed = directory.path().join("installed");
        std::fs::create_dir(&installed)?;
        let chosen = directory.path().join("chosen-path");
        std::os::unix::fs::symlink(&installed, &chosen)?;
        for (name, script) in [
            (
                "claude",
                "#!/bin/sh\nprintf '%s\\n' \"$0\" > \"$0.invoked\"\nprintf '%s\\n' \"$PATH\" > \"$0.path\"\nexec harness-helper\n",
            ),
            ("harness-helper", "#!/bin/sh\nprintf 'claude 2.1.300\\n'\n"),
        ] {
            let path = installed.join(name);
            std::fs::write(&path, script)?;
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))?;
        }
        let answer = discover_in(chosen.as_os_str())?;
        assert_eq!(answer.programs.len(), 1, "{:?}", answer.reason);
        assert_eq!(answer.programs[0].program, "claude");
        assert_eq!(answer.programs[0].version, "2.1.300");
        assert!(answer.reason.is_none(), "{:?}", answer.reason);
        assert_eq!(
            std::fs::read_to_string(chosen.join("claude.invoked"))?,
            format!("{}\n", chosen.join("claude").display())
        );
        assert_eq!(
            std::fs::read_to_string(chosen.join("claude.path"))?,
            format!("{}\n", chosen.display())
        );
        Ok(())
    }
}
