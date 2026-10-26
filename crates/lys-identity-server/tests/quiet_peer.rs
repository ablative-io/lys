#![cfg(test)]

//! HOME-040: peers are released by a close or an answer, never a clock.

use std::error::Error;

use axum::body::Body;
use axum::extract::Request;
use lys_identity_server::mail::{MailSettings, Mailer, Tls};
use lys_identity_server::mail_letters::Letter;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

type Outcome = Result<(), Box<dyn Error>>;

fn letter(email: &str) -> Letter {
    Letter::EmailChange {
        email: email.to_owned(),
        code: "proof".to_owned(),
        leaving: false,
    }
}

fn mailer(port: u16, sent_file: std::path::PathBuf) -> Result<Mailer, Box<dyn Error>> {
    Ok(Mailer::open(&MailSettings {
        host: "127.0.0.1".to_owned(),
        port,
        user: None,
        password_file: None,
        from: "Lys <lys@example.test>".to_owned(),
        tls: Tls::None,
        sent_file,
    })?)
}

#[tokio::test]
async fn a_quiet_letter_leaves_the_record_free_and_its_caller_closes_the_socket() -> Outcome {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let directory = tempfile::tempdir()?;
    let sender = mailer(listener.local_addr()?.port(), directory.path().join("sent"))?;
    let first = letter("a@example.test");
    let mut sending = Box::pin(sender.send_once("first", &first));
    let (mut socket, _) = tokio::select! {
        accepted = listener.accept() => accepted?,
        result = &mut sending => return Err(format!("quiet SMTP answered: {result:?}").into()),
    };
    let waiting = lys_identity_server::waits::snapshot()?.into_iter()
        .find(|call| call.subject == "first").ok_or("quiet letter was not listed")?;
    // An independent operation must reach the same transport while the first is quiet.
    let second = letter("b@example.test");
    let mut other = Box::pin(sender.send_once("second", &second));
    let (mut second_socket, _) = tokio::select! {
        accepted = listener.accept() => accepted?,
        result = &mut other => return Err(format!("quiet SMTP answered: {result:?}").into()),
    };
    let sink = identity_contract::mail_sink::MailSink::start().await?;
    let mut answering = tokio::net::TcpStream::connect(("127.0.0.1", sink.port())).await?;
    let (sent, relayed) = tokio::join!(other, tokio::io::copy_bidirectional(&mut second_socket, &mut answering));
    assert!(sent?.sent);
    relayed?;
    drop(sending);
    assert_eq!(socket.read(&mut [0; 1]).await?, 0);
    assert!(sender.recorded_by("first", first.name()).await?.is_none());
    assert!(sender.recorded_by("second", second.name()).await?.is_some());
    assert!(!lys_identity_server::waits::snapshot()?.iter().any(|call| call.id == waiting.id));
    // The departed operation's own claim is available again, too.
    let retry = sender.send_once("first", &first);
    let relay = async {
        let (mut socket, _) = listener.accept().await?;
        let mut answering = tokio::net::TcpStream::connect(("127.0.0.1", sink.port())).await?;
        tokio::io::copy_bidirectional(&mut socket, &mut answering).await
    };
    let (retried, relayed) = tokio::join!(retry, relay);
    assert!(retried?.sent);
    relayed?;
    assert!(sender.recorded_by("first", first.name()).await?.is_some());
    Ok(())
}

#[tokio::test]
async fn the_same_letter_in_flight_is_sent_once() -> Outcome {
    let mut sink = identity_contract::mail_sink::MailSink::start().await?;
    let directory = tempfile::tempdir()?;
    let sender = mailer(sink.port(), directory.path().join("sent"))?;
    let letter = letter("once@example.test");
    let (first, second) = tokio::join!(
        sender.send_once("same", &letter),
        sender.send_once("same", &letter),
    );
    assert!(first?.sent);
    assert!(second?.sent);
    assert_eq!(sink.next().await.ok_or("sink closed")?.to, ["once@example.test"]);
    assert_eq!(std::fs::read_to_string(directory.path().join("sent"))?.lines().count(), 1);
    Ok(())
}

#[tokio::test]
async fn a_model_peer_close_is_a_named_refusal_with_a_status_and_body() -> Outcome {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let address = format!("http://{}", listener.local_addr()?);
    let request = Request::builder()
        .uri("/0123456789abcdef0123456789abcdef/openai/v1/responses")
        .body(Body::empty())?;
    let mut forwarding = Box::pin(lys_identity_server::model_proxy_route::forward(&address, request));
    let (mut socket, _) = tokio::select! {
        accepted = listener.accept() => accepted?,
        _ = &mut forwarding => return Err("quiet model answered".into()),
    };
    let mut bytes = [0; 4096];
    tokio::select! {
        read = socket.read(&mut bytes) => assert!(read? > 0),
        _ = &mut forwarding => return Err("quiet model answered".into()),
    }
    socket.shutdown().await?;
    drop(socket);
    let answer = forwarding.await;
    assert_eq!(answer.status(), axum::http::StatusCode::SERVICE_UNAVAILABLE);
    let body = axum::body::to_bytes(answer.into_body(), usize::MAX).await?;
    let body: serde_json::Value = serde_json::from_slice(&body)?;
    assert_eq!(body["refusal"], "model_upstream_unreachable");
    Ok(())
}

#[tokio::test]
async fn a_model_call_dropped_by_its_caller_closes_the_quiet_peer() -> Outcome {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let address = format!("http://{}", listener.local_addr()?);
    let request = Request::builder()
        .uri("/0123456789abcdef0123456789abcdef/openai/v1/responses")
        .body(Body::empty())?;
    let mut forwarding = Box::pin(lys_identity_server::model_proxy_route::forward(&address, request));
    let (mut socket, _) = tokio::select! {
        accepted = listener.accept() => accepted?,
        _ = &mut forwarding => return Err("quiet model answered".into()),
    };
    let mut bytes = [0; 4096];
    tokio::select! {
        read = socket.read(&mut bytes) => assert!(read? > 0),
        _ = &mut forwarding => return Err("quiet model answered".into()),
    }
    let waiting = lys_identity_server::waits::snapshot()?.into_iter()
        .find(|call| call.peer == address).ok_or("quiet model was not listed")?;
    assert!(!waiting.subject.contains("0123456789abcdef0123456789abcdef"));
    drop(forwarding);
    assert_eq!(socket.read(&mut bytes).await?, 0);
    assert!(!lys_identity_server::waits::snapshot()?.iter().any(|call| call.id == waiting.id));
    let next = Request::builder()
        .uri("/0123456789abcdef0123456789abcdef/openai/v1/responses")
        .body(Body::empty())?;
    let mut forwarding = Box::pin(lys_identity_server::model_proxy_route::forward(&address, next));
    let (mut socket, _) = tokio::select! {
        accepted = listener.accept() => accepted?,
        _ = &mut forwarding => return Err("the next model call did not connect".into()),
    };
    tokio::select! {
        read = socket.read(&mut bytes) => assert!(read? > 0),
        _ = &mut forwarding => return Err("the next model call answered without its peer".into()),
    }
    drop(socket);
    assert_eq!(forwarding.await.status(), axum::http::StatusCode::SERVICE_UNAVAILABLE);
    Ok(())
}

#[path = "quiet_peer/services.rs"]
mod services;

#[path = "quiet_peer/email.rs"]
mod email;

#[path = "quiet_peer/runner.rs"]
mod runner;
