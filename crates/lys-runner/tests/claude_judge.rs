#![cfg(test)]
//! Claude Code's `PreToolUse` hook input reaches the shared peer judge, and
//! every failure is written as a deny.

use std::error::Error;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixListener;

use lys_runner::claude_judge::{output, request};
use lys_runner::peer::{PeerAct, PeerRequest};
use lys_runner::protocol::{Answer, Greeting, reply_line};
use lys_runner::refusals::Verdict;
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

fn hook() -> Value {
    json!({
        "session_id": "claim-only", "transcript_path": "/home/x/t.jsonl",
        "cwd": "/work", "permission_mode": "default",
        "hook_event_name": "PreToolUse", "tool_name": "Bash",
        "tool_input": {"command": "sensitive command"}, "tool_use_id": "toolu_1",
        "a_member_a_later_release_adds": true
    })
}

#[test]
fn the_hook_input_becomes_a_judge_ask_and_later_members_pass() -> TestResult {
    let PeerRequest { peer, .. } = request(&hook().to_string())?;
    let PeerAct::Judge(asked) = peer else {
        return Err("the hook asked something other than the judge".into());
    };
    assert_eq!(asked.tool_name, "Bash");
    assert_eq!(asked.claimed_session.as_deref(), Some("claim-only"));
    assert_eq!(
        asked.attempt.as_deref(),
        Some(r#"["claim-only","toolu_1"]"#)
    );
    assert!(!asked.subagent);
    let mut sub = hook();
    sub["agent_id"] = json!("agent-1");
    let PeerAct::Judge(asked) = request(&sub.to_string())?.peer else {
        return Err("the subagent's hook asked something other than the judge".into());
    };
    assert!(asked.subagent);
    Ok(())
}

#[test]
fn a_wrong_event_a_relative_cwd_or_a_missing_member_is_refused_without_its_input() {
    let mut wrong = hook();
    wrong["hook_event_name"] = json!("PostToolUse");
    let mut relative = hook();
    relative["cwd"] = json!("work");
    let mut missing = hook();
    missing
        .as_object_mut()
        .map(|object| object.remove("tool_use_id"));
    for input in [wrong, relative, missing] {
        let refused = request(&input.to_string())
            .err()
            .map(|error| error.to_string())
            .unwrap_or_default();
        assert!(refused.contains("claude_hook_malformed"), "{refused}");
        assert!(!refused.contains("sensitive command"));
    }
}

#[test]
fn a_deny_survives_a_real_peer_exchange() -> TestResult {
    let dir = tempfile::tempdir()?;
    let socket = dir.path().join("judge.sock");
    let listener = UnixListener::bind(&socket)?;
    let server = std::thread::spawn(move || -> Result<PeerRequest, String> {
        let serve = || -> Result<PeerRequest, Box<dyn Error>> {
            let (mut stream, _) = listener.accept()?;
            writeln!(stream, "{}", Greeting::fresh(&"ab".repeat(32)).line())?;
            let mut line = String::new();
            BufReader::new(stream.try_clone()?).read_line(&mut line)?;
            let request = serde_json::from_str(&line)?;
            let verdict = Verdict::denied(
                "policy_denied",
                "Rule shell denies this call".to_owned(),
                "recorded",
            );
            writeln!(stream, "{}", reply_line(Answer::Judged { verdict }))?;
            Ok(request)
        };
        serve().map_err(|error| error.to_string())
    });
    let answer = output(&socket, &hook().to_string());
    let sent = server
        .join()
        .map_err(|error| format!("fixture server panicked: {error:?}"))??;
    assert_eq!(answer["hookSpecificOutput"]["permissionDecision"], "deny");
    assert_eq!(
        answer["hookSpecificOutput"]["permissionDecisionReason"],
        "Rule shell denies this call"
    );
    assert!(matches!(sent.peer, PeerAct::Judge(_)));
    Ok(())
}

#[test]
fn a_missing_runner_or_unreadable_input_denies() -> TestResult {
    let dir = tempfile::tempdir()?;
    let socket = dir.path().join("absent.sock");
    let unreachable = output(&socket, &hook().to_string());
    assert_eq!(
        unreachable["hookSpecificOutput"]["permissionDecision"],
        "deny"
    );
    assert!(unreachable.to_string().contains("claude_judge_unavailable"));
    assert!(!unreachable.to_string().contains("sensitive command"));
    let malformed = output(&socket, "not-json-secret").to_string();
    assert!(malformed.contains("claude_hook_malformed"));
    assert!(!malformed.contains("not-json-secret"));
    Ok(())
}
