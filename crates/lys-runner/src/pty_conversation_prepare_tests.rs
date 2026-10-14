#![cfg(test)]
//! Common terminal preparation binds fresh operations without replacing resume.

use std::collections::BTreeMap;
use std::error::Error;

use serde_json::json;

use super::prepare;
use crate::launch_config::Config;
use crate::protocol::Launch;
use crate::tracking::Harness;

type TestResult = Result<(), Box<dyn Error>>;
const SESSION: &str = "op-00112233445566778899aabbccddeeff";
const UUID: &str = "00112233-4455-6677-8899-aabbccddeeff";

fn launch() -> Launch {
    Launch {
        session: SESSION.to_owned(),
        program: "claude".to_owned(),
        arguments: vec!["--model".to_owned(), "chosen".to_owned()],
        directory: "/not-needed-for-fresh-preparation".to_owned(),
        environment: BTreeMap::new(),
        config: Some(Config {
            requires_controls: false,
            files: Vec::new(),
            argument_files: BTreeMap::new(),
            environment_paths: BTreeMap::new(),
            working_directory: false,
            harness: Some(Harness::ClaudeCode),
        }),
        columns: 80,
        rows: 24,
        rotation: None,
        policy: None,
    }
}

#[test]
fn a_fresh_operation_started_by_either_server_route_gets_its_native_uuid_once() -> TestResult {
    let mut prepared = launch();
    let before = prepared.clone();
    prepare(&mut prepared)?;
    assert_eq!(
        prepared.arguments,
        ["--model", "chosen", "--session-id", UUID]
    );
    let arguments = prepared.arguments.clone();
    prepare(&mut prepared)?;
    assert_eq!(prepared.arguments, arguments);
    prepared.arguments = before.arguments.clone();
    assert_eq!(prepared, before);
    Ok(())
}

#[test]
fn a_root_provided_matching_native_uuid_is_preserved_without_duplicate_flags() -> TestResult {
    for arguments in [
        vec![
            "--model".to_owned(),
            "chosen".to_owned(),
            "--session-id".to_owned(),
            UUID.to_owned(),
        ],
        vec![
            "--model".to_owned(),
            "chosen".to_owned(),
            format!("--session-id={UUID}"),
        ],
    ] {
        let mut prepared = launch();
        prepared.arguments = arguments;
        let before = prepared.clone();
        prepare(&mut prepared)?;
        assert_eq!(prepared, before);
    }
    Ok(())
}

#[test]
fn a_native_uuid_that_differs_from_the_operation_is_refused_without_changing_the_launch() {
    let mut prepared = launch();
    prepared.arguments.extend([
        "--session-id".to_owned(),
        "11112233-4455-6677-8899-aabbccddeeff".to_owned(),
    ]);
    let before = prepared.clone();
    let error = prepare(&mut prepared).unwrap_err();
    assert_eq!(error.name(), "pty_conversation_conflict", "{error}");
    assert_eq!(prepared, before);
}

#[test]
fn malformed_or_duplicate_explicit_native_ids_are_refused_without_fresh_replacement() {
    for (arguments, refusal) in [
        (vec!["--session-id"], "pty_conversation_invalid"),
        (
            vec!["--session-id", "not-a-uuid"],
            "pty_conversation_invalid",
        ),
        (
            vec!["--session-id", UUID, "--session-id", UUID],
            "pty_conversation_conflict",
        ),
    ] {
        let mut prepared = launch();
        prepared.arguments = arguments.into_iter().map(str::to_owned).collect();
        let before = prepared.clone();
        let error = prepare(&mut prepared).unwrap_err();
        assert_eq!(error.name(), refusal, "{error}");
        assert_eq!(prepared, before);
    }
}

#[test]
fn native_resume_is_preflighted_and_preserved_without_becoming_a_fresh_operation() -> TestResult {
    let directory = tempfile::tempdir()?;
    let cwd = directory.path().canonicalize()?;
    let cwd_text = cwd
        .to_str()
        .ok_or("fixture working directory is not UTF-8")?;
    let config = cwd.join("claude-config");
    let transcript = config
        .join("projects")
        .join(lys_home::harness::claude_code::projects_slug(cwd_text))
        .join(format!("{UUID}.jsonl"));
    std::fs::create_dir_all(
        transcript
            .parent()
            .ok_or("fixture transcript has no parent")?,
    )?;
    let mut prepared = launch();
    prepared.directory = cwd_text.to_owned();
    prepared
        .environment
        .insert("CLAUDE_CONFIG_DIR".to_owned(), config.display().to_string());
    prepared
        .arguments
        .extend(["--resume".to_owned(), UUID.to_owned()]);
    let before = prepared.clone();
    prepare(&mut prepared)?;
    assert!(
        prepared
            .arguments
            .iter()
            .any(|argument| argument == "--session-id")
    );
    assert_eq!(
        prepared
            .environment
            .get("LYS_PTY_STATUS_NOTE")
            .map(String::as_str),
        Some("the earlier conversation left no transcript; started a new one")
    );
    prepared = before.clone();
    let metadata = json!({"sessionId": UUID, "cwd": cwd_text});
    std::fs::write(transcript, format!("{metadata}\n"))?;
    prepare(&mut prepared)?;
    assert_eq!(prepared, before);
    Ok(())
}

#[test]
fn generic_non_operation_runner_sessions_keep_their_terminal_arguments() -> TestResult {
    let mut prepared = launch();
    prepared.session = "generic-runner-fixture".to_owned();
    prepared.arguments = vec!["-c".to_owned(), "a generic shell fixture".to_owned()];
    let before = prepared.clone();
    prepare(&mut prepared)?;
    assert_eq!(prepared, before);
    Ok(())
}

#[test]
fn an_undeclared_harness_keeps_its_launch_and_fresh_codex_names_unproved_resume_without_guessing_an_id()
-> TestResult {
    for harness in [None, Some(Harness::Codex)] {
        let mut prepared = launch();
        prepared
            .config
            .as_mut()
            .ok_or("fixture config is absent")?
            .harness = harness;
        let before = prepared.clone();
        prepare(&mut prepared)?;
        if harness == Some(Harness::Codex) {
            assert!(
                prepared
                    .environment
                    .get("LYS_PTY_STATUS_NOTE")
                    .is_some_and(|note| note.starts_with("resume not proven for codex "))
            );
            prepared.environment.remove("LYS_PTY_STATUS_NOTE");
        }
        assert_eq!(prepared, before);
    }
    Ok(())
}

#[test]
fn invalid_operation_ids_and_fresh_continue_fork_or_no_persistence_flags_are_refused() {
    let mut prepared = launch();
    prepared.session = "op-not-hex".to_owned();
    let error = prepare(&mut prepared).unwrap_err();
    assert_eq!(error.name(), "pty_conversation_invalid", "{error}");
    for (argument, refusal) in [
        ("--continue", "pty_resume_arguments_invalid"),
        ("-c", "pty_resume_arguments_invalid"),
        ("--fork-session", "pty_resume_arguments_invalid"),
        ("--no-session-persistence", "pty_conversation_conflict"),
        ("--", "pty_conversation_conflict"),
    ] {
        let mut prepared = launch();
        prepared.arguments.push(argument.to_owned());
        let before = prepared.clone();
        let error = prepare(&mut prepared).unwrap_err();
        assert_eq!(error.name(), refusal, "{error}");
        assert_eq!(prepared, before);
    }
}

#[test]
fn a_known_conversation_without_a_transcript_starts_fresh_and_names_the_lost_history() -> TestResult
{
    let directory = tempfile::tempdir()?;
    let cwd = directory.path().canonicalize()?;
    let mut prepared = launch();
    prepared.directory = cwd.display().to_string();
    prepared.environment.insert(
        "CLAUDE_CONFIG_DIR".to_owned(),
        cwd.join("claude-config").display().to_string(),
    );
    prepared.arguments.extend([
        "--resume".to_owned(),
        "aaaaaaaa-bbbb-4ccc-addd-eeeeeeeeeeee".to_owned(),
    ]);
    prepare(&mut prepared)?;
    assert_eq!(
        prepared.arguments,
        ["--model", "chosen", "--session-id", UUID]
    );
    assert_eq!(
        prepared
            .environment
            .get("LYS_PTY_STATUS_NOTE")
            .map(String::as_str),
        Some("the earlier conversation left no transcript; started a new one")
    );
    Ok(())
}
