//! A restart requires confirmed exit evidence.

use super::*;

#[tokio::test(flavor = "multi_thread")]
async fn an_end_without_exit_evidence_does_not_start_another_session() -> TestResult {
    use lys_runner::protocol::{Greeting, reply_line, verify_request};
    use std::io::{BufRead, BufReader, Write};
    use std::os::unix::net::UnixListener;
    let held = Held::open().await?;
    held.review(1).await?;
    let started = held.start().await?;
    let session = started["session"].as_str().ok_or("no session")?.to_owned();
    let socket = held.dir.path().join("other.sock");
    let listener = UnixListener::bind(&socket)?;
    let reply_session = session.clone();
    let server = held.key.public_key_bytes();
    let answering = std::thread::spawn(move || -> std::io::Result<()> {
        loop {
            let (stream, _) = listener.accept()?;
            let mut writer = &stream;
            let greeting = Greeting::fresh("00000000000000000000000000000000");
            writer.write_all(greeting.line().as_bytes())?;
            writer.write_all(b"\n")?;
            let mut line = String::new();
            if BufReader::new(&stream).read_line(&mut line)? == 0 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "runner stand-in received no end request",
                ));
            }
            let act = verify_request(&line, &server, &greeting).map_err(|error| {
                std::io::Error::new(std::io::ErrorKind::InvalidData, error.to_string())
            })?;
            let ended = matches!(&act, Act::End { session } if session == &reply_session);
            let answer = match act {
                Act::End { session } if session == reply_session => Answer::Delivered { session },
                Act::Owned => Answer::Owned {
                    owners: Vec::new(),
                    unreachable: Vec::new(),
                },
                Act::Liveness { .. } => Answer::Liveness {
                    sessions: Vec::new(),
                },
                // Restart reads the session's control before it ends it.
                Act::ControlStatus { session } => Answer::ControlStatus {
                    session,
                    control: None,
                },
                Act::Feed { .. } => Answer::Refused {
                    refusal: "feed_not_kept".to_owned(),
                    words: "this fixture keeps no feed".to_owned(),
                    oldest: None,
                },
                other => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("unexpected runner request: {other:?}"),
                    ));
                }
            };
            let answer = reply_line(answer);
            writer.write_all(answer.as_bytes())?;
            writer.write_all(b"\n")?;
            if ended {
                return Ok(());
            }
        }
    });
    held.ok(
        &format!("/network/machines/{}/runner", held.machine),
        &json!({ "runner": { "kind": "socket", "path": socket.display().to_string() } }),
    )
    .await?;
    let (status, refused) = held
        .service
        .post(
            &format!("/agents/{}/restart", held.agent()),
            Some(&held.ada),
            &json!({ "session": session, "operation": operation()? }),
        )
        .await?;
    answering.join().expect("runner stand-in panicked")?;
    assert_eq!(status, 503, "{refused}");
    assert_eq!(refused["refusal"], "runner_reply_malformed", "{refused}");
    assert!(
        refused["reason"]
            .as_str()
            .ok_or("no refusal reason")?
            .contains("the end answered without the session's exit"),
        "{refused}"
    );
    assert!(held.status(&session)?.ended.is_none());
    held.close()
}

#[tokio::test(flavor = "multi_thread")]
async fn a_machine_without_a_runner_refuses_restart_before_ending() -> TestResult {
    let held = Held::open().await?;
    held.review(1).await?;
    let started = held.start().await?;
    let session = started["session"].as_str().ok_or("no session")?;
    held.ok(
        &format!("/network/machines/{}/runner", held.machine),
        &json!({ "runner": null }),
    )
    .await?;
    let (status, refused) = held
        .service
        .post(
            &format!("/agents/{}/restart", held.agent()),
            Some(&held.ada),
            &json!({ "session": session, "operation": operation()? }),
        )
        .await?;
    assert_ne!(status, 200);
    assert_eq!(refused["refusal"], "runner_absent", "{refused}");
    assert!(held.status(session)?.ended.is_none());
    held.close()
}

#[tokio::test(flavor = "multi_thread")]
async fn a_kept_start_naming_no_directory_restarts_into_the_profiles_folder() -> TestResult {
    let held = Held::open().await?;
    held.review(1).await?;
    // A session kept from before starts named their folder: reported and
    // started by hand, so its kept start carries no directory at all.
    let session = operation()?;
    held.ok(
        &format!("/agents/{}/runtime/sessions/{session}/reports", held.agent()),
        &json!({ "operation": operation()?, "machine": held.machine, "state": "starting", "what": "process requested" }),
    )
    .await?;
    held.client().ask(&Act::Start {
        tracking: None,
        lys_mcp: None,
        proxy: None,
        launch: Box::new(Launch {
            session: session.clone(),
            program: "/bin/cat".to_owned(),
            arguments: Vec::new(),
            directory: "/".to_owned(),
            environment: BTreeMap::new(),
            config: None,
            columns: 80,
            rows: 24,
            rotation: None,
            policy: None,
        }),
    })?;
    held.ok(&format!("/agents/{}/runtime/sessions/{session}/reports", held.agent()), &json!({ "operation": operation()?, "machine": held.machine, "state": "running", "what": "process started" })).await?;
    let started = held
        .ok(
            &format!("/agents/{}/restart", held.agent()),
            &json!({ "session": session, "operation": operation()? }),
        )
        .await?;
    assert_eq!(
        started["directory"], "/tmp",
        "a kept start naming no folder restarts into the profile's working_folder: {started}"
    );
    assert!(
        held.status(&session)?.ended.is_some(),
        "old process still running"
    );
    held.close()
}
