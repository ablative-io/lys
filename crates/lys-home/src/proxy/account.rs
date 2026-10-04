//! Calls made on a model account. A run started on one is given, in the
//! variable its program reads its token from, a placeholder in place of the
//! token: [`MARKER`], the account, and the run's draw on it (a handle id and
//! the handle), which opens nothing without this proxy's own key. A call
//! that carries the placeholder is sent through the secrets broker, which
//! judges the agent's grant on the account, writes the sealed token into
//! the header the account's route names, sends the call to the account's
//! upstream, and takes the token out of whatever comes back. The token never
//! reaches the run, this proxy or its record.
//!
//! The broker reads a draw as a presentation signed with the holder's key
//! over the one request it is for, so this proxy reads the whole request
//! body before it is sent on: a call on a model account is held for its
//! body, and only that call. The presentation is written here in the
//! broker's own form, which the broker's tests check it admits.
//!
//! When the broker refuses the call it names its refusal in
//! [`REFUSAL_HEADER`]; when the provider refuses the token the broker put
//! in, the answer is the provider's own. Either is passed to the run as it
//! came, and this proxy's log names which it was and for which account.

use std::sync::Arc;

use http_body_util::{BodyExt, Full};
use hyper::body::{Bytes, Incoming};
use hyper::header::{HeaderName, HeaderValue};
use hyper::{HeaderMap, Request, Response, StatusCode};
use lys_core::Ed25519Identity;
use lys_core::attestation::sign_attestation;
use rand::TryRngCore;
use rand::rngs::OsRng;
use sha2::{Digest, Sha256};

use super::error::ProxyError;
use super::forward::{Base, ProxyBody, Upstream};

/// How a model account's placeholder opens.
pub const MARKER: &str = "lys-account";
/// The header the secrets broker names its own refusal in.
pub const REFUSAL_HEADER: &str = "lys-refusal";
/// The headers a program carries its token in, which the placeholder is
/// looked for in and taken out of before the call is sent on.
const CARRIERS: [&str; 2] = ["authorization", "x-api-key"];
const PRESENTATION_DOMAIN: &str = "lys-secrets/presentation/v2";
const REQUEST_DOMAIN: &str = "lys-secrets/request/v1";

/// The placeholder a run on `account` is given for its draw `handle`.
#[must_use]
pub fn placeholder(account: &str, handle: &str, token: &str) -> String {
    format!("{MARKER}.{account}.{handle}.{token}")
}

/// Where the secrets broker is, and the key this proxy presents draws with.
#[derive(Clone)]
pub struct Broker {
    /// The broker's base address, on the loopback.
    pub base: Base,
    /// This proxy's own key, the one every draw is bound to.
    pub key: Arc<Ed25519Identity>,
}

impl std::fmt::Debug for Broker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Broker")
            .field("base", &self.base.as_str())
            .finish_non_exhaustive()
    }
}

/// A draw a call carries. It prints nothing of the handle.
pub struct Drawn {
    account: String,
    handle: String,
    token: String,
    carrier: &'static str,
}

fn unread(reason: &str) -> ProxyError {
    ProxyError::ModelAccount {
        account: String::from("unread"),
        reason: reason.to_owned(),
    }
}

fn plain(text: &str) -> bool {
    !text.is_empty()
        && text
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
}

fn hexadecimal(text: &str) -> bool {
    !text.is_empty() && text.len() % 2 == 0 && text.bytes().all(|byte| byte.is_ascii_hexdigit())
}

impl Drawn {
    /// The draw `headers` carry, when one carries the placeholder.
    ///
    /// # Errors
    ///
    /// `ModelAccount` when the placeholder is there and does not read.
    pub fn carried(headers: &HeaderMap) -> Result<Option<Self>, ProxyError> {
        for carrier in CARRIERS {
            let Some(text) = headers.get(carrier).and_then(|value| value.to_str().ok()) else {
                continue;
            };
            let text = text.trim();
            let text = match text.split_once(' ') {
                Some((scheme, rest)) if scheme.eq_ignore_ascii_case("bearer") => rest.trim(),
                _ => text,
            };
            let Some(rest) = text
                .strip_prefix(MARKER)
                .and_then(|rest| rest.strip_prefix('.'))
            else {
                continue;
            };
            let parts: Vec<&str> = rest.split('.').collect();
            let [account, handle, token] = parts.as_slice() else {
                return Err(unread(
                    "the call carries a model account's placeholder that is not account, draw and handle",
                ));
            };
            if !plain(account) || !hexadecimal(handle) || !hexadecimal(token) {
                return Err(unread(
                    "the call carries a model account's placeholder that does not read",
                ));
            }
            return Ok(Some(Self {
                account: (*account).to_owned(),
                handle: (*handle).to_owned(),
                token: (*token).to_owned(),
                carrier,
            }));
        }
        Ok(None)
    }

    /// The account the draw is on.
    #[must_use]
    pub fn account(&self) -> &str {
        &self.account
    }
}

/// Lowercase hex of `bytes`.
fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut text, byte| {
        let _ = write!(text, "{byte:02x}");
        text
    })
}

/// A length-prefixed field, as the broker encodes one.
fn field(into: &mut Vec<u8>, bytes: &[u8]) -> Result<(), ProxyError> {
    let length = u32::try_from(bytes.len())
        .map_err(|_overflow| unread("a field of the presentation is too long to encode"))?;
    into.extend_from_slice(&length.to_be_bytes());
    into.extend_from_slice(bytes);
    Ok(())
}

/// The digest of one request as the broker reads it: its method, its path
/// with query, and the SHA-256 of its body.
fn request_digest(method: &str, path: &str, body: &[u8]) -> Result<[u8; 32], ProxyError> {
    let mut encoded = Vec::new();
    field(&mut encoded, REQUEST_DOMAIN.as_bytes())?;
    field(&mut encoded, method.as_bytes())?;
    field(&mut encoded, path.as_bytes())?;
    field(&mut encoded, &Sha256::digest(body))?;
    Ok(Sha256::digest(&encoded).into())
}

/// The presentation headers of `drawn` for the request `(method, path,
/// body)`, signed with `key` now under a fresh operation id: handle,
/// handle id, operation, signing time and signature, as the broker reads
/// them.
///
/// # Errors
///
/// `ModelAccount` when the random source or the clock fails.
pub fn presentation(
    drawn: &Drawn,
    (method, path, body): (&str, &str, &[u8]),
    key: &Ed25519Identity,
) -> Result<[(&'static str, String); 5], ProxyError> {
    let mut operation = [0_u8; 16];
    OsRng
        .try_fill_bytes(&mut operation)
        .map_err(|error| unread(&format!("the secure random source failed: {error}")))?;
    let signed_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|since| i64::try_from(since.as_millis()).ok())
        .ok_or_else(|| unread("the clock reads before the epoch"))?;
    let digest = request_digest(method, path, body)?;
    let mut payload = Vec::new();
    field(&mut payload, PRESENTATION_DOMAIN.as_bytes())?;
    field(&mut payload, drawn.handle.as_bytes())?;
    field(&mut payload, &operation)?;
    field(&mut payload, &signed_at.to_be_bytes())?;
    field(&mut payload, &digest)?;
    let signature = sign_attestation(&payload, key);
    Ok([
        ("lys-handle", drawn.token.clone()),
        ("lys-handle-id", drawn.handle.clone()),
        ("lys-operation", hex(&operation)),
        ("lys-signed-at", signed_at.to_string()),
        ("lys-presentation", hex(&signature.to_cose_bytes())),
    ])
}

/// Send `request`, a call carrying `drawn`, through `broker`: its body read
/// whole, the placeholder taken out, and the draw presented with this
/// proxy's key for exactly this request. The answer is passed back as it
/// came, and this proxy's log names a refusal by the broker or by the
/// provider.
///
/// # Errors
///
/// `ModelAccount` when the body cannot be read or a header cannot be
/// written, and `Upstream` when the broker cannot be reached.
pub async fn through_broker(
    upstream: &Upstream,
    broker: &Broker,
    drawn: &Drawn,
    rest: &str,
    request: Request<ProxyBody>,
) -> Result<Response<Incoming>, ProxyError> {
    let failed = |reason: String| ProxyError::ModelAccount {
        account: drawn.account.clone(),
        reason,
    };
    let (mut parts, body) = request.into_parts();
    let body: Bytes = body
        .collect()
        .await
        .map_err(|error| failed(format!("the call's body could not be read: {error}")))?
        .to_bytes();
    let path = format!("/{}{rest}", drawn.account);
    parts.headers.remove(drawn.carrier);
    for (name, value) in presentation(drawn, (parts.method.as_str(), &path, &body), &broker.key)? {
        let value = HeaderValue::from_str(&value)
            .map_err(|_value| failed(format!("the {name} header could not be written")))?;
        parts.headers.insert(HeaderName::from_static(name), value);
    }
    let body = Full::new(body)
        .map_err(|never: std::convert::Infallible| match never {})
        .boxed_unsync();
    let response = upstream
        .send(&broker.base, &path, Request::from_parts(parts, body))
        .await?;
    said(&drawn.account, &response);
    Ok(response)
}

/// Name in this proxy's log a refusal of a call on `account`: the broker's,
/// by the name it gave, or the provider's, of the token the broker put in.
fn said(account: &str, response: &Response<Incoming>) {
    let status = response.status();
    if let Some(refusal) = response
        .headers()
        .get(REFUSAL_HEADER)
        .and_then(|value| value.to_str().ok())
    {
        eprintln!(
            "lys-proxy: model_account_refused: the secrets broker refused a call on model account {account}: {refusal} ({status})"
        );
    } else if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
        eprintln!(
            "lys-proxy: model_account_token_refused: the provider refused the token sealed for model account {account} ({status}); the token may be revoked or expired, so register the account's token again"
        );
    }
}

#[cfg(test)]
#[path = "account_tests.rs"]
mod tests;
