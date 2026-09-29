#![cfg(test)]
//! Header ambiguity and origin matching independent of any JSON body's validity.

use axum::http::{HeaderMap, HeaderValue, header};

use super::check;

const ORIGIN: &str = "http://127.0.0.1:8490";

#[test]
fn only_one_json_media_type_and_one_exact_configured_origin_are_admitted() {
    let mut headers = HeaderMap::new();
    assert!(check(&headers, ORIGIN).is_err());
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    assert!(
        check(&headers, ORIGIN).is_ok(),
        "non-browser requests need no Origin"
    );
    headers.insert(header::ORIGIN, HeaderValue::from_static(ORIGIN));
    assert!(check(&headers, ORIGIN).is_ok());
    for origin in [
        "null",
        "http://127.0.0.1:8491",
        "https://127.0.0.1:8490",
        "http://localhost:8490",
        "http://127.0.0.1:8490/",
    ] {
        headers.insert(header::ORIGIN, HeaderValue::from_static(origin));
        assert!(check(&headers, ORIGIN).is_err(), "{origin}");
    }
    headers.insert(header::ORIGIN, HeaderValue::from_static(ORIGIN));
    headers.append(header::ORIGIN, HeaderValue::from_static(ORIGIN));
    assert!(check(&headers, ORIGIN).is_err(), "duplicate Origin refused");
    headers.remove(header::ORIGIN);
    headers.append(header::CONTENT_TYPE, HeaderValue::from_static("text/plain"));
    assert!(
        check(&headers, ORIGIN).is_err(),
        "duplicate media type refused"
    );
    for kind in [
        "application/json; charset=utf-8",
        "application/problem+json",
        "APPLICATION/JSON",
    ] {
        headers.insert(header::CONTENT_TYPE, HeaderValue::from_static(kind));
        assert!(check(&headers, ORIGIN).is_ok(), "{kind}");
    }
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static("text/plain"));
    assert!(check(&headers, ORIGIN).is_err());
}
