#![cfg(test)]
//! A person's ordinary Stop ends the run and permits a fresh Start without a lifecycle transition.

#[path = "support/runner_lifetime.rs"]
mod runner_lifetime;
#[path = "support/runner_start.rs"]
pub mod support;
use runner_lifetime::Lifetime;
use std::sync::atomic::Ordering;

use std::error::Error;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixListener;

use identity_contract::fake_issuer::Login;
use lys_runner::protocol::{Ended, EndedHow, Greeting, reply_line, verify_request};
use lys_runner::{Act, Answer};
use serde_json::json;
use support::{Table, operation};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_person_stops_then_starts_without_reinstating_the_identity() -> Result<(), Box<dyn Error>>
{
    let table = Table::set().await?;
    let agent = table
        .agent_of(1)
        .ok_or("the second person's agent is absent")?;
    let cookie = table
        .service
        .sign_in(Login {
            subject: "bea-subject".to_owned(),
            email: "bea@example.test".to_owned(),
        })
        .await?;
    let profile = format!("/agents/{agent}/provisioning");
    table
        .ok(
            &profile,
            &json!({
                "operation": operation()?, "from_version": 0,
                "model_access": ["claude-fable-5-1"], "tools": [], "skills": [], "mcp_servers": [],
                "instructions": "", "note": "", "harness": table.harness(),
                "permissions": {"default_mode": "plan"}, "working_folder": table.dir.path()
            }),
        )
        .await?;
    let (status, reviewed) = table
        .service
        .post(
            &format!("{profile}/1/review"),
            Some(&cookie),
            &json!({"operation": operation()?}),
        )
        .await?;
    assert_eq!(status, 200, "{reviewed}");

    let socket = table.dir.path().join("session-stop.sock");
    let listener = UnixListener::bind(&socket)?;
    let key = table.server_key.public_key_bytes();
    let lifetime = Lifetime::new(&socket);
    let ended = lifetime.signal();
    let responding = std::thread::spawn(move || -> Result<(usize, usize), String> {
        serve_runner(&listener, key, &ended)
    });
    let machine = table
        .machine(
            &json!({
                "operation": operation()?, "name": "Person's computer", "kind": "laptop",
                "runtime": "fixture", "slots": 1, "may_run": [agent], "may_reach": []
            }),
            Some(json!({"kind": "socket", "path": socket})),
        )
        .await?;
    let session = operation()?;
    let (status, reported) = table.service.post(
        &format!("/agents/{agent}/runtime/sessions/{session}/reports"), Some(&cookie),
        &json!({"operation": operation()?, "state": "starting", "machine": machine, "what": "fixture run", "confirmation": ""})
    ).await?;
    assert_eq!(status, 200, "{reported}");
    let (status, stopped) = table
        .service
        .post(
            &format!("/runtime/sessions/{session}/end"),
            Some(&cookie),
            &json!({}),
        )
        .await?;
    assert_eq!(status, 200, "{stopped}");
    assert_eq!(stopped["answer"]["kind"], "ended");
    let (status, identity) = table
        .service
        .get(&format!("/agents/{agent}"), Some(&cookie))
        .await?;
    assert_eq!(status, 200, "{identity}");
    assert_eq!(identity["state"], "active");
    let (status, started) = table
        .service
        .post(
            &format!("/agents/{agent}/start-command"),
            Some(&cookie),
            &json!({"machine": machine, "operation": operation()?}),
        )
        .await?;
    assert_eq!(status, 200, "{started}");
    assert_eq!(started["runner"]["state"], "running");
    let (status, stops) = table
        .service
        .get(&format!("/agents/{agent}/stops"), Some(&cookie))
        .await?;
    assert_eq!(status, 200, "{stops}");
    assert_eq!(stops["stops"], json!([]));
    lifetime.finish()?;
    assert_eq!(
        responding
            .join()
            .map_err(|error| format!("fixture runner panicked: {error:?}"))??,
        (1, 1)
    );
    table.close()
}

fn serve_runner(
    listener: &UnixListener,
    key: [u8; 32],
    ended: &std::sync::atomic::AtomicBool,
) -> Result<(usize, usize), String> {
    let result = (|| -> Result<(usize, usize), Box<dyn Error>> {
        let (mut starts, mut ends) = (0, 0);
        let mut native: std::collections::BTreeMap<String, String> =
            std::collections::BTreeMap::new();
        let mut channels = Vec::new();
        loop {
            let (stream, _) = listener.accept()?;
            if ended.load(Ordering::Acquire) {
                break;
            }
            let greeting = Greeting::fresh(&"a".repeat(32));
            let mut writer = &stream;
            writeln!(writer, "{}", greeting.line())?;
            let mut line = zeroize::Zeroizing::new(String::new());
            BufReader::new(&stream).read_line(&mut line)?;
            let requested = verify_request(&line, &key, &greeting)?;
            let requested = match requested {
                Act::AsCaller { done, .. } => *done,
                other => other,
            };
            let answer = match requested {
                Act::Status { session } => {
                    runner_lifetime::status(native.keys(), session.as_deref())
                }
                Act::End { session } => {
                    ends += 1;
                    Answer::Ended {
                        session,
                        ended: Ended {
                            how: EndedHow::Exited,
                            at: 2,
                            status: Some(0),
                            signal: None,
                            reason: None,
                            stopped: None,
                        },
                    }
                }
                Act::Start { launch, .. } => {
                    native.insert(
                        launch.session.clone(),
                        launch
                            .arguments
                            .windows(2)
                            .find(|pair| pair[0] == "--session-id" || pair[0] == "--resume")
                            .map(|pair| pair[1].clone())
                            .ok_or("start has no native id")?,
                    );
                    starts += 1;
                    Answer::Started {
                        qualification: None,
                        session: launch.session,
                        pid: 42,
                        started_at: 3,
                    }
                }
                Act::Liveness { session } => runner_lifetime::liveness(&native, session.as_deref()),
                Act::Owned => Answer::Owned {
                    owners: Vec::new(),
                    unreachable: Vec::new(),
                },
                Act::ControlStatus { session } => Answer::ControlStatus {
                    session,
                    control: None,
                },
                Act::GrantChannel => {
                    writeln!(writer, "{}", reply_line(Answer::GrantChannel))?;
                    channels.push(stream);
                    continue;
                }
                Act::Feed { .. } => Answer::Refused {
                    refusal: "feed_not_kept".to_owned(),
                    words: "this fixture keeps no feed".to_owned(),
                    oldest: None,
                },
                other => return Err(format!("unexpected runner act: {other:?}").into()),
            };
            writeln!(writer, "{}", reply_line(answer))?;
        }
        Ok((starts, ends))
    })();
    let result = result.map_err(|error| error.to_string());
    if let Err(error) = &result {
        eprintln!("stop/start stand-in failed: {error}");
    }
    result
}
