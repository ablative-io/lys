#![cfg(test)]
//! Native resume keeps identity and folder binding, or refuses before spawn.

use std::collections::BTreeMap;
use std::error::Error;
use std::io::{BufReader, Cursor, Read};
use std::path::PathBuf;

use serde_json::{Value, json};

use super::{
    claude_arguments, conversation_uuid, preflight, preflight_claude_resume, verify_metadata,
};
use crate::protocol::Launch;

type TestResult = Result<(), Box<dyn Error>>;

const SESSION: &str = "op-00112233445566778899aabbccddeeff";
const UUID: &str = "00112233-4455-6677-8899-aabbccddeeff";

#[test]
fn the_complete_operation_hex_becomes_a_stable_native_uuid() -> TestResult {
    assert_eq!(conversation_uuid(SESSION)?, UUID);
    assert_eq!(
        conversation_uuid("op-00112233445566778899AABBCCDDEEFF")?,
        UUID
    );
    Ok(())
}

#[test]
fn invalid_operation_ids_are_refused_instead_of_getting_a_new_identity() {
    for session in [
        "",
        UUID,
        "op-00112233445566778899aabbccddeef",
        "op-00112233445566778899aabbccddeeff0",
        "op-00112233445566778899aabbccddeefg",
        " op-00112233445566778899aabbccddeeff",
        "op-00112233445566778899aabbccddeeff\n",
        "op-00112233445566778899aabbccddeeé",
    ] {
        let error = conversation_uuid(session).unwrap_err();
        assert_eq!(error.name(), "pty_conversation_invalid", "{error}");
    }
}

#[test]
fn fresh_and_resumed_terminal_arguments_preserve_every_configuration_argument() -> TestResult {
    let given = [
        "--settings",
        "/chosen/settings.json",
        "--mcp-config=/chosen/mcp.json",
        "--append-system-prompt",
        "The existing instructions.",
        "--model",
        "chosen-model",
    ]
    .map(str::to_owned);
    for (resume, flag) in [(false, "--session-id"), (true, "--resume")] {
        let answer = claude_arguments(&given, UUID, resume)?;
        assert_eq!(&answer[..given.len()], &given);
        assert_eq!(&answer[given.len()..], [flag, UUID]);
        assert_eq!(answer.len(), given.len() + 2);
    }
    Ok(())
}

#[test]
fn native_identity_and_persistence_conflicts_are_refused_without_overwriting_arguments() {
    for conflict in [
        "--session-id",
        "--session-id=another-id",
        "--resume",
        "--resume=another-id",
        "-r",
        "-ranother-id",
        "--continue",
        "-c",
        "--fork-session",
        "--no-session-persistence",
        "--",
    ] {
        let given = vec![
            "--model".to_owned(),
            "chosen-model".to_owned(),
            conflict.to_owned(),
        ];
        let before = given.clone();
        for resume in [false, true] {
            let error = claude_arguments(&given, UUID, resume).unwrap_err();
            assert_eq!(error.name(), "pty_conversation_conflict", "{error}");
        }
        assert_eq!(given, before);
    }
}

#[test]
fn native_argument_binding_refuses_a_missing_or_non_uuid_conversation() {
    for conversation in ["", SESSION, "00112233-4455-6677-8899-aabbccddeefg"] {
        for resume in [false, true] {
            let error = claude_arguments(&[], conversation, resume).unwrap_err();
            assert_eq!(error.name(), "pty_conversation_invalid", "{error}");
        }
    }
}

struct Fixture {
    root: tempfile::TempDir,
    launch: Launch,
    transcript: PathBuf,
}

impl Fixture {
    fn new() -> Result<Self, Box<dyn Error>> {
        let root = tempfile::tempdir()?;
        let canonical = root.path().canonicalize()?;
        let directory = canonical.join("work_one");
        std::fs::create_dir(&directory)?;
        let config = canonical.join("claude-config");
        let cwd = directory.to_str().ok_or("fixture directory is not UTF-8")?;
        let transcript = config
            .join("projects")
            .join(lys_home::harness::claude_code::projects_slug(cwd))
            .join(format!("{UUID}.jsonl"));
        std::fs::create_dir_all(transcript.parent().ok_or("transcript has no parent")?)?;
        let launch = Launch {
            session: SESSION.to_owned(),
            program: "claude".to_owned(),
            arguments: vec!["--model=chosen".to_owned()],
            directory: cwd.to_owned(),
            environment: BTreeMap::from([
                (
                    "CLAUDE_CONFIG_DIR".to_owned(),
                    config
                        .to_str()
                        .ok_or("fixture config is not UTF-8")?
                        .to_owned(),
                ),
                ("HOME".to_owned(), "a-relative-home-must-not-win".to_owned()),
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
            transcript,
        })
    }

    fn metadata(&self) -> Value {
        json!({
            "type": "user", "sessionId": UUID, "cwd": self.launch.directory,
            "message": {"content": "Only the top-level binding is inspected."}
        })
    }

    fn write(&self, records: &[Value]) -> TestResult {
        let mut bytes = Vec::new();
        for record in records {
            serde_json::to_writer(&mut bytes, record)?;
            bytes.push(b'\n');
        }
        std::fs::write(&self.transcript, bytes)?;
        Ok(())
    }

    fn declare_harness(&mut self, harness: crate::tracking::Harness) {
        self.launch.config = Some(crate::launch_config::Config {
            requires_controls: false,
            files: Vec::new(),
            argument_files: BTreeMap::new(),
            environment_paths: BTreeMap::new(),
            working_directory: false,
            harness: Some(harness),
        });
    }
}

#[test]
fn terminal_preflight_dispatches_only_the_declared_harness_and_requires_no_transcript_for_fresh()
-> TestResult {
    let mut fixture = Fixture::new()?;
    fixture.launch.arguments = vec!["--resume".to_owned(), UUID.to_owned()];
    preflight(&fixture.launch)?;
    fixture.declare_harness(crate::tracking::Harness::ClaudeCode);
    let error = preflight(&fixture.launch).unwrap_err();
    assert_eq!(error.name(), "pty_resume_missing", "{error}");
    fixture.launch.arguments = claude_arguments(&[], UUID, false)?;
    preflight(&fixture.launch)?;
    fixture.write(&[fixture.metadata()])?;
    fixture.launch.arguments = claude_arguments(&[], UUID, true)?;
    preflight(&fixture.launch)?;
    Ok(())
}

#[test]
fn terminal_claude_resume_refuses_picker_fork_duplicate_and_conflicting_native_flags() -> TestResult
{
    let mut fixture = Fixture::new()?;
    fixture.declare_harness(crate::tracking::Harness::ClaudeCode);
    for arguments in [
        vec!["--resume"],
        vec!["--continue"],
        vec!["-c"],
        vec!["--fork-session", "--resume", UUID],
        vec!["--resume", UUID, "--resume", UUID],
        vec!["--resume", UUID, "--session-id", UUID],
        vec!["--resume", UUID, "--no-session-persistence"],
        vec!["--", "--resume", UUID],
    ] {
        fixture.launch.arguments = arguments.into_iter().map(str::to_owned).collect();
        let error = preflight(&fixture.launch).unwrap_err();
        assert_eq!(error.name(), "pty_resume_arguments_invalid", "{error}");
        assert!(error.to_string().contains("Start again"), "{error}");
    }
    Ok(())
}

#[test]
fn native_resume_uses_explicit_config_and_proves_metadata_after_non_message_records() -> TestResult
{
    let fixture = Fixture::new()?;
    fixture.write(&[json!({"type": "file-history-snapshot"}), fixture.metadata()])?;
    let before = fixture.launch.clone();
    assert_eq!(
        preflight_claude_resume(&fixture.launch, UUID)?,
        fixture.transcript
    );
    assert_eq!(fixture.launch, before);
    Ok(())
}

#[test]
fn native_resume_uses_the_child_home_when_no_claude_config_directory_is_set() -> TestResult {
    let mut fixture = Fixture::new()?;
    let home = fixture.root.path().canonicalize()?.join("effective-home");
    fixture.launch.environment.remove("CLAUDE_CONFIG_DIR");
    fixture
        .launch
        .environment
        .insert("HOME".to_owned(), home.display().to_string());
    fixture.transcript = home
        .join(".claude/projects")
        .join(lys_home::harness::claude_code::projects_slug(
            &fixture.launch.directory,
        ))
        .join(format!("{UUID}.jsonl"));
    std::fs::create_dir_all(
        fixture
            .transcript
            .parent()
            .ok_or("transcript has no parent")?,
    )?;
    fixture.write(&[fixture.metadata()])?;
    assert_eq!(
        preflight_claude_resume(&fixture.launch, UUID)?,
        fixture.transcript
    );
    Ok(())
}

#[test]
fn a_symlinked_working_directory_resumes_the_same_canonical_project() -> TestResult {
    let mut fixture = Fixture::new()?;
    fixture.write(&[fixture.metadata()])?;
    let alias = fixture.root.path().join("chosen-work");
    std::os::unix::fs::symlink(&fixture.launch.directory, &alias)?;
    fixture.launch.directory = alias
        .to_str()
        .ok_or("fixture alias is not UTF-8")?
        .to_owned();
    assert_eq!(
        preflight_claude_resume(&fixture.launch, UUID)?,
        fixture.transcript
    );
    Ok(())
}

#[test]
fn a_missing_native_transcript_directs_start_again_without_changing_the_launch() -> TestResult {
    let fixture = Fixture::new()?;
    let before = fixture.launch.clone();
    let error = preflight_claude_resume(&fixture.launch, UUID).unwrap_err();
    assert_eq!(error.name(), "pty_resume_missing", "{error}");
    assert!(error.to_string().contains("Start again"), "{error}");
    assert_eq!(fixture.launch, before);
    Ok(())
}

#[test]
fn a_transcript_directory_is_refused_as_unreadable_before_any_record_is_read() -> TestResult {
    let fixture = Fixture::new()?;
    std::fs::create_dir(&fixture.transcript)?;
    let error = preflight_claude_resume(&fixture.launch, UUID).unwrap_err();
    assert_eq!(error.name(), "pty_resume_unreadable", "{error}");
    assert!(error.to_string().contains("Start again"), "{error}");
    Ok(())
}

#[test]
fn a_project_slug_collision_cannot_resume_another_working_directory() -> TestResult {
    let fixture = Fixture::new()?;
    let other = fixture.root.path().canonicalize()?.join("work-one");
    std::fs::create_dir(&other)?;
    assert_eq!(
        lys_home::harness::claude_code::projects_slug(&fixture.launch.directory),
        lys_home::harness::claude_code::projects_slug(
            other.to_str().ok_or("other cwd is not UTF-8")?
        )
    );
    let mut metadata = fixture.metadata();
    metadata["cwd"] = json!(other);
    fixture.write(&[metadata])?;
    let error = preflight_claude_resume(&fixture.launch, UUID).unwrap_err();
    assert_eq!(error.name(), "pty_resume_other_directory", "{error}");
    assert!(error.to_string().contains("Start again"), "{error}");
    Ok(())
}

#[test]
fn a_later_record_for_another_session_refuses_the_whole_native_transcript() -> TestResult {
    let fixture = Fixture::new()?;
    let mut other = fixture.metadata();
    other["sessionId"] = json!("11112233-4455-6677-8899-aabbccddeeff");
    fixture.write(&[fixture.metadata(), other])?;
    let error = preflight_claude_resume(&fixture.launch, UUID).unwrap_err();
    assert_eq!(error.name(), "pty_resume_other_session", "{error}");
    assert!(error.to_string().contains("Start again"), "{error}");
    Ok(())
}

#[test]
fn empty_or_separately_partial_metadata_cannot_prove_a_native_resume() -> TestResult {
    let fixture = Fixture::new()?;
    for records in [
        vec![],
        vec![json!({"type": "file-history-snapshot"})],
        vec![
            json!({"sessionId": UUID}),
            json!({"cwd": fixture.launch.directory}),
        ],
    ] {
        fixture.write(&records)?;
        let error = preflight_claude_resume(&fixture.launch, UUID).unwrap_err();
        assert_eq!(error.name(), "pty_resume_unproved", "{error}");
        assert!(error.to_string().contains("Start again"), "{error}");
    }
    Ok(())
}

#[test]
fn malformed_or_non_object_native_metadata_refuses_with_a_start_again_action() -> TestResult {
    let fixture = Fixture::new()?;
    for bytes in ["{", "[]", "{\"sessionId\":7}", "\n"] {
        std::fs::write(&fixture.transcript, bytes)?;
        let error = preflight_claude_resume(&fixture.launch, UUID).unwrap_err();
        assert_eq!(error.name(), "pty_resume_metadata_invalid", "{error}");
        assert!(error.to_string().contains("Start again"), "{error}");
    }
    Ok(())
}

struct Unreadable;

impl Read for Unreadable {
    fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
        Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "fixture reader refused",
        ))
    }
}

#[test]
fn an_unreadable_metadata_stream_is_refused_without_a_fresh_conversation() -> TestResult {
    let fixture = Fixture::new()?;
    let reader = BufReader::new(Unreadable);
    let error = verify_metadata(
        reader,
        std::path::Path::new(&fixture.launch.directory),
        UUID,
    )
    .unwrap_err();
    assert_eq!(error.name(), "pty_resume_unreadable", "{error}");
    assert!(error.to_string().contains("Start again"), "{error}");
    let error = verify_metadata(
        Cursor::new(b""),
        std::path::Path::new(&fixture.launch.directory),
        UUID,
    )
    .unwrap_err();
    assert_eq!(error.name(), "pty_resume_unproved", "{error}");
    Ok(())
}

#[test]
fn invalid_resume_identity_home_or_directory_is_refused_before_file_lookup() -> TestResult {
    let mut fixture = Fixture::new()?;
    let error = preflight_claude_resume(&fixture.launch, SESSION).unwrap_err();
    assert_eq!(error.name(), "pty_resume_conversation_invalid", "{error}");
    assert!(error.to_string().contains("Start again"), "{error}");
    fixture
        .launch
        .environment
        .insert("CLAUDE_CONFIG_DIR".to_owned(), "relative-config".to_owned());
    let error = preflight_claude_resume(&fixture.launch, UUID).unwrap_err();
    assert_eq!(error.name(), "pty_resume_home_unknown", "{error}");
    assert!(error.to_string().contains("Start again"), "{error}");
    fixture.launch.directory = "relative-work".to_owned();
    let error = preflight_claude_resume(&fixture.launch, UUID).unwrap_err();
    assert_eq!(error.name(), "pty_resume_directory_invalid", "{error}");
    assert!(error.to_string().contains("Start again"), "{error}");
    Ok(())
}
