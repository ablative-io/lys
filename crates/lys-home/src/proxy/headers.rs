//! What a call's record keeps of each side's headers: every name, and the
//! values of a named few.
//!
//! PROXY.md asks for "headers that matter". The values kept are the ones a
//! reader of a `lys.call` needs and cannot get from the stored bodies: the
//! provider's id for the request, what the bodies are and how they are
//! encoded, what the client offered to accept, the api version and betas
//! asked for, the client that asked, and everything the provider says about
//! the call in its own headers, its rate limit report among them.
//!
//! Invariants:
//! - No header is dropped without trace: every header's name is kept, one
//!   per header line, whether or not its value is. The order is the order
//!   received, with the later lines of a repeated name beside its first.
//! - A value is kept only for a name written in this file. The headers that
//!   carry a credential are written here too, as never valued, and that list
//!   is checked first: no other list in this file can give one a value.
//! - Nothing is changed on the way: this reads the headers the call already
//!   carries, after which they pass as they came.
//! - A name sent more than once keeps every value in the order received. A
//!   value that is not UTF-8 is kept with each unreadable byte replaced, so
//!   the record says what could be read of it.

use hyper::HeaderMap;

use crate::record::call::captured::Side;

/// Headers whose value is never kept, from either side: each carries a
/// credential. Their names are on the record like any other header's.
const NEVER_VALUED: [&str; 5] = [
    "authorization",
    "cookie",
    "proxy-authorization",
    "set-cookie",
    "x-api-key",
];

/// Request headers whose values are kept, by exact name.
const ASKED: [&str; 6] = [
    "accept-encoding",
    "anthropic-beta",
    "anthropic-version",
    "content-encoding",
    "content-type",
    "user-agent",
];

/// Response headers whose values are kept, by exact name.
const ANSWERED: [&str; 5] = [
    "content-encoding",
    "content-type",
    "request-id",
    "retry-after",
    "x-request-id",
];

/// Response headers whose values are kept by the start of their name: what
/// the provider says about the call, and the rate limit report of each.
/// `x-codex-` is the family the Codex upstream reports an account's primary
/// and secondary windows in.
const ANSWERED_FAMILIES: [&str; 3] = ["anthropic-", "x-ratelimit-", "x-codex-"];

/// What the record keeps of the request's headers.
pub(super) fn asked(headers: &HeaderMap) -> Side {
    kept(headers, |name| ASKED.contains(&name))
}

/// What the record keeps of the response's headers.
pub(super) fn answered(headers: &HeaderMap) -> Side {
    kept(headers, |name| {
        ANSWERED.contains(&name)
            || ANSWERED_FAMILIES
                .iter()
                .any(|start| name.starts_with(start))
    })
}

fn kept(headers: &HeaderMap, valued: impl Fn(&str) -> bool) -> Side {
    let mut side = Side::default();
    // A header name is held in lower case, so the names above match as written.
    for (name, value) in headers {
        let name = name.as_str();
        side.names.push(name.to_owned());
        if !NEVER_VALUED.contains(&name) && valued(name) {
            side.values
                .entry(name.to_owned())
                .or_default()
                .push(String::from_utf8_lossy(value.as_bytes()).into_owned());
        }
    }
    side
}

#[cfg(test)]
#[path = "headers_tests.rs"]
mod tests;
