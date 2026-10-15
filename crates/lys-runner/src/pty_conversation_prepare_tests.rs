#![cfg(test)]
//! Common terminal preparation binds fresh operations without replacing resume.

use std::collections::BTreeMap;
use std::error::Error;

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
fn generic_non_operation_runner_sessions_keep_their_terminal_arguments() -> TestResult {
    let mut prepared = launch();
    prepared.session = "generic-runner-fixture".to_owned();
    prepared
        .config
        .as_mut()
        .ok_or("fixture config absent")?
        .harness = None;
    prepared.arguments = vec!["-c".to_owned(), "a generic shell fixture".to_owned()];
    let before = prepared.clone();
    prepare(&mut prepared)?;
    assert_eq!(prepared, before);
    Ok(())
}

#[test]
fn an_undeclared_harness_and_a_fresh_codex_keep_their_launch_unchanged() -> TestResult {
    for harness in [None, Some(Harness::Codex)] {
        let mut prepared = launch();
        prepared
            .config
            .as_mut()
            .ok_or("fixture config absent")?
            .harness = harness;
        let before = prepared.clone();
        prepare(&mut prepared)?;
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
fn harness_resumes_without_runner_visible_transcripts_keep_every_argument() -> TestResult {
    for harness in [Harness::ClaudeCode, Harness::Codex] {
        let mut prepared = launch();
        prepared
            .config
            .as_mut()
            .ok_or("fixture config absent")?
            .harness = Some(harness);
        prepared.arguments = if harness == Harness::Codex {
            vec![
                "--model".into(),
                "chosen".into(),
                "resume".into(),
                UUID.into(),
            ]
        } else {
            vec![
                "--model".into(),
                "chosen".into(),
                "--resume".into(),
                UUID.into(),
            ]
        };
        prepared
            .environment
            .insert("HOME".into(), "/runner-cannot-see-the-seat-home".into());
        let before = prepared.clone();
        prepare(&mut prepared)?;
        assert_eq!(prepared, before, "the harness resume was replaced");
    }
    Ok(())
}

#[test]
fn a_codex_thread_from_another_folder_keeps_its_resume_arguments() -> TestResult {
    let root = tempfile::tempdir()?;
    let mut prepared = launch();
    prepared
        .config
        .as_mut()
        .ok_or("fixture config absent")?
        .harness = Some(Harness::Codex);
    prepared.directory = root.path().display().to_string();
    prepared.arguments = vec!["resume".into(), UUID.into()];
    prepared
        .environment
        .insert("CODEX_HOME".into(), root.path().display().to_string());
    std::fs::create_dir(root.path().join("sessions"))?;
    std::fs::write(
        root.path()
            .join("sessions")
            .join(format!("rollout-{UUID}.jsonl")),
        serde_json::to_vec(
            &serde_json::json!({"type":"session_meta", "payload":{"id":UUID,"cwd":"/another-folder"}}),
        )?,
    )?;
    let before = prepared.clone();
    prepare(&mut prepared)?;
    assert_eq!(prepared, before);
    Ok(())
}

#[test]
fn fresh_codex_never_sets_a_synthetic_resume_status_note() -> TestResult {
    let mut prepared = launch();
    prepared
        .config
        .as_mut()
        .ok_or("fixture config absent")?
        .harness = Some(Harness::Codex);
    let before = prepared.clone();
    prepare(&mut prepared)?;
    assert_eq!(
        prepared, before,
        "a synthetic status note changed the launch"
    );
    Ok(())
}

#[test]
fn codex_resume_never_substitutes_a_fresh_launch_when_history_is_absent() -> TestResult {
    let root = tempfile::tempdir()?;
    let mut prepared = launch();
    prepared
        .config
        .as_mut()
        .ok_or("fixture config absent")?
        .harness = Some(Harness::Codex);
    prepared.directory = root.path().display().to_string();
    prepared
        .environment
        .insert("CODEX_HOME".into(), root.path().display().to_string());
    prepared.arguments = vec!["resume".into(), UUID.into()];
    let before = prepared.clone();
    prepare(&mut prepared)?;
    assert_eq!(
        prepared, before,
        "Codex resume was silently replaced by a fresh conversation"
    );
    Ok(())
}
