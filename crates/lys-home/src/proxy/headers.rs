//! The headers a call's record keeps: a fixed, named few from each side.
//!
//! PROXY.md asks for "headers that matter". These are the ones a reader of a
//! `lys.call` needs and cannot get from the stored bodies: the provider's id
//! for the request, what the bodies are and how they are encoded, what the
//! client offered to accept, the api version and betas asked for, the client
//! that asked, and the provider's rate limit report and its wait.
//!
//! Invariants:
//! - A header is kept only by a name written in this file. No header that
//!   carries a credential is named here (`authorization`, `x-api-key`,
//!   `cookie`, `set-cookie`, `proxy-authorization`), so none is ever read.
//! - Nothing is changed on the way: the maps are read from the headers the
//!   call already carries, after which the headers pass as they came.
//! - A name sent more than once keeps every value in the order sent. A value
//!   that is not UTF-8 is kept with each unreadable byte replaced, so the
//!   record says the header was there.

use std::collections::BTreeMap;

use hyper::HeaderMap;

/// Request headers kept, by exact name.
const ASKED: [&str; 6] = [
    "accept-encoding",
    "anthropic-beta",
    "anthropic-version",
    "content-encoding",
    "content-type",
    "user-agent",
];

/// Response headers kept, by exact name.
const ANSWERED: [&str; 5] = [
    "content-encoding",
    "content-type",
    "request-id",
    "retry-after",
    "x-request-id",
];

/// Response headers kept by the start of their name: the provider's rate
/// limit report, one header per window and unit.
const ANSWERED_FAMILIES: [&str; 2] = ["anthropic-ratelimit-", "x-ratelimit-"];

/// The request's kept headers.
pub(super) fn asked(headers: &HeaderMap) -> BTreeMap<String, Vec<String>> {
    kept(headers, |name| ASKED.contains(&name))
}

/// The response's kept headers.
pub(super) fn answered(headers: &HeaderMap) -> BTreeMap<String, Vec<String>> {
    kept(headers, |name| {
        ANSWERED.contains(&name)
            || ANSWERED_FAMILIES
                .iter()
                .any(|start| name.starts_with(start))
    })
}

fn kept(headers: &HeaderMap, keep: impl Fn(&str) -> bool) -> BTreeMap<String, Vec<String>> {
    let mut out: BTreeMap<String, Vec<String>> = BTreeMap::new();
    // A header name is held in lower case, so the names above match as written.
    for (name, value) in headers {
        if keep(name.as_str()) {
            out.entry(name.as_str().to_owned())
                .or_default()
                .push(String::from_utf8_lossy(value.as_bytes()).into_owned());
        }
    }
    out
}

#[cfg(test)]
#[path = "headers_tests.rs"]
mod tests;
