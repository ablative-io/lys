#![cfg(test)]

use std::error::Error;
use std::io::ErrorKind;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use hyper::StatusCode;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

use super::{CONNECTION_LIMIT, refusal, serve};

type TestResult<T = ()> = Result<T, Box<dyn Error + Send + Sync>>;

async fn request(address: SocketAddr) -> TestResult<Vec<u8>> {
    let mut stream = TcpStream::connect(address).await?;
    stream
        .write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
        .await?;
    let mut answer = Vec::new();
    match stream.read_to_end(&mut answer).await {
        Ok(_) => {}
        Err(error) if error.kind() == ErrorKind::ConnectionReset => {}
        Err(error) => return Err(error.into()),
    }
    Ok(answer)
}

#[tokio::test]
async fn slow_connections_have_a_fixed_bound_and_saturation_is_named() -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let handled = Arc::new(AtomicUsize::new(0));
    let seen = Arc::clone(&handled);
    let serving = tokio::spawn(serve(listener, move |_| {
        seen.fetch_add(1, Ordering::SeqCst);
        async { refusal(StatusCode::OK, "forwarded") }
    }));
    let mut slow = Vec::with_capacity(CONNECTION_LIMIT);
    for _ in 0..CONNECTION_LIMIT {
        let mut stream = TcpStream::connect(address).await?;
        stream.write_all(b"GET / HTTP/1.1\r\nHost: ").await?;
        slow.push(stream);
    }
    let answer = request(address).await?;
    drop(slow);
    serving.abort();
    let stopped = serving.await;
    assert!(stopped.is_err_and(|error| error.is_cancelled()));
    let answer = String::from_utf8(answer)?;
    assert!(answer.starts_with("HTTP/1.1 503"), "{answer}");
    assert!(answer.contains("proxy_connections_full"), "{answer}");
    assert_eq!(handled.load(Ordering::SeqCst), 0);
    Ok(())
}

#[tokio::test]
async fn completed_connections_return_their_slots() -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let serving = tokio::spawn(serve(listener, |_| async {
        refusal(StatusCode::OK, "forwarded")
    }));
    for _ in 0..CONNECTION_LIMIT * 2 {
        let answer = String::from_utf8(request(address).await?)?;
        assert!(answer.starts_with("HTTP/1.1 200"), "{answer}");
        assert!(answer.ends_with("forwarded\n"), "{answer}");
    }
    serving.abort();
    let stopped = serving.await;
    assert!(stopped.is_err_and(|error| error.is_cancelled()));
    Ok(())
}
