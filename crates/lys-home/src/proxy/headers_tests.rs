//! Gates on what the record keeps of the headers: every name of each side in
//! the order received, every value but a credential's, never a credential's
//! value however its header is named, and the whole of it on the call's record with the bytes that
//! passed unchanged.

use std::collections::BTreeMap;

use http_body_util::BodyExt;
use hyper::header::{HeaderName, HeaderValue};
use hyper::{HeaderMap, StatusCode};

use super::{NEVER_VALUED, answered, asked, credential, kept};
use crate::proxy::forward_tests::{
    Harness, KEY, Res, fake, message_response, messages_request, send, whole,
};
use crate::record::call::captured::{Head, Side};

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

fn values(pairs: &[(&str, &[&str])]) -> BTreeMap<String, Vec<String>> {
    pairs
        .iter()
        .map(|(name, values)| {
            let values = values.iter().map(|value| (*value).to_owned()).collect();
            ((*name).to_owned(), values)
        })
        .collect()
}

fn names(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| (*name).to_owned()).collect()
}

const CREDENTIALS: [(&str, &str); 5] = [
    ("authorization", "Bearer never-kept"),
    ("x-api-key", "never-kept"),
    ("cookie", "never=kept"),
    ("set-cookie", "never=kept"),
    ("proxy-authorization", "Basic never-kept"),
];

#[test]
fn every_request_header_is_named_and_every_value_but_a_credentials_is_kept() {
    let mut sent = vec![
        ("accept-encoding", "gzip, br"),
        ("anthropic-beta", "one"),
        ("x-stainless-lang", "js"),
        ("anthropic-beta", "two"),
        ("anthropic-version", "2023-06-01"),
        ("content-type", "application/json"),
        ("user-agent", "claude-cli/9"),
        ("x-stainless-runtime", "node"),
        ("x-stainless-runtime-version", "v22.1.0"),
        ("x-stainless-package-version", "0.60.0"),
        ("x-stainless-os", "MacOS"),
        ("x-stainless-arch", "arm64"),
        ("x-claude-code-session-id", "0f0e"),
    ];
    sent.extend(CREDENTIALS);
    let side = asked(&headers(&sent));
    assert_eq!(
        side.values,
        values(&[
            ("accept-encoding", &["gzip, br"]),
            ("anthropic-beta", &["one", "two"]),
            ("anthropic-version", &["2023-06-01"]),
            ("content-type", &["application/json"]),
            ("user-agent", &["claude-cli/9"]),
            ("x-claude-code-session-id", &["0f0e"]),
            ("x-stainless-arch", &["arm64"]),
            ("x-stainless-lang", &["js"]),
            ("x-stainless-os", &["MacOS"]),
            ("x-stainless-package-version", &["0.60.0"]),
            ("x-stainless-runtime", &["node"]),
            ("x-stainless-runtime-version", &["v22.1.0"]),
        ])
    );
    // No header vanishes: each of the eighteen lines is named once, and a
    // header map keeps a repeated name's lines together.
    let mut named = side.names.clone();
    named.sort();
    let mut every: Vec<String> = sent.iter().map(|(name, _)| (*name).to_owned()).collect();
    every.sort();
    assert_eq!(named, every);
    assert_eq!(side.names.len(), 18);
}

#[test]
fn every_response_header_is_named_and_every_value_but_a_credentials_is_kept() {
    let mut came = vec![
        ("content-encoding", "br"),
        ("content-type", "application/json"),
        ("request-id", "req_011"),
        ("retry-after", "3"),
        ("x-request-id", "req_openai"),
        ("anthropic-ratelimit-requests-remaining", "7"),
        ("anthropic-ratelimit-tokens-remaining", "900"),
        ("anthropic-ratelimit-unified-5h-utilization", "0.42"),
        ("anthropic-organization-id", "org_1"),
        ("x-ratelimit-remaining-tokens", "900"),
        ("server", "cloudflare"),
        ("cf-ray", "8a1"),
    ];
    came.extend(CREDENTIALS);
    let side = answered(&headers(&came));
    assert_eq!(
        side.values,
        values(&[
            ("anthropic-organization-id", &["org_1"]),
            ("anthropic-ratelimit-requests-remaining", &["7"]),
            ("anthropic-ratelimit-tokens-remaining", &["900"]),
            ("anthropic-ratelimit-unified-5h-utilization", &["0.42"]),
            ("cf-ray", &["8a1"]),
            ("content-encoding", &["br"]),
            ("content-type", &["application/json"]),
            ("request-id", &["req_011"]),
            ("retry-after", &["3"]),
            ("server", &["cloudflare"]),
            ("x-ratelimit-remaining-tokens", &["900"]),
            ("x-request-id", &["req_openai"]),
        ])
    );
    assert_eq!(
        side.names,
        came.iter()
            .map(|(name, _)| (*name).to_owned())
            .collect::<Vec<_>>(),
        "every header is named, in the order received"
    );
}

#[test]
fn a_credential_header_is_named_and_never_valued_from_either_side() {
    let came = headers(&CREDENTIALS);
    for side in [asked(&came), answered(&came), kept(&came)] {
        assert_eq!(side.values, BTreeMap::new());
        assert_eq!(side.names.len(), 5, "each is on the record by name");
    }
    for (name, _) in CREDENTIALS {
        assert!(NEVER_VALUED.contains(&name), "{name}");
    }
}

#[test]
fn a_credential_header_nobody_listed_is_refused_a_value_by_how_it_is_named() {
    // None of these is in the list; each is named as a credential is.
    let unlisted = [
        ("x-goog-api-key", "never-kept"),
        ("openai-apikey", "never-kept"),
        ("x-amz-security-token", "never-kept"),
        ("x-auth", "never-kept"),
        ("x-upstream-authorization", "never-kept"),
        ("x-client-secret", "never-kept"),
        ("x-hub-signature", "never-kept"),
        ("x-csrf-token", "never-kept"),
        ("x_session_cookie", "never-kept"),
    ];
    let side = kept(&headers(&unlisted));
    assert_eq!(side.values, BTreeMap::new());
    assert_eq!(side.names.len(), unlisted.len());
    // A word is matched whole: a header that counts tokens, or names an
    // author, carries no credential and keeps its value.
    for name in [
        "anthropic-ratelimit-tokens-remaining",
        "x-ratelimit-remaining-tokens",
        "x-author",
        "x-tokenizer",
        "x-claude-code-session-id",
    ] {
        assert!(!credential(name), "{name}");
    }
}

#[test]
fn a_value_that_is_not_utf8_is_kept_as_far_as_it_reads() -> Res {
    let mut came = HeaderMap::new();
    came.insert("request-id", HeaderValue::from_bytes(b"req_\xff")?);
    let side = answered(&came);
    assert_eq!(side.values, values(&[("request-id", &["req_\u{fffd}"])]));
    assert_eq!(side.names, names(&["request-id"]));
    Ok(())
}

#[test]
fn the_request_id_is_the_providers_own_header_first() {
    let response = |pairs: &[(&str, &[&str])]| Head {
        status: Some(200),
        request: Side::default(),
        response: Side {
            names: Vec::new(),
            values: values(pairs),
        },
    };
    let both = response(&[("request-id", &["req_a"]), ("x-request-id", &["req_b"])]);
    assert_eq!(both.request_id(), Some("req_a".to_owned()));
    let other = response(&[("x-request-id", &["req_b"])]);
    assert_eq!(other.request_id(), Some("req_b".to_owned()));
    assert_eq!(Head::default().request_id(), None);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_calls_record_carries_its_status_request_id_and_headers() -> Res {
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
    let head = calls[0].head.as_ref().ok_or("the record has no head")?;
    assert_eq!(head.status, Some(429));
    // What was sent and what came back is on the record with its values,
    // beside whatever the transport added itself (a content length among
    // it), so each expected value is asserted and then the rule for all.
    for (side, expected) in [
        (
            &head.request,
            values(&[
                ("accept-encoding", &["gzip, br"]),
                ("anthropic-version", &["2023-06-01"]),
                ("content-type", &["application/json"]),
            ]),
        ),
        (
            &head.response,
            values(&[
                ("anthropic-ratelimit-requests-remaining", &["0"]),
                ("content-type", &["application/json"]),
                ("request-id", &["req_011"]),
                ("retry-after", &["3"]),
            ]),
        ),
    ] {
        for (name, value) in &expected {
            assert_eq!(side.values.get(name), Some(value), "{name}");
        }
        // Every header named has its value kept exactly when it carries no
        // credential: nothing else is left out, and nothing more is let in.
        for name in &side.names {
            assert_eq!(side.values.contains_key(name), !credential(name), "{name}");
        }
    }
    // The credential of each side is on the record by name and by name only;
    // the names the transport adds itself are there too, so only presence is
    // asserted.
    for (side, name) in [
        (&head.request, "authorization"),
        (&head.response, "set-cookie"),
    ] {
        assert!(side.names.iter().any(|named| named == name), "{name}");
        assert!(!side.values.contains_key(name), "{name}");
    }
    let record = serde_json::to_string(&calls[0])?;
    assert!(
        !record.contains("never"),
        "no credential's value is on the record"
    );
    Ok(())
}
