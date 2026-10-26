#![cfg(test)]
//! An unanswered observation closes the socket and preserves a named failure.

use crate::runner_client::{RunnerRecord, ask};
use lys_runner::{Act, protocol::Greeting};
use std::error::Error;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};

#[tokio::test]
async fn an_unanswered_status_read_ends_on_its_caller_and_closes_its_socket()
-> Result<(), Box<dyn Error>> {
    let table = crate::budgets_feed::entry_tests::fixture().await?;
    let dir = tempfile::tempdir()?;
    let socket = dir.path().join("unanswered.sock");
    let listener = tokio::net::UnixListener::bind(&socket)?;
    let quiet = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let address = quiet.local_addr()?;
    let peer_name = socket.to_string_lossy().into_owned();
    let key = table.state.runners.public_key();
    let (seen, entered) = tokio::sync::oneshot::channel();
    let worker = tokio::spawn(async move {
        let (stream, _) = listener.accept().await?;
        let mut stream = BufReader::new(stream);
        let greeting = Greeting::fresh("0123456789abcdef0123456789abcdef");
        stream
            .get_mut()
            .write_all(format!("{}\n", greeting.line()).as_bytes())
            .await?;
        let mut line = String::new();
        stream.read_line(&mut line).await?;
        let act = lys_runner::protocol::verify_request(&line, &key, &greeting)
            .map_err(std::io::Error::other)?;
        assert!(matches!(act, Act::Status { session: None }));
        seen.send(())
            .map_err(|()| std::io::Error::other("status entry receiver left"))?;
        let mut upstream = tokio::net::TcpStream::connect(address).await?;
        upstream.write_all(line.as_bytes()).await?;
        // The caller's close ends the relay; on macOS the closing shutdown
        // answers NotConnected, which is that close, not a failure.
        match tokio::io::copy_bidirectional(stream.get_mut(), &mut upstream).await {
            Err(error) if error.kind() != std::io::ErrorKind::NotConnected => return Err(error),
            _ => {}
        }
        let (stream, _) = listener.accept().await?;
        let mut stream = BufReader::new(stream);
        let greeting = Greeting::fresh("0123456789abcdef0123456789abcdef");
        stream
            .get_mut()
            .write_all(format!("{}\n", greeting.line()).as_bytes())
            .await?;
        line.clear();
        stream.read_line(&mut line).await?;
        assert!(matches!(
            lys_runner::protocol::verify_request(&line, &key, &greeting)
                .map_err(std::io::Error::other)?,
            Act::Status { session: None }
        ));
        let refused = lys_runner::Answer::refusal(&lys_runner::RunnerError::refused(
            "operation_refused",
            "the next ask reached the runner",
        ));
        stream
            .get_mut()
            .write_all(format!("{}\n", lys_runner::protocol::reply_line(refused)).as_bytes())
            .await?;
        Ok::<_, std::io::Error>(())
    });
    let state = std::sync::Arc::clone(&table.state);
    let asking = tokio::spawn(async move {
        ask(
            &state,
            "machine",
            RunnerRecord::Socket {
                path: socket.to_string_lossy().into_owned(),
            },
            Act::Status { session: None },
        )
        .await
    });
    entered.await?;
    let (mut peer, _) = quiet.accept().await?;
    assert!(peer.read(&mut [0; 4096]).await? > 0);
    let call = crate::waits::snapshot()?
        .into_iter()
        .find(|call| call.peer == peer_name)
        .ok_or("runner wait absent")?;
    asking.abort();
    assert!(asking.await.unwrap_err().is_cancelled());
    assert_eq!(peer.read(&mut [0; 1]).await?, 0);
    drop(peer);
    assert!(
        !crate::waits::snapshot()?
            .iter()
            .any(|entry| entry.id == call.id)
    );
    let next = ask(
        &table.state,
        "machine",
        RunnerRecord::Socket { path: peer_name },
        Act::Status { session: None },
    )
    .await
    .unwrap_err();
    assert_eq!(next.name(), "operation_refused");
    worker.await??;
    dir.close()?;
    table.finish().await
}

#[tokio::test]
async fn a_dialled_status_ticket_expires_only_when_its_own_caller_leaves()
-> Result<(), Box<dyn Error>> {
    let table = crate::budgets_feed::entry_tests::fixture().await?;
    table
        .state
        .runners
        .hub()
        .with("machine", |held| held.bridges = 1)?;
    let state = std::sync::Arc::clone(&table.state);
    let asking = tokio::spawn(async move {
        super::ask_exactly(
            &state,
            "machine",
            RunnerRecord::Dialled {
                key: "0".repeat(64),
                runner: None,
            },
            Act::Status { session: None },
            false,
        )
        .await
    });
    let (ticket, _) = table.state.runners.hub().next("machine").await?;
    assert!(
        table
            .state
            .runners
            .hub()
            .with("machine", |held| held.delivered.contains_key(&ticket))?
    );
    asking.abort();
    assert!(asking.await.unwrap_err().is_cancelled());
    let late = table
        .state
        .runners
        .hub()
        .reply("machine", &ticket, Ok("late".to_owned()))
        .err()
        .ok_or("a departed caller received a late reply")?;
    assert_eq!(late.name(), "runner_reply_expired");
    assert_eq!(late.status(), axum::http::StatusCode::GONE);
    table.finish().await
}
