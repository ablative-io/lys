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
