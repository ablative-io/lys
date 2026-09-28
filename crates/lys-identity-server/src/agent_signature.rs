//! An agent signs in to a request by signing it with the key its capability
//! certificate names, so no credential of the agent's ever passes through
//! this service.
//!
//! The request carries one header, `lys-agent-signature`, of four words: the
//! agent's id, the signing time in milliseconds since the Unix epoch, a
//! nonce of at least sixteen bytes in hex, and a `COSE_Sign1` signature in
//! hex over the request's method, path, body digest, signing time and nonce.
//! The signature is accepted only when it verifies against the key of one of
//! the agent's certificates that is not withdrawn, the agent is active, the
//! signing time is within a minute of this service's clock, and the nonce was
//! not seen before within that minute. Every failure is the one refusal
//! `AgentSignatureRefused`, naming which check refused it and never which key
//! was tried.

use std::collections::BTreeMap;
use std::str::FromStr;
use std::sync::PoisonError;
use std::time::{SystemTime, UNIX_EPOCH};

use axum::http::HeaderMap;
use lys_core::attestation::verify_attestation_bytes_by_signer;
use lys_core::ca::certificate_subject_public_key;
use lys_identity::projection::Projection;
use lys_identity::{AgentId, IdentityId, LifecycleState};
use sha2::{Digest, Sha256};

use crate::error::ServerError;
use crate::routes::AppState;

/// The header an agent's signed request carries.
pub const HEADER: &str = "lys-agent-signature";

/// The domain every agent request signature is made under.
const DOMAIN: &str = "lys-identity/agent-request/v1";

/// How far a signing time may stand from this service's clock.
const WINDOW_MS: u64 = 60_000;

/// The fewest bytes a nonce carries.
const NONCE_MIN: usize = 16;

/// The nonces seen within the window, each with its signing time.
pub type Nonces = std::sync::Mutex<BTreeMap<String, u64>>;

fn refused(reason: &'static str) -> ServerError {
    ServerError::AgentSignatureRefused { reason }
}

/// The bytes an agent signs for a request.
pub fn payload(method: &str, path: &str, body: &[u8], signed_at_ms: u64, nonce: &str) -> Vec<u8> {
    let digest = hex(&Sha256::digest(body));
    format!("{DOMAIN}\n{method}\n{path}\n{digest}\n{signed_at_ms}\n{nonce}").into_bytes()
}

/// The agent a signed request is from; none when it carries no signature.
pub fn signed_agent(
    state: &AppState,
    directory: &Projection,
    headers: &HeaderMap,
    (method, path, body): (&str, &str, &[u8]),
) -> Result<Option<AgentId>, ServerError> {
    let Some(value) = headers.get(HEADER) else {
        return Ok(None);
    };
    let text = value
        .to_str()
        .map_err(|_unread| refused("the header is not text"))?;
    let words: Vec<&str> = text.split_ascii_whitespace().collect();
    let [agent, signed_at, nonce, signature] = words.as_slice() else {
        return Err(refused("the header is not four words"));
    };
    let agent =
        AgentId::from_str(agent).map_err(|_unread| refused("the agent id does not read"))?;
    let signed_at: u64 = signed_at
        .parse()
        .map_err(|_unread| refused("the signing time does not read"))?;
    let nonce_bytes = unhex(nonce).ok_or_else(|| refused("the nonce is not hex"))?;
    if nonce_bytes.len() < NONCE_MIN {
        return Err(refused("the nonce is shorter than sixteen bytes"));
    }
    let cose = unhex(signature).ok_or_else(|| refused("the signature is not hex"))?;
    let now = now_ms()?;
    if now.abs_diff(signed_at) > WINDOW_MS {
        return Err(refused(
            "the signing time is more than a minute from this service's clock",
        ));
    }
    let record = directory
        .record(IdentityId::Agent(agent))
        .ok_or_else(|| refused("the signature does not verify"))?;
    if record.state() != LifecycleState::Active {
        return Err(refused("the agent is not active"));
    }
    let signed = payload(method, path, body, signed_at, nonce);
    let keys = certified_keys(state, &agent.to_string())?;
    let verified = keys
        .iter()
        .any(|key| verify_attestation_bytes_by_signer(&cose, &signed, key).is_ok());
    if !verified {
        return Err(refused("the signature does not verify"));
    }
    let mut seen = state
        .agent_nonces
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    seen.retain(|_nonce, at| now.abs_diff(*at) <= WINDOW_MS);
    if seen.insert(nonce.to_ascii_lowercase(), signed_at).is_some() {
        return Err(refused("the nonce was already used"));
    }
    Ok(Some(agent))
}

/// The keys of `agent`'s certificates that are not withdrawn.
fn certified_keys(state: &AppState, agent: &str) -> Result<Vec<[u8; 32]>, ServerError> {
    let Some(store) = state.certificates.as_ref() else {
        return Err(refused("no certificate log is configured"));
    };
    let store = store.lock().unwrap_or_else(PoisonError::into_inner);
    Ok(store
        .certificates()
        .filter(|entered| entered.issued.agent == agent && entered.withdrawn.is_none())
        .filter_map(|entered| unbase64(&entered.issued.der))
        .filter_map(|der| certificate_subject_public_key(&der).ok())
        .collect())
}

fn now_ms() -> Result<u64, ServerError> {
    let elapsed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_before| refused("this service's clock is before the Unix epoch"))?;
    u64::try_from(elapsed.as_millis())
        .map_err(|_large| refused("this service's clock does not fit"))
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut text = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        text.push(char::from(DIGITS[usize::from(byte >> 4)]));
        text.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    text
}

fn unhex(text: &str) -> Option<Vec<u8>> {
    if text.len() % 2 != 0 {
        return None;
    }
    (0..text.len())
        .step_by(2)
        .map(|at| u8::from_str_radix(text.get(at..at + 2)?, 16).ok())
        .collect()
}

fn unbase64(text: &str) -> Option<Vec<u8>> {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = Vec::new();
    let mut buffer = 0_u32;
    let mut bits = 0_u32;
    for letter in text.bytes().filter(|letter| *letter != b'=') {
        let value = ALPHABET.iter().position(|known| *known == letter)?;
        buffer = (buffer << 6) | u32::try_from(value).ok()?;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push(u8::try_from((buffer >> bits) & 0xff).ok()?);
        }
    }
    Some(out)
}
