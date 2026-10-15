//! A held greeting has no idle deadline; a peer leaving before it is normal EOF.

use super::{DispatchSlots, connection};
use crate::{Act, Answer, Sessions};
use std::error::Error;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

#[tokio::test]
async fn a_peer_that_left_before_its_greeting_is_not_a_broken_connection()
-> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let sessions = Sessions::open(dir.path(), 4096)?;
    let (server, client) = tokio::net::UnixStream::pair()?;
    drop(client);
    let (stop, stopped) = tokio::sync::watch::channel(false);
    let result = connection(
        &sessions,
        [0; 32],
        server,
        Arc::new(DispatchSlots::new()),
        stopped,
    )
    .await;
    drop(stop);
    sessions.writer.barrier()?;
    assert!(result.is_ok(), "{result:?}");
    Ok(())
}

#[tokio::test]
async fn a_greeting_remains_usable_across_a_long_poll() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let sessions = Sessions::open(dir.path(), 4096)?;
    let key = lys_core::Ed25519Identity::ephemeral();
    let (server, client) = tokio::net::UnixStream::pair()?;
    let (stop, stopped) = tokio::sync::watch::channel(false);
    let held = Arc::clone(&sessions);
    let public = key.public_key_bytes();
    let serving = tokio::spawn(async move {
        connection(
            &held,
            public,
            server,
            Arc::new(DispatchSlots::new()),
            stopped,
        )
        .await
    });
    let mut client = BufReader::new(client);
    let mut greeting = String::new();
    client.read_line(&mut greeting).await?;
    tokio::time::pause();
    tokio::time::advance(std::time::Duration::from_secs(600)).await;
    tokio::time::resume();
    assert!(!serving.is_finished(), "the outstanding greeting expired");
    let greeting = crate::protocol::read_greeting(greeting.trim_end())?;
    let request = crate::protocol::sign_request(&key, &greeting, &Act::Liveness { session: None })?;
    client
        .get_mut()
        .write_all(format!("{request}\n").as_bytes())
        .await?;
    let mut reply = String::new();
    client.read_line(&mut reply).await?;
    let answer = crate::protocol::read_reply(&reply)?;
    stop.send(true)?;
    drop(client);
    serving.await??;
    sessions.writer.barrier()?;
    assert!(matches!(answer, Answer::Liveness { .. }));
    Ok(())
}
