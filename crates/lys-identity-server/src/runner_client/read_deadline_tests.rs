//! An unanswered observation closes the socket and preserves a named failure.

use crate::runner_client::{RunnerRecord, ask};
use lys_runner::{Act, protocol::Greeting};
use std::error::Error;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

#[tokio::test]
async fn an_unanswered_status_read_has_a_deadline_and_closes_its_socket()
-> Result<(), Box<dyn Error>> {
    let table = crate::budgets_feed::entry_tests::fixture().await?;
    let dir = tempfile::tempdir()?;
    let socket = dir.path().join("unanswered.sock");
    let listener = tokio::net::UnixListener::bind(&socket)?;
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
        line.clear();
        assert_eq!(
            stream.read_line(&mut line).await?,
            0,
            "the timed-out caller left its socket open"
        );
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
    tokio::time::pause();
    tokio::time::advance(std::time::Duration::from_secs(6)).await;
    let refusal = asking
        .await?
        .err()
        .ok_or("an unanswered observation succeeded")?;
    tokio::time::resume();
    assert_eq!(refusal.name(), "runner_unreachable");
    assert!(refusal.to_string().contains("within 5 seconds"));
    worker.await??;
    dir.close()?;
    table.finish().await
}

#[tokio::test]
async fn a_dialled_status_ticket_expires_only_when_its_own_caller_times_out()
-> Result<(), Box<dyn Error>> {
    use std::future::Future;
    use std::task::Poll;

    let table = crate::budgets_feed::entry_tests::fixture().await?;
    let mut next = Box::pin(table.state.runners.hub().next("machine"));
    std::future::poll_fn(|context| match next.as_mut().poll(context) {
        Poll::Pending => Poll::Ready(Ok(())),
        Poll::Ready(_) => Poll::Ready(Err("the unasked bridge did not wait")),
    })
    .await?;
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
    let (ticket, _) = next.await?;
    assert!(
        table
            .state
            .runners
            .hub()
            .with("machine", |held| held.delivered.contains_key(&ticket))?
    );
    tokio::time::pause();
    tokio::time::advance(std::time::Duration::from_secs(6)).await;
    let timeout = asking
        .await?
        .err()
        .ok_or("an unanswered status did not time out")?;
    tokio::time::resume();
    assert_eq!(timeout.name(), "runner_unreachable");
    let late = table
        .state
        .runners
        .hub()
        .reply("machine", &ticket, Ok("late".to_owned()))
        .err()
        .ok_or("a timed-out caller received a late reply")?;
    assert_eq!(late.name(), "runner_reply_expired");
    assert_eq!(late.status(), axum::http::StatusCode::GONE);
    table.finish().await
}
