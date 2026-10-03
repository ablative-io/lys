#![cfg(test)]

use std::error::Error;
use std::io::ErrorKind;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use hyper::StatusCode;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

use super::{refusal, serve};

// Both endpoints share this test process. Leave descriptors for its runtime
// and listener within a 256-descriptor test environment.
const CONCURRENT_CONNECTIONS: usize = 80;

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
async fn all_connections_beyond_the_old_limit_are_answered() -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let handled = Arc::new(AtomicUsize::new(0));
    let seen = Arc::clone(&handled);
    let serving = tokio::spawn(serve(listener, move |_| {
        seen.fetch_add(1, Ordering::SeqCst);
        async { refusal(StatusCode::OK, "forwarded") }
    }));
    let mut slow = Vec::with_capacity(CONCURRENT_CONNECTIONS);
    for _ in 0..CONCURRENT_CONNECTIONS {
        let mut stream = TcpStream::connect(address).await?;
        stream.write_all(b"GET / HTTP/1.1\r\nHost: ").await?;
        slow.push(stream);
    }
    let answer = request(address).await?;
    let answer = String::from_utf8(answer)?;
    assert!(answer.starts_with("HTTP/1.1 200"), "{answer}");
    assert!(answer.ends_with("forwarded\n"), "{answer}");
    let mut completed = tokio::task::JoinSet::new();
    for mut stream in slow {
        completed.spawn(async move {
            stream
                .write_all(b"localhost\r\nConnection: close\r\n\r\n")
                .await?;
            let mut answer = Vec::new();
            stream.read_to_end(&mut answer).await?;
            let answer = String::from_utf8(answer)?;
            assert!(answer.starts_with("HTTP/1.1 200"), "{answer}");
            assert!(answer.ends_with("forwarded\n"), "{answer}");
            TestResult::Ok(())
        });
    }
    while let Some(done) = completed.join_next().await {
        done??;
    }
    assert_eq!(handled.load(Ordering::SeqCst), CONCURRENT_CONNECTIONS + 1);
    serving.abort();
    let stopped = serving.await;
    assert!(stopped.is_err_and(|error| error.is_cancelled()));
    Ok(())
}

#[tokio::test]
async fn completed_connections_leave_the_listener_ready() -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let serving = tokio::spawn(serve(listener, |_| async {
        refusal(StatusCode::OK, "forwarded")
    }));
    for _ in 0..128 {
        let answer = String::from_utf8(request(address).await?)?;
        assert!(answer.starts_with("HTTP/1.1 200"), "{answer}");
        assert!(answer.ends_with("forwarded\n"), "{answer}");
    }
    serving.abort();
    let stopped = serving.await;
    assert!(stopped.is_err_and(|error| error.is_cancelled()));
    Ok(())
}
