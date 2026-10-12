#![cfg(test)]
//! A stop cannot accept an exit answer without a reaped status or signal.

#[path = "support/runner_start.rs"]
pub mod support;

use lys_runner::protocol::{Greeting, reply_line, verify_request};
use lys_runner::{Act, Answer, Ended, EndedHow};
use serde_json::json;
use std::error::Error;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixListener;
use support::{Table, operation};

#[tokio::test(flavor = "multi_thread")]
async fn stop_refuses_an_exit_answer_without_reap_evidence() -> Result<(), Box<dyn Error>> {
    let table = Table::set().await?;
    let socket = table.dir.path().join("unproved.sock");
    let listener = UnixListener::bind(&socket)?;
    let machine = table.machine(&json!({
        "operation": operation()?, "name": "Unproved exit", "kind": "laptop", "runtime": "sh",
        "slots": 1, "may_run": [table.agent()], "may_reach": []
    }), Some(json!({"kind": "socket", "path": socket}))).await?;
    let session = operation()?;
    table
        .ok(
            &format!(
                "/agents/{}/runtime/sessions/{session}/reports",
                table.agent()
            ),
            &json!({
                "operation": operation()?, "state": "starting", "machine": machine,
                "what": "launched", "confirmation": ""
            }),
        )
        .await?;
    let key = table.server_key.public_key_bytes();
    let replying_session = session.clone();
    let answering = std::thread::spawn(move || -> Result<(), String> {
        let mut liveness = 0;
        let mut ends = 0;
        while liveness < 1 || ends < 1 {
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
                } if session == replying_session => {
                    liveness += 1;
                    Answer::Liveness {
                        sessions: vec![lys_runner::liveness::LivenessView {
                            session,
                            pid: Some(41),
                            alive: true,
                            managed: false,
                            harness_session: None,
                            turn_active: false,
                            last_signal_at: None,
                            started_at: 1,
                            ended: false,
                        }],
                    }
                }
                Act::End { session } if session == replying_session => {
                    ends += 1;
                    Answer::Ended {
                        session,
                        ended: Ended {
                            how: EndedHow::Exited,
                            at: 1,
                            status: None,
                            signal: None,
                            reason: Some("the process exit could not be read".to_owned()),
                            stopped: None,
                        },
                    }
                }
                other => return Err(format!("expected Owned, Liveness or End, got {other:?}")),
            };
            writeln!(writer, "{}", reply_line(answer)).map_err(|error| error.to_string())?;
        }
        if liveness != 1 || ends != 1 {
            return Err(format!("liveness={liveness}, ends={ends}"));
        }
        Ok(())
    });
    let sent = table
        .service
        .post(
            &format!("/agents/{}/stop", table.agent()),
            Some(&table.ada),
            &json!({"operation": operation()?, "reason": "require exit evidence"}),
        )
        .await;
    answering
        .join()
        .map_err(|panic| format!("runner fixture panicked: {panic:?}"))??;
    table.close()?;
    let (status, body) = sent?;
    assert_eq!(status, 200, "{body}");
    assert_eq!(
        body["sessions_confirmed"].as_array().map(Vec::len),
        Some(0),
        "{body}"
    );
    assert!(
        body["sessions_refused"]
            .to_string()
            .contains("process_exit_unconfirmed"),
        "{body}"
    );
    Ok(())
}
