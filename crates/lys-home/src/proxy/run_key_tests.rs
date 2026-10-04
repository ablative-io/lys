#![cfg(test)]
//! Gates on the run key: it is exactly what a launch mints, and a call whose
//! first path part is anything else is refused before it is journalled or
//! forwarded.

use std::sync::atomic::Ordering;

use http_body_util::{BodyExt, Full};
use hyper::header::CONTENT_TYPE;
use hyper::{Request, StatusCode};

use crate::proxy::forward::run_key;
use crate::proxy::forward_tests::{
    Harness, KEY, Res, fake, message_response, messages_body, send, whole,
};
use crate::proxy::usage::is_run_key;
use crate::record::call::CallStatus;

/// A run key of the shape a launch mints.
const RUN: &str = "0123456789abcdef0123456789abcdef";

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn only_what_a_launch_mints_is_a_run_key_and_any_other_first_part_is_refused() -> Res {
    // What a launch mints is a key, and is taken off the path.
    assert!(is_run_key(&crate::record::fresh_id()));
    let keyed = format!("/{RUN}/anthropic/v1/messages");
    assert_eq!(run_key(&keyed), (Some(RUN), "/anthropic/v1/messages"));
    assert_eq!(
        run_key("/anthropic/v1/messages"),
        (None, "/anthropic/v1/messages")
    );

    let (upstream, asked) = fake(|| async { whole(StatusCode::OK, &message_response()) }).await?;
    let harness = Harness::start(upstream).await?;
    // An over-long part, a part of the right letters and the wrong length,
    // upper-case digits, and a letter that is no digit.
    let wrong = [
        format!("{RUN}0"),
        RUN[1..].to_owned(),
        RUN.to_ascii_uppercase(),
        RUN.replace('f', "g"),
    ];
    for part in &wrong {
        assert!(!is_run_key(part), "{part}");
        let path = format!("/{part}/anthropic/v1/messages");
        assert_eq!(run_key(&path), (None, path.as_str()), "{part}");
        let request = Request::post(path)
            .header(CONTENT_TYPE, "application/json")
            .body(Full::new(messages_body(Some(KEY), false)))?;
        let (response, _connection) = send(harness.addr, request).await?;
        assert_eq!(response.status(), StatusCode::NOT_FOUND, "{part}");
        let words = response.into_body().collect().await?.to_bytes();
        let words = String::from_utf8_lossy(&words);
        assert!(words.contains("lys-proxy refused the call"), "{words}");
        assert!(words.contains("32 lowercase hexadecimal digits"), "{words}");
    }
    // Nothing of those calls was journalled, forwarded or recorded.
    assert_eq!(asked.load(Ordering::SeqCst), 0);
    assert_eq!(std::fs::read_dir(harness.state("journal"))?.count(), 0);

    // The same call under a key a launch could have minted goes through,
    // and its record names the run.
    let request = Request::post(keyed)
        .header(CONTENT_TYPE, "application/json")
        .body(Full::new(messages_body(Some(KEY), false)))?;
    let (response, _connection) = send(harness.addr, request).await?;
    assert_eq!(response.status(), StatusCode::OK);
    let answered = response.into_body().collect().await?.to_bytes();
    assert!(!answered.is_empty());
    let report = harness.report()?;
    assert_eq!(report.status, CallStatus::Complete);
    assert_eq!(asked.load(Ordering::SeqCst), 1);
    let calls = harness.calls(KEY)?;
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].run.as_deref(), Some(RUN));
    Ok(())
}
