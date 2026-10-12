#![cfg(test)]
//! A runner that retains the harness past the end budget prevents another start.

#[path = "support/runner_start.rs"]
pub mod support;

use std::error::Error;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixListener;
use std::sync::mpsc;

use lys_runner::protocol::{Greeting, reply_line, verify_request};
use lys_runner::{Act, Answer};
use serde_json::json;
use support::{Table, operation};

#[tokio::test(flavor = "multi_thread")]
async fn stop_end_budget_records_lingering_pid_and_refuses_start() -> Result<(), Box<dyn Error>> {
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
    let (release, released) = mpsc::channel();
    let responder = std::thread::spawn(move || -> Result<(), String> {
        let mut pending = None;
        let mut released = Some(released);
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
                            managed: false,
                            harness_session: None,
                            turn_active: false,
                            last_signal_at: None,
                            started_at: 1,
                            ended: false,
                        }],
                    }
                }
                Act::End { session } if session == serving_session => {
                    ends += 1;
                    let released = released.take().ok_or("a second End was sent")?;
                    pending = Some(std::thread::spawn(move || -> Result<(), String> {
                        released.recv().map_err(|error| error.to_string())?;
                        drop(stream);
                        Ok(())
                    }));
                    continue;
                }
                other => return Err(format!("unexpected runner request {other:?}")),
            };
            writeln!(writer, "{}", reply_line(answer)).map_err(|error| error.to_string())?;
        }
        pending
            .ok_or("no End was held")?
            .join()
            .map_err(|_| "held End panicked")??;
        if liveness != 2 || ends != 1 {
            return Err(format!("liveness={liveness}, ends={ends}"));
        }
        Ok(())
    });
    let stop = table
        .service
        .post(
            &format!("/agents/{}/stop", table.agent()),
            Some(&table.ada),
            &json!({"operation": operation()?, "reason": "prove the end budget"}),
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
            .contains("process_lingering"),
        "{}",
        stop.1
    );
    assert!(
        stop.1["sessions_refused"].to_string().contains("pid 4242"),
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
            .contains("pid 4242"),
        "{stops}"
    );
    table.ok(&format!("/identities/{}/transitions", table.agent()),
        &json!({"operation": operation()?, "transition": "activate", "reason": "try to restart"})).await?;
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
    release.send(())?;
    responder
        .join()
        .map_err(|_| "runner responder panicked")??;
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
