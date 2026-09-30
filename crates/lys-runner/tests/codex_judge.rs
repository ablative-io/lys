#![cfg(test)]
//! Native Codex hook fixtures retain identity boundaries and never grant access.

use std::error::Error;
use std::path::Path;

use lys_runner::codex_judge::{request, response};
use lys_runner::judge::{Asked, Authority, Judgement, Policy, Rule, RuleKind, judge};
use lys_runner::peer::PeerAct;
use lys_runner::refusals::{JudgeAsk, Verdict};
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

fn native(tool: &str) -> Value {
    json!({
        "session_id": "thread-a", "turn_id": "turn-a", "cwd": "/work",
        "transcript_path": null, "hook_event_name": "PreToolUse",
        "model": "fixture", "permission_mode": "never",
        "tool_name": tool, "tool_input": {"command": "private command"},
        "tool_use_id": "call-a"
    })
}

fn asked(value: &Value) -> Result<JudgeAsk, Box<dyn Error>> {
    match request(&value.to_string())?.peer {
        PeerAct::Judge(asked) => Ok(asked),
        PeerAct::Collect(_) => Err("hook became a collector request".into()),
        PeerAct::Restart { .. } => Err("hook became a restart request".into()),
    }
}

#[test]
fn shell_is_already_bash_and_keeps_only_a_claimed_identity() -> TestResult {
    let native = native("Bash");
    let call = asked(&native)?;
    assert_eq!(call.tool_name, "Bash");
    assert_eq!(call.tool_input, native["tool_input"]);
    assert_eq!(call.claimed_session.as_deref(), Some("thread-a"));
    assert!(!call.subagent);
    let wire = serde_json::to_value(request(&native.to_string())?)?;
    assert!(wire["peer"].get("cwd").is_none());
    assert!(wire["peer"].get("agent").is_none());
    Ok(())
}

#[test]
fn patch_does_not_masquerade_as_a_single_file_edit() -> TestResult {
    let call = asked(&native("apply_patch"))?;
    let policy = Policy {
        agent: "bound-agent".to_owned(),
        version: 3,
        rules: vec![Rule {
            id: "private-path".to_owned(),
            tool: "Write".to_owned(),
            kind: RuleKind::PathPrefix,
            target: Some("/private".to_owned()),
            authority: Authority::Hard,
        }],
    }
    .checked()
    .map_err(|refused| format!("{}: {}", refused.refusal, refused.words))?;
    let result = judge(
        &policy,
        &Asked {
            tool: &call.tool_name,
            input: &call.tool_input,
            cwd: Path::new("/work"),
            subagent: call.subagent,
        },
    );
    assert!(matches!(
        result,
        Judgement::Deny {
            refusal: "policy_uninspectable",
            ..
        }
    ));
    Ok(())
}

#[test]
fn attempts_include_turn_and_thread_without_delimiter_collisions() -> TestResult {
    let mut first = native("Bash");
    first["session_id"] = json!("a\nb");
    first["turn_id"] = json!("c");
    let mut second = first.clone();
    second["session_id"] = json!("a");
    second["turn_id"] = json!("b\nc");
    assert_ne!(asked(&first)?.attempt, asked(&second)?.attempt);
    second = first.clone();
    second["turn_id"] = json!("another-turn");
    assert_ne!(asked(&first)?.attempt, asked(&second)?.attempt);
    assert_eq!(asked(&first)?.attempt, asked(&first)?.attempt);
    Ok(())
}

#[test]
fn either_native_subagent_field_marks_the_call_unproved() -> TestResult {
    for field in ["agent_id", "agent_type"] {
        let mut hook = native("Bash");
        hook[field] = json!("worker");
        assert!(asked(&hook)?.subagent);
    }
    Ok(())
}

#[test]
fn unsupported_payloads_refuse_without_leaking_contents() {
    let mut cases = Vec::new();
    for tool in ["exec_command", "shell_command", "write_stdin"] {
        cases.push(native(tool));
    }
    for (field, value) in [
        ("hook_event_name", json!("agent-turn-complete")),
        ("tool_use_id", json!("")),
        ("cwd", json!("relative")),
        ("tool_input", json!({"cmd": "secret-marker"})),
        ("injected", json!("secret-marker")),
    ] {
        let mut hook = native("Bash");
        hook[field] = value;
        cases.push(hook);
    }
    assert_eq!(cases.len(), 8);
    for hook in cases {
        let result = request(&hook.to_string());
        assert!(result.is_err());
        if let Err(error) = result {
            let text = error.to_string();
            assert!(text.starts_with("codex_hook_malformed:"));
            assert!(!text.contains("secret-marker"));
            assert!(!text.contains("private command"));
        }
    }
}

#[test]
fn pass_is_not_an_allow_override_and_denial_has_native_shape() {
    assert_eq!(response(&Verdict::pass(Some(3))), json!({}));
    let verdict = Verdict::denied(
        "policy_denied",
        "Rule private-path denies this act".to_owned(),
        "recorded",
    );
    assert_eq!(
        response(&verdict),
        json!({"hookSpecificOutput": {
            "hookEventName": "PreToolUse", "permissionDecision": "deny",
            "permissionDecisionReason": "Rule private-path denies this act"
        }})
    );
}

#[test]
fn incomplete_audit_still_denies_with_a_nonempty_reason() {
    let verdict = Verdict::denied("audit_failed", String::new(), "incomplete");
    let answer = response(&verdict);
    assert_eq!(answer["hookSpecificOutput"]["permissionDecision"], "deny");
    assert!(
        answer["hookSpecificOutput"]["permissionDecisionReason"]
            .as_str()
            .is_some_and(|reason| !reason.is_empty())
    );
}
