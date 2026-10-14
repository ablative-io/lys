#![cfg(test)]
//! Codex terminal resume reads one native binding, never a guessed thread.

use std::collections::BTreeMap;
use std::error::Error;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use serde_json::{Value, json};

use super::preflight;
use crate::protocol::Launch;

type TestResult = Result<(), Box<dyn Error>>;
const UUID: &str = "00112233-4455-6677-8899-aabbccddeeff";

struct Fixture {
    root: tempfile::TempDir,
    launch: Launch,
    rollout: PathBuf,
}

impl Fixture {
    fn new() -> Result<Self, Box<dyn Error>> {
        let root = tempfile::tempdir()?;
        let canonical = root.path().canonicalize()?;
        let directory = canonical.join("work");
        std::fs::create_dir(&directory)?;
        let config = canonical.join("codex-home");
        let program = canonical.join("cdx");
        std::fs::write(
            &program,
            "#!/bin/sh\nif [ \"$1\" != '--version' ]; then exit 91; fi\nprintf 'codex-cli 0.161.0-alpha.3\\n'\n",
        )?;
        std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o700))?;
        let rollout = config
            .join("sessions/2026/10/14")
            .join(format!("rollout-2026-10-14T21-00-00-{UUID}.jsonl"));
        std::fs::create_dir_all(rollout.parent().ok_or("rollout has no parent")?)?;
        let launch = Launch {
            session: "op-00112233445566778899aabbccddeeff".to_owned(),
            program: program
                .to_str()
                .ok_or("fixture program is not UTF-8")?
                .to_owned(),
            arguments: [
                "--model",
                "chosen",
                "-c",
                "service_tier=\"priority\"",
                "resume",
                UUID,
            ]
            .map(str::to_owned)
            .to_vec(),
            directory: directory
                .to_str()
                .ok_or("fixture directory is not UTF-8")?
                .to_owned(),
            environment: BTreeMap::from([
                (
                    "CODEX_HOME".to_owned(),
                    config
                        .to_str()
                        .ok_or("fixture config is not UTF-8")?
                        .to_owned(),
                ),
                ("HOME".to_owned(), "relative-home-must-not-win".to_owned()),
            ]),
            config: None,
            columns: 80,
            rows: 24,
            rotation: None,
            policy: None,
        };
        Ok(Self {
            root,
            launch,
            rollout,
        })
    }

    fn metadata(&self) -> Value {
        json!({"type": "session_meta", "payload": {"id": UUID, "cwd": self.launch.directory}})
    }

    fn write(&self, metadata: &Value) -> TestResult {
        let mut bytes = serde_json::to_vec(metadata)?;
        bytes.extend_from_slice(b"\nIgnored conversation payload is never parsed by preflight.\n");
        std::fs::write(&self.rollout, bytes)?;
        Ok(())
    }
}

#[test]
fn explicit_codex_resume_proves_first_metadata_and_preserves_all_existing_launch_inputs()
-> TestResult {
    let fixture = Fixture::new()?;
    fixture.write(&fixture.metadata())?;
    let before = fixture.launch.clone();
    preflight(&fixture.launch)?;
    assert_eq!(fixture.launch, before);
    Ok(())
}

#[test]
fn a_fresh_codex_terminal_does_not_invent_a_thread_or_require_native_history() -> TestResult {
    let mut fixture = Fixture::new()?;
    fixture.launch.arguments.truncate(4);
    fixture.launch.directory = "not-read-for-a-fresh-launch".to_owned();
    let before = fixture.launch.clone();
    preflight(&fixture.launch)?;
    assert_eq!(fixture.launch, before);
    Ok(())
}

#[test]
fn codex_resume_uses_effective_child_home_when_codex_home_is_absent() -> TestResult {
    let mut fixture = Fixture::new()?;
    let home = fixture.root.path().canonicalize()?.join("child-home");
    fixture.launch.environment.remove("CODEX_HOME");
    fixture
        .launch
        .environment
        .insert("HOME".to_owned(), home.display().to_string());
    fixture.rollout = home
        .join(".codex/sessions/2026/10/14")
        .join(format!("rollout-2026-10-14T21-00-00-{UUID}.jsonl"));
    std::fs::create_dir_all(fixture.rollout.parent().ok_or("rollout has no parent")?)?;
    fixture.write(&fixture.metadata())?;
    preflight(&fixture.launch)?;
    Ok(())
}

#[test]
fn a_missing_codex_rollout_is_refused_with_start_again_and_never_fresh_arguments() -> TestResult {
    let fixture = Fixture::new()?;
    let before = fixture.launch.clone();
    let error = preflight(&fixture.launch).unwrap_err();
    assert_eq!(error.name(), "pty_resume_missing", "{error}");
    assert!(error.to_string().contains("Start again"), "{error}");
    assert_eq!(fixture.launch, before);
    Ok(())
}

#[test]
fn codex_picker_last_prompt_and_duplicate_resume_forms_are_refused_by_name() -> TestResult {
    let mut fixture = Fixture::new()?;
    for arguments in [
        vec!["resume"],
        vec!["resume", "--last"],
        vec!["--last", "resume", UUID],
        vec!["resume", UUID, "a-new-prompt"],
        vec!["resume", UUID, "resume", UUID],
        vec!["resume", "--", UUID],
    ] {
        fixture.launch.arguments = arguments.into_iter().map(str::to_owned).collect();
        let error = preflight(&fixture.launch).unwrap_err();
        assert_eq!(error.name(), "pty_resume_arguments_invalid", "{error}");
        assert!(error.to_string().contains("Start again"), "{error}");
    }
    fixture.launch.arguments = vec!["resume".to_owned(), "a-session-name".to_owned()];
    let error = preflight(&fixture.launch).unwrap_err();
    assert_eq!(error.name(), "pty_resume_conversation_invalid", "{error}");
    Ok(())
}

#[test]
fn a_filename_match_cannot_resume_a_different_codex_conversation() -> TestResult {
    let fixture = Fixture::new()?;
    let mut metadata = fixture.metadata();
    metadata["payload"]["id"] = json!("11112233-4455-6677-8899-aabbccddeeff");
    fixture.write(&metadata)?;
    let error = preflight(&fixture.launch).unwrap_err();
    assert_eq!(error.name(), "pty_resume_other_session", "{error}");
    assert!(error.to_string().contains("Start again"), "{error}");
    Ok(())
}

#[test]
fn a_matching_codex_id_cannot_resume_another_working_directory() -> TestResult {
    let fixture = Fixture::new()?;
    let other = fixture.root.path().canonicalize()?.join("other-work");
    std::fs::create_dir(&other)?;
    let mut metadata = fixture.metadata();
    metadata["payload"]["cwd"] = json!(other);
    fixture.write(&metadata)?;
    let error = preflight(&fixture.launch).unwrap_err();
    assert_eq!(error.name(), "pty_resume_other_directory", "{error}");
    assert!(error.to_string().contains("Start again"), "{error}");
    Ok(())
}

#[test]
fn codex_resume_canonicalizes_the_declared_working_directory() -> TestResult {
    let mut fixture = Fixture::new()?;
    fixture.write(&fixture.metadata())?;
    let alias = fixture.root.path().join("chosen-work");
    std::os::unix::fs::symlink(&fixture.launch.directory, &alias)?;
    fixture.launch.directory = alias
        .to_str()
        .ok_or("fixture alias is not UTF-8")?
        .to_owned();
    preflight(&fixture.launch)?;
    Ok(())
}

#[test]
fn malformed_incomplete_or_non_session_first_records_refuse_codex_resume() -> TestResult {
    let fixture = Fixture::new()?;
    for first in [
        "",
        "{\n",
        "[]\n",
        "{\"type\":\"session_meta\",\"payload\":{}}\n",
        "{\"type\":\"response_item\",\"payload\":{\"id\":\"00112233-4455-6677-8899-aabbccddeeff\",\"cwd\":\"/\"}}\n",
    ] {
        std::fs::write(&fixture.rollout, first)?;
        let error = preflight(&fixture.launch).unwrap_err();
        assert_eq!(error.name(), "pty_resume_metadata_invalid", "{error}");
        assert!(error.to_string().contains("Start again"), "{error}");
    }
    std::fs::write(&fixture.rollout, serde_json::to_vec(&fixture.metadata())?)?;
    let error = preflight(&fixture.launch).unwrap_err();
    assert_eq!(error.name(), "pty_resume_metadata_invalid", "{error}");
    Ok(())
}

#[test]
fn multiple_rollouts_for_one_codex_id_are_refused_instead_of_selecting_a_copy() -> TestResult {
    let fixture = Fixture::new()?;
    fixture.write(&fixture.metadata())?;
    let duplicate = fixture
        .rollout
        .parent()
        .ok_or("rollout has no parent")?
        .join(format!("rollout-2026-10-14T21-00-01-{UUID}.jsonl"));
    std::fs::copy(&fixture.rollout, duplicate)?;
    let error = preflight(&fixture.launch).unwrap_err();
    assert_eq!(error.name(), "pty_resume_ambiguous", "{error}");
    assert!(error.to_string().contains("Start again"), "{error}");
    Ok(())
}

#[test]
fn a_matching_codex_rollout_directory_is_refused_without_reading_message_content() -> TestResult {
    let fixture = Fixture::new()?;
    std::fs::create_dir(&fixture.rollout)?;
    let error = preflight(&fixture.launch).unwrap_err();
    assert_eq!(error.name(), "pty_resume_unreadable", "{error}");
    assert!(error.to_string().contains("Start again"), "{error}");
    Ok(())
}

#[test]
fn native_codex_metadata_refusal_precedes_executable_selection_or_a_version_probe() -> TestResult {
    let mut fixture = Fixture::new()?;
    fixture.launch.program = fixture
        .root
        .path()
        .join("no-command-was-installed")
        .display()
        .to_string();
    let missing = preflight(&fixture.launch).unwrap_err();
    assert_eq!(missing.name(), "pty_resume_missing", "{missing}");
    let mut other = fixture.metadata();
    other["payload"]["id"] = json!("11112233-4455-6677-8899-aabbccddeeff");
    fixture.write(&other)?;
    let mismatch = preflight(&fixture.launch).unwrap_err();
    assert_eq!(mismatch.name(), "pty_resume_other_session", "{mismatch}");
    Ok(())
}

#[test]
fn a_proved_local_transcript_cannot_enable_native_resume_on_an_unsafe_codex_build() -> TestResult {
    let fixture = Fixture::new()?;
    fixture.write(&fixture.metadata())?;
    std::fs::write(
        &fixture.launch.program,
        "#!/bin/sh\nprintf 'codex-cli 0.36.0\\n'\n",
    )?;
    let before = fixture.launch.clone();
    let error = preflight(&fixture.launch).unwrap_err();
    assert_eq!(error.name(), "pty_resume_unsupported", "{error}");
    assert!(
        error
            .to_string()
            .contains("reviewed Codex command or build"),
        "{error}"
    );
    assert_eq!(fixture.launch, before);
    Ok(())
}
