#![cfg(test)]
//! Incomplete responses retain their cancellation or partial outcome.

use super::{
    CallStatus, Harness, KEY, Res, StatusCode, fake, fed, messages_events, messages_request, send,
};
use http_body_util::BodyExt;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_client_that_closes_mid_stream_is_recorded_cancelled_with_no_response_parts() -> Res {
    let (upstream, _) = fake(|| async {
        let (tx, response) = fed(StatusCode::OK, "text/event-stream");
        tokio::spawn(async move {
            // Withhold the terminal event so cancellation leaves an unfinished response.
            for event in messages_events(1).into_iter().take(1) {
                if tx.send(event).await.is_err() {
                    break;
                }
            }
            tx.closed().await;
        });
        response
    })
    .await?;
    let harness = Harness::start(upstream).await?;
    let (response, connection) = send(harness.addr, messages_request(Some(KEY), true)?).await?;
    let mut body = response.into_body();
    let first = body.frame().await.ok_or("no first frame")??;
    assert!(first.is_data());
    drop(body);
    connection.abort();
    let report = harness.report()?;
    assert_eq!(report.status, CallStatus::Cancelled);
    let calls = harness.calls(KEY)?;
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].status, CallStatus::Cancelled);
    assert!(calls[0].response.is_empty());
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn an_upstream_stream_that_ends_before_message_stop_is_partial() -> Res {
    let (upstream, _) = fake(|| async {
        let (tx, response) = fed(StatusCode::OK, "text/event-stream");
        tokio::spawn(async move {
            for event in messages_events(3).into_iter().take(4) {
                if tx.send(event).await.is_err() {
                    return;
                }
            }
        });
        response
    })
    .await?;
    let harness = Harness::start(upstream).await?;
    let (response, _connection) = send(harness.addr, messages_request(Some(KEY), true)?).await?;
    let received = response.into_body().collect().await?.to_bytes();
    assert_eq!(received, messages_events(3)[..4].concat());
    let report = harness.report()?;
    assert_eq!(report.status, CallStatus::Partial);
    let calls = harness.calls(KEY)?;
    assert_eq!(calls.len(), 1);
    assert!(calls[0].response.is_empty());
    Ok(())
}
