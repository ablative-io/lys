//! Fresh terminal starts bind Claude's native id without changing the reviewed configuration.

use lys_runner::Launch;
use serde_json::json;

use super::{fresh_terminal_conversation, remember_native_in};

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

const OPERATION: &str = "op-00112233445566778899aabbccddeeff";
const UUID: &str = "00112233-4455-6677-8899-aabbccddeeff";
const OTHER_OPERATION: &str = "op-fedcba9876543210fedcba9876543210";
const OTHER_UUID: &str = "fedcba98-7654-3210-fedc-ba9876543210";
const SELECTED_UUID: &str = "aaaaaaaa-bbbb-4ccc-addd-eeeeeeeeeeee";

fn launch(harness: &str, requires_controls: bool) -> Result<Launch> {
    Ok(serde_json::from_value(json!({
        "session": OPERATION,
        "program": "/chosen/harness",
        "arguments": [
            "--settings", "/chosen/settings.json",
            "--mcp-config", "/chosen/mcp.json",
            "--model", "chosen-model"
        ],
        "directory": "/project",
        "environment": {"CONFIG_ROOT": "/chosen"},
        "columns": 100,
        "rows": 40,
        "config": {
            "files": [],
            "argument_files": {},
            "environment_paths": {},
            "working_directory": false,
            "harness": harness,
            "requires_controls": requires_controls
        }
    }))?)
}

#[test]
fn a_first_fresh_claude_terminal_keeps_configuration_arguments_and_binds_the_complete_operation_uuid()
-> Result {
    let mut launch = launch("claude_code", false)?;
    let mut expected = launch.clone();
    expected.environment.insert(
        "LYS_PTY_STATUS_NOTE".to_owned(),
        "no earlier conversation to resume; started a new one".to_owned(),
    );
    expected.arguments = [
        "--settings",
        "/chosen/settings.json",
        "--mcp-config",
        "/chosen/mcp.json",
        "--model",
        "chosen-model",
        "--session-id",
        UUID,
    ]
    .map(str::to_owned)
    .to_vec();
    fresh_terminal_conversation(&mut launch, None)?;
    assert_eq!(launch, expected);
    let native = launch
        .arguments
        .last()
        .ok_or("the fresh launch has no native id")?;
    assert_eq!(
        native.split('-').map(str::len).collect::<Vec<_>>(),
        [8, 4, 4, 4, 12]
    );
    Ok(())
}

#[test]
fn a_new_start_operation_selects_a_different_fresh_native_conversation() -> Result {
    let mut first = launch("claude_code", false)?;
    let mut second = first.clone();
    second.session = OTHER_OPERATION.to_owned();
    fresh_terminal_conversation(&mut first, None)?;
    fresh_terminal_conversation(&mut second, None)?;
    assert_eq!(first.arguments.last().map(String::as_str), Some(UUID));
    assert_eq!(
        second.arguments.last().map(String::as_str),
        Some(OTHER_UUID)
    );
    assert_ne!(first.arguments.last(), second.arguments.last());
    Ok(())
}

#[test]
fn a_selected_resume_keeps_the_launch_whole_without_adding_a_fresh_session_id() -> Result {
    let mut launch = launch("claude_code", false)?;
    launch
        .arguments
        .extend(["--resume".to_owned(), SELECTED_UUID.to_owned()]);
    let before = launch.clone();
    fresh_terminal_conversation(&mut launch, Some(SELECTED_UUID))?;
    assert_eq!(launch, before);
    assert!(
        !launch
            .arguments
            .iter()
            .any(|argument| argument == "--session-id")
    );
    Ok(())
}

#[test]
fn a_managed_claude_launch_keeps_its_existing_conversation_selection_boundary() -> Result {
    let mut launch = launch("claude_code", true)?;
    let before = launch.clone();
    fresh_terminal_conversation(&mut launch, None)?;
    assert_eq!(launch, before);
    Ok(())
}

#[test]
fn a_fresh_codex_terminal_keeps_its_launch_without_guessing_a_native_thread_id() -> Result {
    let mut launch = launch("codex", false)?;
    let mut before = launch.clone();
    before.environment.insert(
        "LYS_PTY_STATUS_NOTE".to_owned(),
        "no earlier conversation to resume; started a new one".to_owned(),
    );
    fresh_terminal_conversation(&mut launch, None)?;
    assert_eq!(launch, before);
    Ok(())
}

#[test]
fn a_template_resume_conflicts_with_a_fresh_start_and_is_refused_without_mutation() -> Result {
    for argument in ["--resume", "--resume=another-id", "-r"] {
        let mut launch = launch("claude_code", false)?;
        launch.arguments.push(argument.to_owned());
        let before = launch.clone();
        let error = fresh_terminal_conversation(&mut launch, None)
            .err()
            .ok_or("a template resume silently replaced the fresh conversation")?;
        assert_eq!(error.name(), "pty_conversation_conflict");
        assert_eq!(launch, before);
    }
    Ok(())
}

#[test]
fn a_malformed_fresh_operation_refuses_without_a_default_native_identity_or_launch_mutation()
-> Result {
    for operation in ["", UUID, "op-123", "op-gggggggggggggggggggggggggggggggg"] {
        let mut launch = launch("claude_code", false)?;
        launch.session = operation.to_owned();
        let before = launch.clone();
        let error = fresh_terminal_conversation(&mut launch, None)
            .err()
            .ok_or("a malformed operation acquired a fresh native identity")?;
        assert_eq!(error.name(), "pty_conversation_invalid");
        assert_eq!(launch, before);
    }
    Ok(())
}

#[test]
fn an_observed_codex_thread_is_durable_idempotent_and_preserves_the_original_admission() -> Result {
    use crate::runtime_state::{Report, Reported};
    use crate::runtime_store::RuntimeStore;
    use lys_core::Ed25519Identity;
    use std::sync::Arc;

    let root = tempfile::tempdir()?;
    let key = Arc::new(Ed25519Identity::load_or_generate(&root.path().join("key"))?);
    let directory = root.path().join("runtime");
    let mut store = RuntimeStore::open(&directory, Arc::clone(&key))?;
    let admitted = json!({"session": OPERATION, "harness": "codex", "directory": "/project", "conversation": null});
    let mut report = Report {
        operation: OPERATION.to_owned(),
        session: OPERATION.to_owned(),
        agent: Some("agent".to_owned()),
        machine: "machine".to_owned(),
        state: Reported::Starting,
        what: "start admitted".to_owned(),
        confirmation: String::new(),
        reported_by: "person".to_owned(),
        at: 1,
        launch: Some(admitted.clone()),
    };
    store.report(report.clone())?;
    report.operation = OTHER_OPERATION.to_owned();
    report.state = Reported::Running;
    report.reported_by = "the runner of machine machine".to_owned();
    report.what = "the runner started process 42".to_owned();
    report.launch = None;
    store.report(report.clone())?;
    remember_native_in(&mut store, OPERATION, UUID)?;
    remember_native_in(&mut store, OPERATION, UUID)?;
    assert_eq!(
        store
            .session(OPERATION)
            .ok_or("session missing")?
            .reports
            .len(),
        3
    );
    report.operation = lys_identity::OperationId::generate()?.to_string();
    report.state = Reported::Stopped;
    report.what = "the process exited".to_owned();
    report.confirmation = "exit 0".to_owned();
    store.report(report)?;
    drop(store);
    let reopened = RuntimeStore::open(&directory, key)?;
    let tracked = reopened
        .session(OPERATION)
        .ok_or("reopened session missing")?;
    assert!(tracked.stopped());
    assert_eq!(
        tracked.first().and_then(|first| first.launch.as_ref()),
        Some(&admitted)
    );
    assert_eq!(
        tracked.reports[2]
            .launch
            .as_ref()
            .and_then(|view| view["native_conversation"].as_str()),
        Some(UUID)
    );
    Ok(())
}
