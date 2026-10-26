#![cfg(test)]

//! A TCP quiet peer behind the runner's local protocol socket.

use super::Outcome;
use std::sync::Arc;
use axum::response::IntoResponse;
use lys_identity_server::runner_client::{Leave, LeaveOnDrop, RunnerRecord, Runners};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream, UnixListener};

#[tokio::test(flavor = "multi_thread")]
async fn a_runner_ask_ends_on_the_quiet_peers_close_by_name() -> Outcome {
    let quiet = TcpListener::bind("127.0.0.1:0").await?;
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("runner.sock");
    let local = UnixListener::bind(&path)?;
    let key = Arc::new(lys_core::Ed25519Identity::ephemeral());
    let public = key.public_key_bytes();
    let runners = Runners::new(key, None);
    let leave = Arc::new(Leave::default());
    let guard = LeaveOnDrop(Arc::clone(&leave));
    let asking = tokio::task::spawn_blocking(move || runners.ask("machine",
        &RunnerRecord::Socket { path:path.to_string_lossy().into_owned() },
        &lys_runner::Act::Status { session:None }, &leave));
    let (socket, _) = local.accept().await?;
    let mut socket = BufReader::new(socket);
    let greeting = lys_runner::protocol::Greeting::fresh("0123456789abcdef0123456789abcdef");
    socket.get_mut().write_all(format!("{}\n", greeting.line()).as_bytes()).await?;
    let mut request = String::new();
    socket.read_line(&mut request).await?;
    assert!(matches!(lys_runner::protocol::verify_request(&request, &public, &greeting)?, lys_runner::Act::Status { session:None }));
    let mut upstream = TcpStream::connect(quiet.local_addr()?).await?;
    let (mut peer, _) = quiet.accept().await?;
    upstream.write_all(request.as_bytes()).await?;
    assert!(peer.read(&mut [0; 4096]).await? > 0);
    drop(peer);
    let (relayed, result) = tokio::join!(tokio::io::copy_bidirectional(socket.get_mut(), &mut upstream), asking);
    relayed?;
    let error = result?.err().ok_or("closed runner succeeded")?;
    let refusal = lys_identity_server::error::ServerError::from(error);
    assert_eq!(refusal.name(), "runner_unreachable");
    let response = refusal.into_response();
    assert_eq!(response.status(), axum::http::StatusCode::SERVICE_UNAVAILABLE);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await?;
    let body: serde_json::Value = serde_json::from_slice(&body)?;
    assert_eq!(body["refusal"], "runner_unreachable");
    drop(guard);
    Ok(())
}
