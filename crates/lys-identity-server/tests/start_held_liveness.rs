#![cfg(test)]
//! The runner's bound conversation suffices without a matching launch record.

#[path = "support/runner_start.rs"]
pub mod support;

use std::error::Error;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixListener;

use lys_runner::protocol::{Greeting, reply_line, verify_request};
use lys_runner::{Act, Answer};
use serde_json::json;
use support::{Table, operation};

#[tokio::test(flavor = "multi_thread")]
async fn start_held_conversation_from_runner_liveness_is_recorded() -> Result<(), Box<dyn Error>> {
    let table = Table::unprofiled().await?;
    let conversation = "019c6e27-e55b-73d1-87d8-4e01f1f75043";
    let profile = operation()?;
    let path = format!("/agents/{}/provisioning", table.agent());
    table
        .ok(
            &path,
            &json!({
                "operation": profile, "from_version": 0, "model_access": ["claude-fable-5-1"],
                "tools": [], "skills": [], "mcp_servers": [], "instructions": "", "note": "",
                "harness": table.harness(), "permissions": {"default_mode": "plan"},
                "working_folder": table.dir.path(), "session": {"stall_after":300,"resume": conversation}
            }),
        )
        .await?;
    table
        .ok(
            &format!("{path}/1/review"),
            &json!({"operation": operation()?}),
        )
        .await?;
    let socket = table.dir.path().join("held.sock");
    let listener = UnixListener::bind(&socket)?;
    let machine = table
        .machine(
            &json!({
                "operation": operation()?, "name": "Existing writer", "kind": "laptop",
                "runtime": "sh", "slots": 1, "may_run": [table.agent()], "may_reach": []
            }),
            Some(json!({"kind": "socket", "path": socket})),
        )
        .await?;
    let session = operation()?;
    table
        .ok(
            &format!(
                "/agents/{}/runtime/sessions/{session}/reports",
                table.agent()
            ),
            &json!({
                "operation": operation()?, "state": "starting", "machine": machine,
                "what": "an existing writer", "confirmation": ""
            }),
        )
        .await?;
    let key = table.server_key.public_key_bytes();
    let held_session = session.clone();
    let responder = std::thread::spawn(move || -> Result<(), String> {
        serve_held(&listener, key, &held_session, conversation)
    });
    let started = table
        .service
        .post(
            &format!("/agents/{}/start", table.agent()),
            Some(&table.ada),
            &json!({"profile_version": profile, "machine": machine}),
        )
        .await;
    match responder.join() {
        Ok(answer) => answer?,
        Err(payload) => std::panic::resume_unwind(payload),
    }
    let (status, answer) = started?;
    assert_eq!(status, 409, "{answer}");
    assert_eq!(answer["error"], "conversation_held", "{answer}");
    assert!(answer["words"].to_string().contains(&session), "{answer}");
    let index = answer["start_record"]["index"]
        .as_u64()
        .ok_or("missing start receipt")?;
    let (status, recorded) = table
        .service
        .get(&format!("/runner-receipts/{index}"), Some(&table.ada))
        .await?;
    assert_eq!(status, 200, "{recorded}");
    assert_eq!(recorded, answer["start_record"]);
    table.close()
}

fn serve_held(
    listener: &UnixListener,
    key: [u8; 32],
    held_session: &str,
    conversation: &str,
) -> Result<(), String> {
    let mut observed = 0;
    while observed < 1 {
        let (stream, _) = listener.accept().map_err(|error| error.to_string())?;
        let greeting = Greeting::fresh("00");
        let mut writer = &stream;
        writeln!(writer, "{}", greeting.line()).map_err(|error| error.to_string())?;
        let mut line = String::new();
        BufReader::new(&stream)
            .read_line(&mut line)
            .map_err(|error| error.to_string())?;
        let act = verify_request(&line, &key, &greeting).map_err(|error| error.to_string())?;
        let answer = match act {
            Act::Feed { .. } => Answer::Refused {
                refusal: "feed_not_kept".to_owned(),
                words: "this fixture keeps no feed".to_owned(),
                oldest: None,
            },
            Act::Liveness { session: None } => Answer::Liveness {
                sessions: Vec::new(),
            },
            Act::GrantChannel => Answer::GrantChannel,
            Act::Owned => Answer::Owned {
                owners: Vec::new(),
                unreachable: Vec::new(),
            },
            Act::Liveness {
                session: Some(session),
            } if session == held_session => {
                observed += 1;
                Answer::Liveness {
                    sessions: vec![lys_runner::liveness::LivenessView {
                        session,
                        pid: Some(4242),
                        alive: true,
                        harness_session: Some(conversation.to_owned()),
                        turn_active: false,
                        last_signal_at: None,
                        started_at: 1,
                        ended: false,
                        harness: lys_runner::liveness::HarnessLiveness {
                            managed: true,
                            started_fresh: false,
                        },
                    }],
                }
            }
            other => return Err(format!("unexpected runner request {other:?}")),
        };
        writeln!(writer, "{}", reply_line(answer)).map_err(|error| error.to_string())?;
    }
    if observed != 1 {
        return Err(format!("liveness observed {observed} times"));
    }
    Ok(())
}
