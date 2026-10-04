#![cfg(test)]
//! Run credentials ride the signed start act, never its answer or public receipt.
#[path = "support/runner_start.rs"]
mod support;
use lys_runner::protocol::{Greeting, reply_line, verify_request};
use lys_runner::{Act, Answer};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::error::Error;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixListener;
use support::{Table, operation};

type TestResult = Result<(), Box<dyn Error>>;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn agent_pass_start_act_and_end_report_keep_secrets_out_of_receipts() -> TestResult {
    let table = Table::set().await?;
    let socket = table.dir.path().join("capture.sock");
    let listener = UnixListener::bind(&socket)?;
    let key = table.server_key.public_key_bytes();
    let (send, receive) = std::sync::mpsc::channel();
    let answering = std::thread::spawn(move || {
        let result = (|| -> TestResult {
            for _ in 0..1 {
                let (stream, _) = listener.accept()?;
                let greeting = Greeting::fresh(&"a".repeat(32));
                let mut writer = &stream;
                writeln!(writer, "{}", greeting.line())?;
                let mut line = zeroize::Zeroizing::new(String::new());
                BufReader::new(&stream).read_line(&mut line)?;
                let act = verify_request(&line, &key, &greeting)?;
                let Act::AsCaller { done, .. } = act else {
                    return Err("start was not done for its caller".into());
                };
                let Act::Start {
                    launch,
                    lys_mcp: Some(entry),
                    ..
                } = *done
                else {
                    return Err("start carried no run pass".into());
                };
                assert_eq!(entry.pass.len(), 43);
                assert!(!format!("{entry:?}").contains(&entry.pass));
                assert!(!serde_json::to_string(&launch)?.contains(&entry.pass));
                let session = launch.session.clone();
                send.send((session.clone(), entry))
                    .map_err(|error| error.to_string())?;
                writeln!(
                    writer,
                    "{}",
                    reply_line(Answer::Started {
                        session,
                        pid: 42,
                        started_at: 1
                    })
                )?;
            }
            Ok(())
        })();
        result.map_err(|error| error.to_string())
    });
    let machine = table.machine(&json!({"operation": operation()?, "name":"Capture", "kind":"laptop", "runtime":"sh", "slots":2, "may_run":[table.agent()], "may_reach":[]}), Some(json!({"kind":"socket", "path":socket.display().to_string()}))).await?;
    for _ in 0..1 {
        let (status, answer) = table
            .start(
                &table.agent(),
                &json!({"machine":machine, "operation":operation()?}),
            )
            .await?;
        assert_eq!(status, 200);
        let (session, entry) = receive.recv()?;
        assert!(entry.url.ends_with("/api/mcp"));
        assert!(!answer.to_string().contains(&entry.pass));
        let file = table.service.dir.path().join("agent-passes.json");
        let bytes = std::fs::read_to_string(&file)?;
        assert!(!bytes.contains(&entry.pass));
        let digest = lys_runner::protocol::hex(&Sha256::digest(entry.pass.as_bytes()));
        let stored: Value = serde_json::from_str(&bytes)?;
        assert!(stored["passes"].get(&digest).is_some());
        let (status, receipt) = table.service.get("/runner-receipts/0", None).await?;
        assert_eq!(status, 200);
        assert!(!receipt.to_string().contains(&entry.pass));
        assert!(!receipt.to_string().contains("lys_mcp"));
        let seat = entry.seat.as_ref().ok_or("an AI's start carries no seat")?;
        assert!(!receipt.to_string().contains(&seat.key));
        let path = format!(
            "/agents/{}/runtime/sessions/{session}/reports",
            table.agent()
        );
        let (status, _) = table.service.post(&path, Some(&table.ada), &json!({"operation":operation()?, "machine":machine, "state":"stopped", "what":"process ended", "confirmation":"observed exit"})).await?;
        assert_eq!(status, 200);
        let ended: Value = serde_json::from_str(&std::fs::read_to_string(file)?)?;
        assert!(ended["passes"].get(&digest).is_none());
    }
    answering
        .join()
        .map_err(|error| format!("capture runner panicked: {error:?}"))??;
    table.close()
}

/// The fake runner's socket, the starts it received, and its thread.
type Capture = (
    std::path::PathBuf,
    std::sync::mpsc::Receiver<(String, lys_runner::protocol::LysMcp)>,
    std::thread::JoinHandle<Result<(), String>>,
);

/// A fake runner that accepts `starts` signed starts, answering each Started
/// and handing back each session and run entry. The service holds a grant
/// channel to a runner that answers, and follows its feed: this one holds
/// each channel it is asked for until it is done, and refuses its feed by
/// name. Neither is a start.
fn capture_starts(table: &Table, starts: usize) -> Result<Capture, Box<dyn Error>> {
    let socket = table.dir.path().join("capture-certificates.sock");
    let listener = UnixListener::bind(&socket)?;
    let key = table.server_key.public_key_bytes();
    let (send, receive) = std::sync::mpsc::channel();
    let answering = std::thread::spawn(move || {
        let result = (|| -> TestResult {
            let (mut started, mut channels) = (0, Vec::new());
            while started < starts {
                let (stream, _) = listener.accept()?;
                let greeting = Greeting::fresh(&"b".repeat(32));
                let mut writer = &stream;
                writeln!(writer, "{}", greeting.line())?;
                let mut line = zeroize::Zeroizing::new(String::new());
                BufReader::new(&stream).read_line(&mut line)?;
                let done = match verify_request(&line, &key, &greeting)? {
                    Act::AsCaller { done, .. } => done,
                    Act::GrantChannel => {
                        writeln!(writer, "{}", reply_line(Answer::GrantChannel))?;
                        channels.push(stream);
                        continue;
                    }
                    Act::Feed { .. } => {
                        let refused = Answer::Refused {
                            refusal: "feed_not_kept".to_owned(),
                            words: "this stand-in keeps no feed".to_owned(),
                            oldest: None,
                        };
                        writeln!(writer, "{}", reply_line(refused))?;
                        continue;
                    }
                    other => return Err(format!("the stand-in was asked {other:?}").into()),
                };
                started += 1;
                let Act::Start {
                    launch,
                    lys_mcp: Some(entry),
                    ..
                } = *done
                else {
                    return Err("start carried no run pass".into());
                };
                let session = launch.session.clone();
                send.send((session.clone(), entry))
                    .map_err(|error| error.to_string())?;
                writeln!(
                    writer,
                    "{}",
                    reply_line(Answer::Started {
                        session,
                        pid: 42,
                        started_at: 1
                    })
                )?;
            }
            Ok(())
        })();
        result.map_err(|error| error.to_string())
    });
    Ok((socket, receive, answering))
}

fn pass_held(table: &Table, pass: &str) -> Result<bool, Box<dyn Error>> {
    let file = table.service.dir.path().join("agent-passes.json");
    let stored: Value = serde_json::from_str(&std::fs::read_to_string(file)?)?;
    let digest = lys_runner::protocol::hex(&Sha256::digest(pass.as_bytes()));
    Ok(stored["passes"].get(&digest).is_some())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_ai_lys_starts_stands_on_one_certificate_and_its_withdrawal_ends_the_run() -> TestResult
{
    let table = Table::set().await?;
    let (socket, receive, answering) = capture_starts(&table, 2)?;
    let machine = table.machine(&json!({"operation": operation()?, "name":"Certified", "kind":"laptop", "runtime":"sh", "slots":2, "may_run":[table.agent()], "may_reach":[]}), Some(json!({"kind":"socket", "path":socket.display().to_string()}))).await?;
    let agent = table.agent();
    let certificates = format!("/agents/{agent}/certificates");

    let (status, _) = table
        .start(
            &agent,
            &json!({"machine":machine, "operation":operation()?}),
        )
        .await?;
    assert_eq!(status, 200);
    let (_, first) = receive.recv()?;
    let (status, held) = table.service.get(&certificates, Some(&table.ada)).await?;
    assert_eq!(status, 200);
    let issued = held["certificates"].as_array().ok_or("no certificates")?;
    assert_eq!(issued.len(), 1, "the AI is given one certificate: {held}");
    let serial = issued[0]["serial"].as_str().ok_or("no serial")?.to_owned();
    assert!(issued[0]["withdrawn"].is_null());
    let key = table
        .service
        .dir
        .path()
        .join("agent-keys")
        .join(&agent)
        .join(format!("{serial}.key"));
    assert!(key.is_file(), "Lys keeps the AI's key in its own store");
    assert!(pass_held(&table, &first.pass)?);
    let seat = first.seat.as_ref().ok_or("an AI's start carries no seat")?;
    assert_eq!(
        seat.serial, serial,
        "the seat is delegated by its certificate"
    );
    assert!(!format!("{first:?}").contains(&seat.key));

    let (status, withdrawn) = table
        .service
        .post(
            &format!("{certificates}/{serial}/withdrawal"),
            Some(&table.ada),
            &json!({"reason":"the AI is to stop acting"}),
        )
        .await?;
    assert_eq!(status, 200, "{withdrawn}");
    assert!(
        !pass_held(&table, &first.pass)?,
        "a withdrawn certificate ends the AI's runs"
    );

    let (status, _) = table
        .start(
            &agent,
            &json!({"machine":machine, "operation":operation()?}),
        )
        .await?;
    assert_eq!(status, 200);
    let (_, second) = receive.recv()?;
    let (_, held) = table.service.get(&certificates, Some(&table.ada)).await?;
    let issued = held["certificates"].as_array().ok_or("no certificates")?;
    let standing: Vec<&Value> = issued
        .iter()
        .filter(|certificate| certificate["withdrawn"].is_null())
        .collect();
    assert_eq!(issued.len(), 2, "{held}");
    assert_eq!(
        standing.len(),
        1,
        "the AI stands on one certificate: {held}"
    );
    assert_ne!(standing[0]["serial"], json!(serial));
    assert!(pass_held(&table, &second.pass)?);
    answering
        .join()
        .map_err(|error| format!("capture runner panicked: {error:?}"))??;
    table.close()
}
