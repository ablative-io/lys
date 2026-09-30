#![cfg(test)]
//! Real Unix-socket exchanges test native deny output at the runner boundary.

use std::error::Error;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixListener;

use lys_runner::codex_judge_client::{ask, output};
use lys_runner::peer::{PeerAct, PeerRequest};
use lys_runner::protocol::{Answer, Greeting, reply_line};
use lys_runner::refusals::Verdict;
use serde_json::json;

type TestResult = Result<(), Box<dyn Error>>;

fn stdin() -> String {
    json!({
        "session_id":"claim-only", "turn_id":"turn", "tool_use_id":"call",
        "cwd":"/work", "transcript_path":null, "model":"fixture",
        "permission_mode":"never", "hook_event_name":"PreToolUse",
        "tool_name":"Bash", "tool_input":{"command":"sensitive command"}
    })
    .to_string()
}

#[test]
fn native_deny_survives_a_real_peer_protocol_exchange() -> TestResult {
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
            writeln!(
                stream,
                "{}",
                reply_line(Answer::Judged {
                    verdict: Verdict::denied(
                        "policy_denied",
                        "Rule shell denies this call".to_owned(),
                        "recorded"
                    ),
                })
            )?;
            Ok(request)
        };
        serve().map_err(|error| error.to_string())
    });
    let answer = output(&socket, &stdin());
    let request = server
        .join()
        .map_err(|error| format!("fixture server panicked: {error:?}"))??;
    assert_eq!(answer["hookSpecificOutput"]["permissionDecision"], "deny");
    assert_eq!(
        answer["hookSpecificOutput"]["permissionDecisionReason"],
        "Rule shell denies this call"
    );
    match request.peer {
        PeerAct::Judge(asked) => {
            assert_eq!(asked.claimed_session.as_deref(), Some("claim-only"));
            assert_eq!(asked.tool_name, "Bash");
        }
        PeerAct::Collect(_) => return Err("judge sent a collector request".into()),
        PeerAct::Restart { .. } => return Err("judge sent a restart request".into()),
    }
    Ok(())
}

#[test]
fn missing_judge_denies_without_an_audit_claim() -> TestResult {
    let dir = tempfile::tempdir()?;
    let socket = dir.path().join("absent.sock");
    assert!(ask(&socket, &stdin()).is_err());
    let answer = output(&socket, &stdin());
    assert_eq!(answer["hookSpecificOutput"]["permissionDecision"], "deny");
    assert!(
        answer["hookSpecificOutput"]["permissionDecisionReason"]
            .as_str()
            .is_some_and(|reason| reason.contains("runner_unreachable"))
    );
    assert!(!answer.to_string().contains("sensitive command"));
    Ok(())
}

#[test]
fn malformed_input_denies_before_connecting() -> TestResult {
    let dir = tempfile::tempdir()?;
    let answer = output(&dir.path().join("absent.sock"), "not-json-secret");
    let text = answer.to_string();
    assert!(text.contains("codex_hook_malformed"));
    assert!(!text.contains("runner_unreachable"));
    assert!(!text.contains("not-json-secret"));
    Ok(())
}
