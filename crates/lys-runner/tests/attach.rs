#![cfg(test)]
//! AGENTS-002 R4: attach reads a managed session's frames as lines: a
//! person's turn renders as a `user` line, the assistant's text, each tool
//! call and its result, and the harness's status each as their own kind. A
//! session in a pseudo-terminal has no frames; its attach is a follow of its
//! terminal bytes, and asking it for lines is refused by name. Rendered
//! lines kept and followed on a live managed session are held in the
//! crate's own cases (`tests/attach/cases.rs`).

use std::collections::BTreeMap;
use std::error::Error;
use std::sync::Arc;

use lys_core::Ed25519Identity;
use lys_runner::attach::{ASSISTANT, STATUS, TOOL_CALL, TOOL_RESULT, USER, render};
use lys_runner::harness_control::Transport;
use lys_runner::{Act, Answer, Client, Launch, Options, Runner, RunnerError};
use serde_json::json;

type TestResult = Result<(), Box<dyn Error>>;

const CONVERSATION: &str = "aaaaaaaa-bbbb-4ccc-addd-eeeeeeeeeeee";

#[test]
fn a_claude_user_turn_renders_as_one_user_line_with_its_words() {
    let turn = json!({"type":"user","session_id":CONVERSATION,"uuid":"u-1",
        "parent_tool_use_id":null,"message":{"role":"user","content":"hello from the person"}});
    let lines = render(Transport::Claude, &turn, 7);
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].kind, USER);
    assert_eq!(lines[0].text, "hello from the person");
    assert_eq!(lines[0].at, 7);
}

#[test]
fn assistant_text_tool_calls_results_and_status_each_render_as_their_kind() {
    let said = json!({"type":"assistant","session_id":CONVERSATION,
        "message":{"content":[{"type":"thinking","thinking":"private"},
        {"type":"text","text":"reading it"},
        {"type":"tool_use","id":"t-1","name":"Read","input":{"file_path":"/a"}}]}});
    let lines = render(Transport::Claude, &said, 1);
    assert_eq!(
        lines
            .iter()
            .map(|line| line.kind.as_str())
            .collect::<Vec<_>>(),
        [ASSISTANT, TOOL_CALL]
    );
    assert_eq!(lines[0].text, "reading it");
    let result = json!({"type":"user","session_id":CONVERSATION,
        "message":{"role":"user","content":[{"type":"tool_result","tool_use_id":"t-1",
        "content":"the file's words","is_error":false}]}});
    let lines = render(Transport::Claude, &result, 2);
    assert_eq!(lines[0].kind, TOOL_RESULT);
    assert_eq!(lines[0].text, "the file's words");
    let ended = json!({"type":"result","subtype":"success","session_id":CONVERSATION,
        "is_error":false});
    let lines = render(Transport::Claude, &ended, 3);
    assert_eq!(lines[0].kind, STATUS);
    assert_eq!(lines[0].text, "turn ended: success");
    let partial = json!({"type":"stream_event","session_id":CONVERSATION,"event":{}});
    assert!(render(Transport::Claude, &partial, 4).is_empty());
}

#[test]
fn a_codex_user_message_renders_as_a_user_line() {
    let item = json!({"method":"item/completed","params":{"item":{"type":"userMessage",
        "id":"i-1","content":[{"type":"text","text":"hello codex"}]}}});
    let lines = render(Transport::Codex, &item, 5);
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].kind, USER);
    assert_eq!(lines[0].text, "hello codex");
}

#[test]
fn attach_read_on_a_pseudo_terminal_session_is_refused_by_name() -> TestResult {
    let dir = tempfile::tempdir()?;
    let key = Arc::new(Ed25519Identity::load_or_generate(
        &dir.path().join("server.key"),
    )?);
    let options = Options {
        socket: dir.path().join("runner.sock"),
        state: dir.path().join("state"),
        server_key: key.public_key_bytes(),
        scrollback: 1 << 16,
    };
    let serving = Runner::open(&options)?.spawn();
    let client = Client::new(options.socket, key);
    let directory = dir.path().to_str().ok_or("the test folder is not UTF-8")?;
    match client.ask(&Act::Start {
        lys_mcp: None,
        proxy: None,
        launch: Box::new(Launch {
            session: "terminal".to_owned(),
            program: "/bin/sh".to_owned(),
            arguments: vec!["-c".to_owned(), "cat".to_owned()],
            directory: directory.to_owned(),
            environment: BTreeMap::new(),
            config: None,
            columns: 80,
            rows: 24,
            rotation: None,
            policy: None,
        }),
    })? {
        Answer::Started { .. } => {}
        other => return Err(format!("start answered {other:?}").into()),
    }
    match client.ask(&Act::AttachRead {
        session: "terminal".to_owned(),
        cursor: None,
        follow: false,
    }) {
        Err(RunnerError::Refused { refusal, .. }) => {
            assert_eq!(refusal, "attach_pty_use_read_bytes");
        }
        other => return Err(format!("attach_read on a terminal answered {other:?}").into()),
    }
    match client.ask(&Act::AttachRead {
        session: "never-started".to_owned(),
        cursor: None,
        follow: false,
    }) {
        Err(RunnerError::Refused { refusal, .. }) => assert_eq!(refusal, "session_unknown"),
        other => return Err(format!("attach_read on no session answered {other:?}").into()),
    }
    serving.stop()?;
    Ok(())
}
