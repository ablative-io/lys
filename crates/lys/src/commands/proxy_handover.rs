//! The model proxy's listener handed to the build that succeeds it, so no
//! model call fails because of an upgrade.
//!
//! Every proxy started with `--handover` serves that Unix socket, owner-only,
//! for its whole life. A successor started with `--take-over` connects and
//! asks; the running proxy stops accepting, finishes and records every call
//! in flight while new calls wait in the listener's queue, removes its
//! socket, and sends the listener itself. The successor accepts from the
//! same queue and only then opens the journal and the home, so they have one
//! writer at a time. Nothing waits on a clock: the successor waits until the
//! listener arrives or the running proxy closes without sending it, which is
//! refused by name.

use std::io::{BufRead, BufReader, Write};
use std::os::fd::AsFd;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::sync::Arc;

use lys_home::proxy::forward::Proxy;

use crate::commands::error::{CliError, CliResult};

/// What a successor writes to ask for the listener.
const ASK: &str = "take";

/// The byte the listener is sent with.
const GIVEN: u8 = b'L';

fn io(context: String) -> impl FnOnce(std::io::Error) -> CliError {
    move |source| CliError::Io { context, source }
}

/// Take the listener from the proxy serving `handover`, once it has
/// finished and recorded every call it was answering.
pub fn take(handover: &Path) -> CliResult<std::net::TcpListener> {
    let shown = handover.display();
    let mut stream = std::os::unix::net::UnixStream::connect(handover)
        .map_err(io(format!("asking the running proxy at {shown} for its listener")))?;
    stream
        .write_all(format!("{ASK}\n").as_bytes())
        .and_then(|()| stream.flush())
        .map_err(io(format!("asking the running proxy at {shown} for its listener")))?;
    let (marker, fd) = lys_runner::handover::receive_descriptor(&stream)?;
    if marker != GIVEN {
        return Err(CliError::Io {
            context: format!("taking the listener from the running proxy at {shown}"),
            source: std::io::Error::other(format!("it sent marker {marker}, not the listener")),
        });
    }
    let listener = std::net::TcpListener::from(fd);
    listener
        .set_nonblocking(true)
        .map_err(io(format!("taking the listener from {shown}")))?;
    Ok(listener)
}

/// Whether a proxy answers on `handover`: one built before handover does
/// not, and is stopped and started as before.
pub fn served(handover: &Path) -> bool {
    std::os::unix::net::UnixStream::connect(handover).is_ok()
}

/// Bind `handover`, owner-only. A socket file nothing answers on is one a
/// proxy left behind, and is replaced; one a proxy answers on is refused.
fn bind(handover: &Path) -> CliResult<tokio::net::UnixListener> {
    let shown = handover.display();
    if handover.exists() {
        if served(handover) {
            return Err(CliError::Io {
                context: format!("serving the proxy's handover at {shown}"),
                source: std::io::Error::other("another proxy answers there"),
            });
        }
        std::fs::remove_file(handover).map_err(io(format!("removing the stale {shown}")))?;
    }
    let listener = tokio::net::UnixListener::bind(handover)
        .map_err(io(format!("serving the proxy's handover at {shown}")))?;
    std::fs::set_permissions(handover, std::fs::Permissions::from_mode(0o600))
        .map_err(io(format!("closing {shown} to others")))?;
    Ok(listener)
}

/// The first connection on `ask` that asks for the listener.
async fn asked(ask: tokio::net::UnixListener) -> CliResult<tokio::net::UnixStream> {
    loop {
        let (stream, _) = ask
            .accept()
            .await
            .map_err(io("accepting on the proxy's handover".to_owned()))?;
        let stream = stream
            .into_std()
            .map_err(io("reading a handover ask".to_owned()))?;
        stream
            .set_nonblocking(false)
            .map_err(io("reading a handover ask".to_owned()))?;
        let mut line = String::new();
        let read = BufReader::new(&stream).read_line(&mut line);
        if read.is_ok() && line.trim() == ASK {
            stream
                .set_nonblocking(true)
                .map_err(io("holding a handover ask".to_owned()))?;
            return tokio::net::UnixStream::from_std(stream)
                .map_err(io("holding a handover ask".to_owned()));
        }
        eprintln!("lys-proxy: a connection to the handover did not ask for the listener");
    }
}

/// Serve `proxy` on `listener` until a successor asks on `handover`, then
/// finish every call, hand the listener over and return, ending the proxy.
pub async fn serve(
    proxy: Arc<Proxy>,
    listener: tokio::net::TcpListener,
    handover: &Path,
    state: &Path,
) -> CliResult<()> {
    let ask = bind(handover)?;
    let (taken, taking) = tokio::sync::oneshot::channel();
    drop(tokio::spawn(async move {
        match asked(ask).await {
            Ok(stream) => {
                if taken.send(stream).is_err() {
                    eprintln!("lys-proxy: the handover ask was not heard");
                }
            }
            // The proxy goes on serving calls; it cannot be handed over,
            // and an upgrade then stops and starts it, saying so.
            Err(error) => {
                eprintln!("lys-proxy: proxy_handover_unavailable: {error}");
                std::future::pending::<()>().await;
            }
        }
    }));
    let mut taker = None;
    let listener = lys_home::proxy::handing::serve_then_hand(proxy, listener, state, async {
        taker = taking.await.ok();
    })
    .await?;
    let Some(taker) = taker else {
        return Err(CliError::Io {
            context: format!("serving the proxy's handover at {}", handover.display()),
            source: std::io::Error::other("the handover ask ended unheard"),
        });
    };
    let taker = taker
        .into_std()
        .map_err(io("answering the handover ask".to_owned()))?;
    taker
        .set_nonblocking(false)
        .map_err(io("answering the handover ask".to_owned()))?;
    // Removed before the listener is sent, so the successor binds a free path.
    std::fs::remove_file(handover).map_err(io(format!("removing {}", handover.display())))?;
    lys_runner::handover::send_descriptor(&taker, listener.as_fd(), GIVEN)?;
    eprintln!("lys-proxy: listener handed over; every call this proxy held was answered and recorded");
    Ok(())
}
