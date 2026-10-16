#![cfg(test)]
use crate::budgets_feed::entry_tests::fixture;
use crate::seats_state::{Added, Line, Started};
use lys_runner::{Act, Answer, Ended, EndedHow};
use serde_json::json;
use std::error::Error;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
type Result<T = ()> = std::result::Result<T, Box<dyn Error>>;
const SESSION: &str = "op-0123456789abcdef0123456789abcdef";

async fn answer(listener: tokio::net::UnixListener, key: [u8; 32]) -> std::io::Result<()> {
    loop {
        let (stream, _) = listener.accept().await?;
        let greeting = lys_runner::protocol::Greeting::fresh("0123456789abcdef0123456789abcdef");
        let mut stream = BufReader::new(stream);
        stream
            .get_mut()
            .write_all(format!("{}\n", greeting.line()).as_bytes())
            .await?;
        let mut request = String::new();
        stream.read_line(&mut request).await?;
        let act = lys_runner::protocol::verify_request(&request, &key, &greeting)
            .map_err(std::io::Error::other)?;
        let act = match act {
            Act::AsCaller { done, .. } => *done,
            other => other,
        };
        let (answer, done) = match act {
            Act::Owned => (Answer::Owned { owners: Vec::new(), unreachable: Vec::new() }, false),
            Act::Liveness { .. } => (Answer::Liveness { sessions: vec![lys_runner::liveness::LivenessView {
                session: SESSION.to_owned(), pid: Some(42), alive: true, harness_session: None, turn_active: false, last_signal_at: Some(1), started_at: 1, ended: false,harness: lys_runner::liveness::HarnessLiveness { managed: false, started_fresh: false },}] }, false),
            Act::End { session } => (Answer::Ended { session, ended: Ended { how: EndedHow::Exited, at: 3, status: Some(0), signal: None, reason: Some("stdin EOF".to_owned()), stopped: None } }, true),
            Act::Operate { .. } | Act::Input { .. } | Act::Keys { .. } => (Answer::Refused { refusal: "acp_stdin_close_required".to_owned(), words: "an ACP seat must close stdin instead of receiving a terminal exit operation".to_owned(), oldest: None }, true),
            _ => return Err(std::io::Error::other("unexpected non-stop request in the ACP stop fixture")),
        };
        stream
            .get_mut()
            .write_all(format!("{}\n", lys_runner::protocol::reply_line(answer)).as_bytes())
            .await?;
        if done {
            return Ok(());
        }
    }
}

#[tokio::test]
async fn seat_program_stop_closes_stdin_without_a_terminal_exit_or_timed_wait() -> Result {
    let table = fixture().await?;
    let socket = table.state.estate_plan_file.with_file_name("acp-stop.sock");
    let listener = tokio::net::UnixListener::bind(&socket)?;
    let key =
        lys_core::Ed25519Identity::load(&table.state.grant_setup.key_file)?.public_key_bytes();
    let worker = tokio::spawn(answer(listener, key));
    let result: Result<serde_json::Value> = async {
        table.track(SESSION, "machine")?;
        crate::network_api::with_network(&table.state, |store| {
            store.name_runner(
                "machine",
                Some(crate::runner_client::RunnerRecord::Socket {
                    path: socket.to_string_lossy().into_owned(),
                }),
            )
        })?;
        let held = crate::seats_api::with_seats(&table.state, |store| {
            store.keep(Line::Added(Added {
                operation: "add-acp".to_owned(),
                name: "acp-fixture".to_owned(),
                agent: table.agent.clone(),
                harness: "seat-program/v1".to_owned(),
                profile_version: 1,
                machine: "machine".to_owned(),
                working_folder: None,
                account: None,
                by: "person".to_owned(),
                at: 1,
            }))?;
            store.keep(Line::Started(Started {
                operation: SESSION.to_owned(),
                name: "acp-fixture".to_owned(),
                session: SESSION.to_owned(),
                harness_session: None,
                transport: "acp".to_owned(),
                qualification: None,
                by: "person".to_owned(),
                at: 2,
            }))?;
            store.named("acp-fixture")
        })?;
        super::end(
            &table.state,
            &held,
            "person",
            "op-fedcba9876543210fedcba9876543210",
            false,
        )
        .await
        .map(|ended| json!({"ended": ended.ended}))
        .map_err(Into::into)
    }
    .await;
    worker.abort();
    match worker.await {
        Ok(result) => result?,
        Err(error) if error.is_cancelled() => {}
        Err(error) => return Err(error.into()),
    }
    table.finish().await?;
    let ended: serde_json::Value = result?;
    assert_eq!(ended["ended"], true);
    Ok(())
}
