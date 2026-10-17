//! A sign-in return address as an app's registration and its Sign-in
//! settings admit it, and a redirect a sign-in presents judged against a
//! listed entry (DIRECTORY-097).
//!
//! An address is absolute, `https` anywhere or `http` to this machine
//! (`localhost`, `127.0.0.1` or `[::1]`), and carries no fragment: the rule as it
//! stood, in its order, for every address without a wildcard. An `https`
//! address may name every machine of a domain: its host is `*.` followed by
//! a domain of two or more labels, the wildcard being the whole first label
//! and nothing else of the address. An http wildcard is refused as http off
//! this machine is; a bare `*`, a second wildcard label, a wildcard inside a
//! label, a one-label or empty-labelled suffix, a wildcard beside a user or
//! outside the host, and a wildcard under a domain written outside ASCII are
//! each refused `redirect_invalid` with their own reason.
//!
//! A presented redirect is admitted by an entry it equals byte for byte, or,
//! when the entry is a wildcard entry, by the entry's own text with exactly
//! one label (a-z, 0-9 and hyphen, compared without regard to ASCII case, as
//! the suffix is) in the wildcard's place: the scheme, port, path and query
//! as the entry writes them, nothing normalised, nothing implied. What a
//! sign-in presents is judged as presented and never rewritten.

use reqwest::Url;

use crate::apps_error::AppError;

const BARE_WILDCARD: &str =
    "is a bare wildcard; a wildcard is one label under a domain of two or more labels";
const DEEPER_WILDCARD: &str =
    "has more than one wildcard label; only the first label of the host may be a wildcard";
const WILDCARD_INSIDE_LABEL: &str =
    "has a wildcard inside a label; the wildcard is the whole first label";
const ONE_LABEL_SUFFIX: &str = "names a wildcard under a one-label suffix; the domain under the wildcard needs two or more labels";
const EMPTY_LABEL: &str = "has an empty label in the domain under the wildcard";
const WILDCARD_USER: &str =
    "names a user beside a wildcard; a wildcard address names machines and no user";
const WILDCARD_OUTSIDE_HOST: &str =
    "has a wildcard outside the host; the wildcard is the whole first label of the host";
const WILDCARD_NON_ASCII: &str = "names a domain outside ASCII under the wildcard; the domain is written as a browser sends it, in ASCII";

/// The suffix under a wildcard host (`*.` and a domain of two or more
/// labels), `None` for a host with no wildcard, or the refusal's reason for
/// any other shape of `*` in a host.
fn wildcard_suffix(host: &str) -> Result<Option<&str>, &'static str> {
    if !host.contains('*') {
        return Ok(None);
    }
    if host == "*" || host == "*." {
        return Err(BARE_WILDCARD);
    }
    let Some(suffix) = host.strip_prefix("*.") else {
        return Err(WILDCARD_INSIDE_LABEL);
    };
    if suffix.split('.').any(|label| label == "*") {
        return Err(DEEPER_WILDCARD);
    }
    if suffix.contains('*') {
        return Err(WILDCARD_INSIDE_LABEL);
    }
    if suffix.split('.').any(str::is_empty) {
        return Err(EMPTY_LABEL);
    }
    if suffix.split('.').count() < 2 {
        return Err(ONE_LABEL_SUFFIX);
    }
    Ok(Some(suffix))
}

/// `address` as it is kept, or `redirect_invalid` with its reason: the one
/// rule for a registration's addresses and for the sign-in settings' alike.
pub(crate) fn admitted(address: &str) -> Result<String, AppError> {
    let refused = |reason| AppError::RedirectInvalid {
        address: address.to_owned(),
        reason,
    };
    let url = Url::parse(address).map_err(|_unparsed| refused("is not an absolute address"))?;
    let local = matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"));
    match url.scheme() {
        "https" => {}
        "http" if local => {}
        _ => return Err(refused("is neither https nor http to this machine")),
    }
    if url.fragment().is_some() {
        return Err(refused("carries a fragment"));
    }
    let host = url.host_str().unwrap_or_default();
    let in_host = host.matches('*').count();
    if address.matches('*').count() > in_host {
        return Err(refused(WILDCARD_OUTSIDE_HOST));
    }
    if wildcard_suffix(host).map_err(refused)?.is_some() {
        if !url.username().is_empty() || url.password().is_some() {
            return Err(refused(WILDCARD_USER));
        }
        if !address.is_ascii() {
            return Err(refused(WILDCARD_NON_ASCII));
        }
    }
    Ok(address.to_owned())
}

/// Whether the listed `entry` admits `presented`: equal byte for byte, or,
/// for a wildcard entry, the entry's own text with one label in the
/// wildcard's place. `presented` is judged as given and never rewritten.
pub(crate) fn matches(entry: &str, presented: &str) -> bool {
    let wildcard = Url::parse(entry)
        .ok()
        .and_then(|url| url.host_str().map(|host| host.starts_with("*.")))
        .unwrap_or(false);
    let Some((before, after)) = entry.split_once("*.").filter(|_| wildcard) else {
        return entry == presented;
    };
    let host_end = after.find(['/', '?', ':', '#']).unwrap_or(after.len());
    let (suffix, rest) = after.split_at(host_end);
    let Some(tail) = presented.strip_prefix(before) else {
        return false;
    };
    let Some((label, under)) = tail.split_once('.') else {
        return false;
    };
    let label_ok = !label.is_empty()
        && label
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-');
    let Some((presented_suffix, presented_rest)) = under.split_at_checked(suffix.len()) else {
        return false;
    };
    label_ok && presented_suffix.eq_ignore_ascii_case(suffix) && presented_rest == rest
}
