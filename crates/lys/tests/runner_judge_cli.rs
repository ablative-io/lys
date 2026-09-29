#![cfg(test)]
//! `lys runner judge`, the `PreToolUse` hook command: with no runner to ask
//! it still exits zero and writes a deny the harness reads, for each
//! harness, without repeating the tool's input.

use std::error::Error;
use std::io::Write;
use std::process::{Command, Stdio};

use serde_json::{Value, json};

#[test]
fn with_no_runner_each_harness_is_answered_with_a_deny() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let hook = json!({
        "session_id": "s", "turn_id": "t", "tool_use_id": "u", "cwd": "/work",
        "transcript_path": null, "model": "m", "permission_mode": "default",
        "hook_event_name": "PreToolUse", "tool_name": "Bash",
        "tool_input": {"command": "sensitive command"}
    });
    for harness in ["claude", "codex"] {
        let mut child = Command::new(env!("CARGO_BIN_EXE_lys"))
            .args(["runner", "judge", "--harness", harness, "--socket"])
            .arg(dir.path().join("absent.sock"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;
        child
            .stdin
            .take()
            .ok_or("no stdin")?
            .write_all(hook.to_string().as_bytes())?;
        let done = child.wait_with_output()?;
        assert!(done.status.success(), "{harness}: {:?}", done.status);
        let answer: Value = serde_json::from_slice(&done.stdout)?;
        assert_eq!(
            answer["hookSpecificOutput"]["permissionDecision"], "deny",
            "{harness}"
        );
        assert!(!answer.to_string().contains("sensitive command"));
    }
    Ok(())
}
