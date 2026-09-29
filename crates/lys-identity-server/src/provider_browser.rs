//! A provider state and code alone never authenticate a browser. The start
//! sets a fresh host-only, HTTP-only cookie; only its SHA-256 digest is held
//! with the flow. The callback must present exactly one matching cookie.
use axum::http::{HeaderMap, header};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use rand::{TryRngCore, rngs::OsRng};
use sha2::{Digest, Sha256};

use crate::error::ServerError;

const COOKIE: &str = "lys_provider";

pub(crate) fn begin(secure: bool) -> Result<(String, [u8; 32]), ServerError> {
    let mut bytes = [0u8; 32];
    OsRng
        .try_fill_bytes(&mut bytes)
        .map_err(|error| ServerError::SignInFailed {
            reason: format!("the secure random source failed: {error}"),
        })?;
    let token = URL_SAFE_NO_PAD.encode(bytes);
    let digest = Sha256::digest(token.as_bytes()).into();
    let secure = if secure { "; Secure" } else { "" };
    Ok((
        format!(
            "{COOKIE}={token}; HttpOnly; SameSite=Lax; Path=/auth/v1/providers/callback{secure}"
        ),
        digest,
    ))
}

pub(crate) fn digest(headers: &HeaderMap) -> Result<[u8; 32], ServerError> {
    let mut found = None;
    for header in headers.get_all(header::COOKIE) {
        let Ok(header) = header.to_str() else {
            return Err(ServerError::SignInStateUnknown);
        };
        for pair in header.split(';') {
            if let Some((COOKIE, token)) = pair.trim().split_once('=') {
                if token.is_empty() || found.is_some() {
                    return Err(ServerError::SignInStateUnknown);
                }
                found = Some(Sha256::digest(token.as_bytes()).into());
            }
        }
    }
    found.ok_or(ServerError::SignInStateUnknown)
}

pub(crate) fn matches(expected: &[u8; 32], presented: &[u8; 32]) -> bool {
    expected
        .iter()
        .zip(presented)
        .fold(0u8, |difference, (a, b)| difference | (a ^ b))
        == 0
}
