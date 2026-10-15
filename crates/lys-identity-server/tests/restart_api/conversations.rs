//! Restart selects the live native conversation before asking for its exit.

use super::*;
use lys_runner::liveness::LivenessView;
use lys_runner::protocol::{Greeting, StatusView, reply_line, verify_request};
use std::collections::BTreeSet;
use std::sync::Mutex;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

const NATIVE: &str = "aaaaaaaa-bbbb-4ccc-addd-eeeeeeeeeeee";

#[derive(Default)]
struct Computer {
    launches: Vec<Launch>,
    ended: BTreeSet<String>,
    selected_before_end: bool,
}

fn reply(computer: &mut Computer, act: Act) -> Result<Answer, Box<dyn Error>> {
    Ok(match act {
        Act::Start { launch, .. } => {
            let answer = Answer::Started {
                session: launch.session.clone(),
                pid: 42,
                started_at: 1,
                qualification: None,
            };
            computer.launches.push(*launch);
            answer
        }
        Act::Liveness { session } => {
            let sessions = computer
                .launches
                .iter()
                .filter(|launch| session.as_ref().is_none_or(|id| id == &launch.session))
                .map(|launch| {
                    let ended = computer.ended.contains(&launch.session);
                    LivenessView {
                        session: launch.session.clone(),
                        pid: Some(42),
                        alive: !ended,
                        managed: false,
                        harness_session: Some(NATIVE.to_owned()),
                        started_fresh: false,
                        turn_active: false,
                        last_signal_at: Some(1),
                        started_at: 1,
                        ended,
                    }
                })
                .collect();
            if session.is_some() && computer.ended.is_empty() {
                computer.selected_before_end = true;
            }
            Answer::Liveness { sessions }
        }
        Act::End { session } => {
            computer.ended.insert(session.clone());
            Answer::Ended {
                session,
                ended: lys_runner::Ended {
                    how: lys_runner::EndedHow::Exited,
                    at: 2,
                    status: Some(0),
                    signal: None,
                    reason: None,
                    stopped: None,
                },
            }
        }
        Act::Status { session } => Answer::Status {
            status: StatusView {
                runner: "fixture".to_owned(),
                protocol: Greeting::fresh("fixture").version,
                sessions: computer
                    .launches
                    .iter()
                    .filter(|launch| session.as_ref().is_none_or(|id| id == &launch.session))
                    .map(|launch| {
                        serde_json::from_value(json!({
                            "session": launch.session, "pid": 42, "started_at": 1,
                            "columns": 80, "rows": 24, "oldest": 0, "cursor": 0,
                            "account": null, "moves": [], "ended": null, "control": null,
                        }))
                    })
                    .collect::<Result<Vec<_>, _>>()?,
            },
        },
        Act::Owned => Answer::Owned {
            owners: Vec::new(),
            unreachable: Vec::new(),
        },
        Act::Feed { .. } => Answer::Refused {
            refusal: "feed_not_kept".to_owned(),
            words: "this fixture keeps no feed".to_owned(),
            oldest: None,
        },
        other => return Err(format!("unexpected runner request: {other:?}").into()),
    })
}

async fn serve(
    listener: tokio::net::UnixListener,
    answering: Arc<Mutex<Computer>>,
    server: [u8; 32],
) -> Result<(), String> {
    loop {
        let (stream, _) = listener.accept().await.map_err(|error| error.to_string())?;
        let mut stream = BufReader::new(stream);
        let greeting = Greeting::fresh("00000000000000000000000000000000");
        stream
            .write_all(format!("{}\n", greeting.line()).as_bytes())
            .await
            .map_err(|error| error.to_string())?;
        let mut line = String::new();
        stream
            .read_line(&mut line)
            .await
            .map_err(|error| error.to_string())?;
        let act = verify_request(&line, &server, &greeting).map_err(|error| error.to_string())?;
        let answer = {
            let mut computer = answering.lock().map_err(|error| error.to_string())?;
            reply(&mut computer, act).map_err(|error| error.to_string())?
        };
        stream
            .write_all(format!("{}\n", reply_line(answer)).as_bytes())
            .await
            .map_err(|error| error.to_string())?;
    }
}

async fn resumes(harness: &str, flag: &str) -> TestResult {
    let held = Held::open().await?;
    let mut declared = harness_description::declared();
    if harness == "codex" {
        declared["name"] = json!("codex");
        declared["description"]["models"]["maximum"] = json!(1);
        declared["description"]["models"]["further_encoding"] = json!({"kind":"array"});
        declared["description"]["rendering_contract"] = json!("codex/template-v1");
    }
    held.ok(&format!("/agents/{}/provisioning", held.agent()), &json!({
        "operation": operation()?, "from_version": 1, "working_folder": "/tmp",
        "model_access": ["model"], "tools": [], "skills": [], "mcp_servers": [],
        "instructions": "", "note": "", "permissions": {"default_mode":"plan"}, "harness": declared,
    })).await?;
    held.review(2).await?;
    let socket = held.dir.path().join("conversation.sock");
    let listener = tokio::net::UnixListener::bind(&socket)?;
    let computer = Arc::new(Mutex::new(Computer::default()));
    let answering = Arc::clone(&computer);
    let server = held.key.public_key_bytes();
    let task = tokio::spawn(serve(listener, answering, server));
    let result = async {
        held.ok(
            &format!("/network/machines/{}/runner", held.machine),
            &json!({"runner":{"kind":"socket","path":socket}}),
        )
        .await?;
        let before = held.start().await?;
        let preview_path = format!(
            "/agents/{}/start-command?machine={}",
            held.agent(),
            held.machine
        );
        let (status, preview) = held.service.get(&preview_path, Some(&held.ada)).await?;
        assert_eq!(status, 200, "{preview}");
        assert_eq!(preview["machine"], held.machine);
        assert_eq!(preview["directory"], "/tmp");
        let (status, _) = held.service.get(&preview_path, Some(&held.bea)).await?;
        assert_eq!(status, 403);
        let request = json!({"session":before["session"],"operation":operation()?});
        computer
            .lock()
            .map_err(|error| error.to_string())?
            .selected_before_end = false;
        let path = format!("/agents/{}/restart", held.agent());
        let after = held.ok(&path, &request).await?;
        assert_eq!(after["conversation"], NATIVE);
        assert_eq!(after["directory"], "/tmp");
        assert_ne!(after["session"], before["session"]);
        let repeated = held.ok(&path, &request).await?;
        assert_eq!(repeated["session"], after["session"]);
        let computer = computer.lock().map_err(|error| error.to_string())?;
        assert!(computer.selected_before_end);
        assert_eq!(computer.launches.len(), 2);
        assert!(
            computer.launches[1]
                .arguments
                .windows(2)
                .any(|pair| pair[0] == flag && pair[1] == NATIVE)
        );
        assert!(
            !computer.launches[1]
                .arguments
                .iter()
                .any(|argument| argument == "--session-id")
        );
        Ok::<(), Box<dyn Error>>(())
    }
    .await;
    task.abort();
    match task.await {
        Err(error) if error.is_cancelled() => {}
        Err(error) => return Err(error.into()),
        Ok(Err(error)) => return Err(error.into()),
        Ok(Ok(())) => return Err("the runner fixture stopped before cleanup".into()),
    }
    held.close()?;
    result
}

#[tokio::test(flavor = "multi_thread")]
async fn restart_resumes_the_claude_code_session_id() -> TestResult {
    resumes("claude-code", "--resume").await
}

#[tokio::test(flavor = "multi_thread")]
async fn restart_resumes_the_live_codex_thread() -> TestResult {
    resumes("codex", "resume").await
}
