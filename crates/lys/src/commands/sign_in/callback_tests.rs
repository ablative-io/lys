use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::{Duration, Instant};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[tokio::test]
async fn an_elapsed_approval_deadline_names_the_next_sign_in_instead_of_waiting_forever()
-> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let error = super::wait(listener, "state", Instant::now() - Duration::from_secs(1))
        .await
        .err()
        .ok_or("expired approval accepted")?
        .to_string();
    assert!(error.contains("sign_in_approval_expired"));
    assert!(error.contains("lys sign-in"));
    Ok(())
}

#[tokio::test]
async fn an_incomplete_connection_cannot_hold_up_a_whole_valid_callback() -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let mut served = tokio::spawn(super::wait(
        listener,
        "fixture-state",
        Instant::now() + Duration::from_secs(600),
    ));
    let mut incomplete = TcpStream::connect(address).await?;
    let mut browser = TcpStream::connect(address).await?;
    browser.write_all(b"GET /lys/sign-in?state=fixture-state&code=fixture-code HTTP/1.1\r\nHost: localhost\r\n\r\n").await?;
    let mut byte = [0_u8; 1];
    let code = tokio::select! {
        result = &mut served => result??,
        read = incomplete.read(&mut byte) => {
            if !served.is_finished() {
                served.abort();
                let cancelled = served.await;
                assert!(cancelled.is_err());
                return Err(format!("incomplete connection ended before the valid callback was accepted: {read:?}").into());
            }
            assert_eq!(read?, 0);
            served.await??
        }
    };
    assert_eq!(code.as_str(), "fixture-code");
    let mut response = String::new();
    browser.read_to_string(&mut response).await?;
    assert!(response.contains("Approval received"));
    assert!(!response.contains("Signed in to Lys"));
    drop(incomplete);
    Ok(())
}
