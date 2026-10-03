//! Gates on the kept headers: only the named ones, never a credential, every
//! value of a repeated name, and the whole of it on the call's record with
//! the bytes that passed unchanged.

use std::collections::BTreeMap;

use http_body_util::BodyExt;
use hyper::header::{HeaderName, HeaderValue};
use hyper::{HeaderMap, StatusCode};

use super::{answered, asked};
use crate::proxy::forward_tests::{
    Harness, KEY, Res, fake, message_response, messages_request, send, whole,
};
use crate::record::call::captured::Head;

fn headers(pairs: &[(&'static str, &'static str)]) -> HeaderMap {
    let mut map = HeaderMap::new();
    for (name, value) in pairs {
        map.append(
            HeaderName::from_static(name),
            HeaderValue::from_static(value),
        );
    }
    map
}

fn map(pairs: &[(&str, &[&str])]) -> BTreeMap<String, Vec<String>> {
    pairs
        .iter()
        .map(|(name, values)| {
            let values = values.iter().map(|value| (*value).to_owned()).collect();
            ((*name).to_owned(), values)
        })
        .collect()
}

const CREDENTIALS: [(&str, &str); 5] = [
    ("authorization", "Bearer never-kept"),
    ("x-api-key", "never-kept"),
    ("cookie", "never=kept"),
    ("set-cookie", "never=kept"),
    ("proxy-authorization", "Basic never-kept"),
];

#[test]
fn only_the_named_request_headers_are_kept_and_no_credential() {
    let mut sent = vec![
        ("accept-encoding", "gzip, br"),
        ("anthropic-beta", "one"),
        ("anthropic-beta", "two"),
        ("anthropic-version", "2023-06-01"),
        ("content-type", "application/json"),
        ("user-agent", "claude-cli/9"),
        ("x-stainless-lang", "js"),
        (
            "request-id",
            "a response header's name, not kept of a request",
        ),
    ];
    sent.extend(CREDENTIALS);
    assert_eq!(
        asked(&headers(&sent)),
        map(&[
            ("accept-encoding", &["gzip, br"]),
            ("anthropic-beta", &["one", "two"]),
            ("anthropic-version", &["2023-06-01"]),
            ("content-type", &["application/json"]),
            ("user-agent", &["claude-cli/9"]),
        ])
    );
}

#[test]
fn only_the_named_response_headers_and_the_rate_limit_report_are_kept() {
    let mut came = vec![
        ("content-encoding", "br"),
        ("content-type", "application/json"),
        ("request-id", "req_011"),
        ("retry-after", "3"),
        ("x-request-id", "req_openai"),
        ("anthropic-ratelimit-requests-remaining", "7"),
        ("anthropic-ratelimit-unified-5h-utilization", "0.42"),
        ("x-ratelimit-remaining-tokens", "900"),
        ("anthropic-organization-id", "not kept"),
        ("server", "not kept"),
        (
            "user-agent",
            "a request header's name, not kept of a response",
        ),
    ];
    came.extend(CREDENTIALS);
    assert_eq!(
        answered(&headers(&came)),
        map(&[
            ("anthropic-ratelimit-requests-remaining", &["7"]),
            ("anthropic-ratelimit-unified-5h-utilization", &["0.42"]),
            ("content-encoding", &["br"]),
            ("content-type", &["application/json"]),
            ("request-id", &["req_011"]),
            ("retry-after", &["3"]),
            ("x-ratelimit-remaining-tokens", &["900"]),
            ("x-request-id", &["req_openai"]),
        ])
    );
}

#[test]
fn a_value_that_is_not_utf8_is_kept_as_having_been_there() -> Res {
    let mut came = HeaderMap::new();
    came.insert("request-id", HeaderValue::from_bytes(b"req_\xff")?);
    assert_eq!(answered(&came), map(&[("request-id", &["req_\u{fffd}"])]));
    Ok(())
}

#[test]
fn the_request_id_is_the_providers_own_header_first() {
    let both = Head {
        status: Some(200),
        request: BTreeMap::new(),
        response: map(&[("request-id", &["req_a"]), ("x-request-id", &["req_b"])]),
    };
    assert_eq!(both.request_id(), Some("req_a".to_owned()));
    let other = Head {
        response: map(&[("x-request-id", &["req_b"])]),
        ..Head::default()
    };
    assert_eq!(other.request_id(), Some("req_b".to_owned()));
    assert_eq!(Head::default().request_id(), None);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_calls_record_carries_its_status_request_id_and_kept_headers() -> Res {
    let (upstream, _) = fake(|| async {
        let mut response = whole(StatusCode::TOO_MANY_REQUESTS, &message_response());
        for (name, value) in [
            ("request-id", "req_011"),
            ("retry-after", "3"),
            ("anthropic-ratelimit-requests-remaining", "0"),
            ("set-cookie", "never=kept"),
        ] {
            response.headers_mut().insert(
                HeaderName::from_static(name),
                HeaderValue::from_static(value),
            );
        }
        response
    })
    .await?;
    let harness = Harness::start(upstream).await?;
    let mut request = messages_request(Some(KEY), false)?;
    for (name, value) in [
        ("authorization", "Bearer never-kept"),
        ("accept-encoding", "gzip, br"),
        ("anthropic-version", "2023-06-01"),
    ] {
        request.headers_mut().insert(
            HeaderName::from_static(name),
            HeaderValue::from_static(value),
        );
    }
    let (response, _connection) = send(harness.addr, request).await?;
    // What the client is answered is what the upstream sent, headers and all.
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(response.headers()["request-id"], "req_011");
    assert_eq!(response.headers()["set-cookie"], "never=kept");
    let body = response.into_body().collect().await?.to_bytes();
    assert_eq!(body, message_response().to_string());

    let report = harness.report()?;
    assert!(report.linked);
    let calls = harness.calls(KEY)?;
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].request_id.as_deref(), Some("req_011"));
    assert_eq!(
        calls[0].head,
        Some(Head {
            status: Some(429),
            request: map(&[
                ("accept-encoding", &["gzip, br"]),
                ("anthropic-version", &["2023-06-01"]),
                ("content-type", &["application/json"]),
            ]),
            response: map(&[
                ("anthropic-ratelimit-requests-remaining", &["0"]),
                ("content-type", &["application/json"]),
                ("request-id", &["req_011"]),
                ("retry-after", &["3"]),
            ]),
        })
    );
    Ok(())
}
