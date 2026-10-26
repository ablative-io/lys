#![cfg(test)]
//! A stop whose runner closes before the process exits is named on the stop and
//! in the stops record, and the lingering process refuses another start
//! (HOME-040 R3: no end budget decides it).

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
async fn a_runner_closing_on_the_end_is_named_and_the_lingering_pid_refuses_start()
-> Result<(), Box<dyn Error>> {
    let table = Table::set().await?;
    let socket = table.dir.path().join("lingering.sock");
    let listener = UnixListener::bind(&socket)?;
    let machine = table
        .machine(
            &json!({
                "operation": operation()?, "name": "Lingering harness", "kind": "laptop",
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
                "what": "launched", "confirmation": ""
            }),
        )
        .await?;
    let key = table.server_key.public_key_bytes();
    let serving_session = session.clone();
    let responder = respond(listener, key, serving_session);
    let stop = table
        .service
        .post(
            &format!("/agents/{}/stop", table.agent()),
            Some(&table.ada),
            &json!({"operation": operation()?, "reason": "prove the runner close"}),
        )
        .await?;
    assert_eq!(stop.0, 200, "{}", stop.1);
    assert_eq!(
        stop.1["sessions_confirmed"].as_array().map(Vec::len),
        Some(0)
    );
    assert!(
        stop.1["sessions_refused"]
            .to_string()
            .contains("runner_unreachable"),
        "{}",
        stop.1
    );
    let (status, stops) = table
        .service
        .get(
            &format!("/agents/{}/stops", table.agent()),
            Some(&table.ada),
        )
        .await?;
    assert_eq!(status, 200, "{stops}");
    assert!(
        stops["stops"][0]["sessions_refused"]
            .to_string()
            .contains("runner_unreachable"),
        "{stops}"
    );
    table.ok(&format!("/identities/{}/transitions", table.agent()),
        &json!({"operation": operation()?, "transition": "reinstate", "reason": "try to restart"})).await?;
    let (_, profile) = table
        .service
        .get(
            &format!("/agents/{}/provisioning", table.agent()),
            Some(&table.ada),
        )
        .await?;
    let started = table
        .service
        .post(
            &format!("/agents/{}/start", table.agent()),
            Some(&table.ada),
            &json!({"profile_version": profile["profile"]["operation"], "machine": machine}),
        )
        .await?;
    responder
        .join()
        .map_err(|error| format!("runner responder panicked: {error:?}"))??;
    table.close()?;
    assert_eq!(started.0, 409, "{}", started.1);
    assert_eq!(started.1["error"], "process_lingering", "{}", started.1);
    assert!(
        started.1["words"].to_string().contains("pid 4242"),
        "{}",
        started.1
    );
    Ok(())
}

fn respond(
    listener: UnixListener,
    key: [u8; 32],
    serving_session: String,
) -> std::thread::JoinHandle<Result<(), String>> {
    std::thread::spawn(move || -> Result<(), String> {
        let mut liveness = 0;
        let mut ends = 0;
        while liveness < 2 || ends < 1 {
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
                Act::ControlStatus { session } if session == serving_session => {
                    Answer::ControlStatus {
                        session,
                        control: None,
                    }
                }
                Act::Owned => Answer::Owned {
                    owners: Vec::new(),
                    unreachable: Vec::new(),
                },
                Act::Liveness {
                    session: Some(session),
                } if session == serving_session => {
                    liveness += 1;
                    Answer::Liveness {
                        sessions: vec![lys_runner::liveness::LivenessView {
                            session,
                            pid: Some(4242),
                            alive: true,
                            harness_session: None,
                            turn_active: false,
                            last_signal_at: None,
                            started_at: 1,
                            ended: false,
                            harness: lys_runner::liveness::HarnessLiveness {
                                managed: false,
                                started_fresh: false,
                            },
                        }],
                    }
                }
                // The runner closes with the process still held: no reap.
                Act::End { session } if session == serving_session => {
                    ends += 1;
                    drop(stream);
                    continue;
                }
                other => return Err(format!("unexpected runner request {other:?}")),
            };
            writeln!(writer, "{}", reply_line(answer)).map_err(|error| error.to_string())?;
        }
        if liveness != 2 || ends != 1 {
            return Err(format!("liveness={liveness}, ends={ends}"));
        }
        Ok(())
    })
}
