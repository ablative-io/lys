#![cfg(test)]
//! Only proved Codex versions can pass native terminal resume preflight.

use std::collections::BTreeMap;
use std::error::Error;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use super::prove;
use crate::protocol::Launch;

type TestResult = Result<(), Box<dyn Error>>;

struct Fixture {
    root: tempfile::TempDir,
    program: PathBuf,
    launch: Launch,
}

impl Fixture {
    fn new() -> Result<Self, Box<dyn Error>> {
        let root = tempfile::tempdir()?;
        let program = root.path().join("controlled-codex");
        let launch = Launch {
            session: "op-00112233445566778899aabbccddeeff".to_owned(),
            program: program
                .to_str()
                .ok_or("fixture executable is not UTF-8")?
                .to_owned(),
            arguments: vec![
                "resume".to_owned(),
                "00112233-4455-6677-8899-aabbccddeeff".to_owned(),
            ],
            directory: root.path().display().to_string(),
            environment: BTreeMap::new(),
            config: None,
            columns: 80,
            rows: 24,
            rotation: None,
            policy: None,
        };
        Ok(Self {
            root,
            program,
            launch,
        })
    }

    fn script(&self, body: &str) -> TestResult {
        std::fs::write(
            &self.program,
            format!("#!/bin/sh\nif [ \"$1\" != '--version' ]; then exit 91; fi\n{body}\n"),
        )?;
        std::fs::set_permissions(&self.program, std::fs::Permissions::from_mode(0o700))?;
        Ok(())
    }
}

#[test]
fn only_reviewed_fail_closed_codex_builds_pass_the_normalized_native_resume_version_proof()
-> TestResult {
    let fixture = Fixture::new()?;
    for version in ["0.161.0-alpha.3", "0.161.0-channels.6"] {
        fixture.script(&format!("printf 'codex-cli {version}\\n'"))?;
        let before = fixture.launch.clone();
        prove(&fixture.launch)?;
        assert_eq!(fixture.launch, before);
    }
    Ok(())
}

#[test]
fn legacy_unreviewed_and_unknown_codex_versions_refuse_resume_without_fresh_fallback() -> TestResult
{
    let fixture = Fixture::new()?;
    for version in [
        "0.36.0",
        "0.161.0",
        "0.161.0-alpha.4",
        "0.161.0-channels.7",
        "1.0.0",
    ] {
        fixture.script(&format!("printf 'codex-cli {version}\\n'"))?;
        let before = fixture.launch.clone();
        let error = prove(&fixture.launch).unwrap_err();
        assert_eq!(error.name(), "pty_resume_unsupported", "{error}");
        assert!(error.to_string().contains(version), "{error}");
        assert!(
            error
                .to_string()
                .contains("choose a reviewed Codex command or build"),
            "{error}"
        );
        assert!(!error.to_string().contains("Start again"), "{error}");
        assert_eq!(fixture.launch, before);
    }
    Ok(())
}

#[test]
fn missing_non_text_non_version_and_failed_probe_reports_refuse_with_one_build_action() -> TestResult
{
    let fixture = Fixture::new()?;
    let missing = prove(&fixture.launch).unwrap_err();
    assert_eq!(missing.name(), "pty_resume_unsupported", "{missing}");
    for body in [
        "printf '\\377'",
        "printf 'no version\\n'",
        "printf 'codex-cli 0.161.0-alpha.3\\n'\nexit 7",
    ] {
        fixture.script(body)?;
        let error = prove(&fixture.launch).unwrap_err();
        assert_eq!(error.name(), "pty_resume_unsupported", "{error}");
        assert!(
            error
                .to_string()
                .contains("choose a reviewed Codex command or build"),
            "{error}"
        );
    }
    Ok(())
}

#[test]
fn the_version_proof_executes_the_launch_selected_command_and_uses_its_environment() -> TestResult {
    let mut fixture = Fixture::new()?;
    let chosen = fixture.root.path().join("chosen-path");
    std::fs::create_dir(&chosen)?;
    let command = chosen.join("cdx");
    std::fs::write(
        &command,
        "#!/bin/sh\nif [ \"$1\" != '--version' ] || [ \"$PROBE_ENV\" != 'selected' ]; then exit 91; fi\nprintf 'codex-cli 0.161.0-channels.6\\n'\n",
    )?;
    std::fs::set_permissions(command, std::fs::Permissions::from_mode(0o700))?;
    fixture.launch.program = "cdx".to_owned();
    fixture.launch.environment = BTreeMap::from([
        (
            "PATH".to_owned(),
            chosen
                .to_str()
                .ok_or("fixture PATH is not UTF-8")?
                .to_owned(),
        ),
        ("PROBE_ENV".to_owned(), "selected".to_owned()),
    ]);
    prove(&fixture.launch)?;
    Ok(())
}

#[test]
fn an_unreviewed_codex_can_start_fresh_with_a_named_status_note_but_cannot_resume() -> TestResult {
    let fixture = Fixture::new()?;
    fixture.script("printf 'codex-cli 0.36.0\\n'")?;
    assert_eq!(
        super::fresh_note(&fixture.launch)?,
        Some("resume not proven for codex 0.36.0, started fresh".to_owned())
    );
    assert_eq!(
        prove(&fixture.launch).unwrap_err().name(),
        "pty_resume_unsupported"
    );
    Ok(())
}
