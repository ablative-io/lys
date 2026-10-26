#![cfg(test)]
//! Local runner control is request-owned, like its signed session asks.

use std::error::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[tokio::test]
async fn closing_a_local_control_caller_closes_its_peer_and_removes_its_wait()
-> Result<(), Box<dyn Error>> {
    let root = tempfile::tempdir()?;
    let socket = root.path().join("runner.sock");
    let listener = tokio::net::UnixListener::bind(socket.with_extension("reconnect.sock"))?;
    let quiet = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let address = quiet.local_addr()?;
    let relay = tokio::spawn(async move {
        let (mut local, _) = listener.accept().await?;
        let mut upstream = tokio::net::TcpStream::connect(address).await?;
        tokio::io::copy_bidirectional(&mut local, &mut upstream).await
    });
    let mut asking = Box::pin(super::local_control(&socket, "control-close-test", b'S'));
    let (mut peer, _) = tokio::select! {
        accepted = quiet.accept() => accepted?,
        _ = &mut asking => return Err("quiet control answered".into()),
    };
    let mut command = [0];
    tokio::select! {
        read = peer.read_exact(&mut command) => { read?; },
        _ = &mut asking => return Err("quiet control answered before a close".into()),
    }
    assert_eq!(command, [b'S']);
    let call = crate::waits::snapshot()?
        .into_iter()
        .find(|call| call.subject == "control-close-test")
        .ok_or("control wait absent")?;
    drop(asking);
    assert_eq!(peer.read(&mut command).await?, 0);
    peer.shutdown().await?;
    drop(peer);
    relay.await??;
    assert!(
        !crate::waits::snapshot()?
            .iter()
            .any(|entry| entry.id == call.id)
    );
    Ok(())
}
