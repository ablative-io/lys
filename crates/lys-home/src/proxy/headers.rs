//! What a call's record keeps of each side's headers: every name, and every
//! value but a credential's.
//!
//! Tom, 5 October 2026, on the values an earlier list left out: "runtime,
//! runtime version... package version, OS, language, architecture... Claude
//! Code session ID, like all of these things are incredibly valuable." So no
//! list chooses what is worth keeping: a header's value is kept unless the
//! header carries a credential.
//!
//! Invariants:
//! - No header is dropped without trace: every header's name is kept, one
//!   per header line, whether or not its value is. The order is the order
//!   received, with the later lines of a repeated name beside its first.
//! - A credential's value is never kept, from either side. A header carries
//!   one when its name is written in [`NEVER_VALUED`], or when one of the
//!   words its name is made of (split at `-` and `_`) is written in
//!   [`CREDENTIAL_WORDS`], or when its name holds `api-key` or `apikey`: a
//!   credential header nobody listed is refused a value by how it is named.
//!   A word is matched whole, so `anthropic-ratelimit-tokens-remaining`,
//!   which counts tokens and carries none, keeps its value.
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

/// Words that, as one whole part of a header's name, say it carries a
/// credential.
const CREDENTIAL_WORDS: [&str; 9] = [
    "auth",
    "authorization",
    "cookie",
    "credential",
    "credentials",
    "password",
    "secret",
    "signature",
    "token",
];

/// Whether a header of this name carries a credential, by the list or by
/// how it is named. The name is in lower case, as a header map holds it.
pub(super) fn credential(name: &str) -> bool {
    NEVER_VALUED.contains(&name)
        || name.contains("api-key")
        || name.contains("apikey")
        || name
            .split(['-', '_'])
            .any(|word| CREDENTIAL_WORDS.contains(&word))
}

/// What the record keeps of the request's headers.
pub(super) fn asked(headers: &HeaderMap) -> Side {
    kept(headers)
}

/// What the record keeps of the response's headers.
pub(super) fn answered(headers: &HeaderMap) -> Side {
    kept(headers)
}

fn kept(headers: &HeaderMap) -> Side {
    let mut side = Side::default();
    // A header name is held in lower case, so the names above match as written.
    for (name, value) in headers {
        let name = name.as_str();
        side.names.push(name.to_owned());
        if !credential(name) {
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
