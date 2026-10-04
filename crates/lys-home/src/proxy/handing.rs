//! Serving until the listener is handed to a successor, without failing a
//! call.
//!
//! When the hand-over is asked for, the proxy stops accepting but keeps its
//! listener open, so a call that arrives meanwhile waits in the kernel's
//! queue rather than being refused. Every connection is told to finish the
//! call it is answering and then close; an idle one closes at once. Once
//! every connection has ended and the sink has recorded every call, the
//! listener is answered to the caller, which hands it to the successor:
//! the successor accepts from the same queue, so nothing that arrived is
//! lost, and it begins only once this proxy will write nothing more, so the
//! journal and the home have one writer at a time.

use std::convert::Infallible;
use std::future::Future;
use std::path::Path;
use std::sync::Arc;

use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;
use tokio::task::JoinSet;

use crate::proxy::error::ProxyError;
use crate::proxy::forward::Proxy;

/// Serve `proxy` on `listener` until `asked` resolves, then finish every
/// call in flight, record them, and answer the listener, still open.
/// `state` is the proxy's state directory, named if the recording fails.
pub async fn serve_then_hand(
    proxy: Arc<Proxy>,
    listener: TcpListener,
    state: &Path,
    asked: impl Future<Output = ()>,
) -> Result<TcpListener, ProxyError> {
    tokio::pin!(asked);
    let (finish, finishing) = tokio::sync::watch::channel(false);
    let mut connections = JoinSet::new();
    loop {
        let (stream, _) = tokio::select! {
            biased;
            () = &mut asked => break,
            finished = connections.join_next(), if !connections.is_empty() => {
                if let Some(Err(error)) = finished {
                    eprintln!("lys-proxy: proxy_connection_task_failed: {error}");
                }
                continue;
            }
            accepted = listener.accept() => {
                accepted.map_err(|source| ProxyError::Accept { source })?
            }
        };
        let proxy = Arc::clone(&proxy);
        let mut finishing = finishing.clone();
        connections.spawn(async move {
            let service = service_fn(move |request| {
                let proxy = Arc::clone(&proxy);
                async move { Ok::<_, Infallible>(proxy.handle(request).await) }
            });
            let connection = http1::Builder::new().serve_connection(TokioIo::new(stream), service);
            tokio::pin!(connection);
            let served = tokio::select! {
                served = connection.as_mut() => served,
                _ = finishing.wait_for(|finish| *finish) => {
                    connection.as_mut().graceful_shutdown();
                    connection.await
                }
            };
            if let Err(error) = served {
                eprintln!("lys-proxy: proxy_connection_failed: {error}");
            }
        });
    }
    eprintln!(
        "lys-proxy: handing the listener over: finishing {} connections",
        connections.len()
    );
    if finish.send(true).is_err() {
        eprintln!("lys-proxy: no connection was waiting to be told to finish");
    }
    while let Some(result) = connections.join_next().await {
        if let Err(error) = result {
            eprintln!("lys-proxy: proxy_connection_task_failed: {error}");
        }
    }
    if let Err(error) = proxy.sink().shutdown() {
        return Err(ProxyError::io(
            "recording every call before handing the listener over",
            state,
            std::io::Error::other(error.to_string()),
        ));
    }
    Ok(listener)
}
