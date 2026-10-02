//! Each run Lys starts sits on a seat: a key Lys makes for that run alone,
//! delegated from the AI's certificate key, which never leaves Lys. The
//! runner holds the seat key and signs the run's pass with it as it writes
//! the AI's config; every call on that pass must carry the `lys-seat` header,
//! and is refused unless the AI's standing certificate delegated the seat for
//! that session, the delegation has not ended, and the seat signed that pass.

use std::str::FromStr;

use axum::http::HeaderMap;
use lys_core::Ed25519Identity;
use lys_core::attestation::{sign_attestation, verify_attestation_bytes_by_signer};
use lys_home::harness::lys_mcp::{SEAT_HEADER, Seat, delegation_bytes, hex, pass_bytes, unhex};
use lys_identity::AgentId;
use rand::{TryRngCore, rngs::OsRng};
use zeroize::Zeroizing;

use crate::certificate_keys::SigningCertificate;
use crate::certificates_issue::{VALID_FOR, with_store};
use crate::error::ServerError;
use crate::routes::AppState;
use crate::session::now;

fn refused(reason: &str) -> ServerError {
    ServerError::AgentPassRefused {
        reason: format!("the run's seat {reason}"),
    }
}

fn unavailable(reason: impl std::fmt::Display) -> ServerError {
    ServerError::CertificatesUnavailable {
        reason: reason.to_string(),
    }
}

/// A seat for `agent`'s run `session`, delegated by the key of its standing
/// certificate `serial`; none when this install keeps no certificate log.
pub(crate) fn seat(
    state: &AppState,
    agent: AgentId,
    serial: &str,
    session: &str,
) -> Result<Option<Seat>, ServerError> {
    if state.certificates.is_none() {
        return Ok(None);
    }
    let name = agent.to_string();
    let (path, issued_at) = with_store(state, |store| {
        let entered = store
            .certificate(serial)
            .filter(|entered| entered.issued.agent == name && entered.withdrawn.is_none())
            .ok_or_else(|| {
                unavailable("the certificate a seat is delegated from does not stand")
            })?;
        Ok((store.agent_key(&name, serial), entered.issued.issued_at))
    })?;
    let path = path.ok_or_else(|| unavailable("the certificate's key is kept nowhere"))?;
    let certificate = Ed25519Identity::load(&path).map_err(unavailable)?;
    let mut seed = Zeroizing::new([0_u8; 32]);
    OsRng
        .try_fill_bytes(&mut *seed)
        .map_err(|error| unavailable(format!("random source failed: {error}")))?;
    let seat_key = Ed25519Identity::from_seed(&seed);
    // The certificate's own end: it is issued for VALID_FOR from issued_at
    // (certificates_issue), so the two change together or not at all.
    let not_after = issued_at.saturating_add(VALID_FOR.as_secs());
    let delegated = delegation_bytes(
        &name,
        serial,
        session,
        &seat_key.public_key_bytes(),
        not_after,
    );
    Ok(Some(Seat {
        agent: name,
        session: session.to_owned(),
        serial: serial.to_owned(),
        not_after,
        delegation: hex(&sign_attestation(&delegated, &certificate).to_cose_bytes()),
        key: hex(seed.as_slice()),
    }))
}

/// Checks the `lys-seat` header of a call on `agent`'s pass for `session`.
/// An install that keeps no certificate log seats no run, and checks none.
pub(crate) fn check(
    state: &AppState,
    headers: &HeaderMap,
    agent: AgentId,
    session: &str,
    pass: &str,
) -> Result<(), ServerError> {
    if state.certificates.is_none() {
        return Ok(());
    }
    if headers.get_all(SEAT_HEADER).iter().count() != 1 {
        return Err(refused("is not named by exactly one lys-seat header"));
    }
    let text = headers
        .get(SEAT_HEADER)
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| refused("header is not text"))?;
    let words: Vec<&str> = text.split(' ').collect();
    let [serial, not_after, public, delegation, signature] = words.as_slice() else {
        return Err(refused("header is not five words"));
    };
    let not_after = u64::from_str(not_after).map_err(|_unread| refused("end does not read"))?;
    if not_after <= now() {
        return Err(refused("delegation has ended"));
    }
    let public: [u8; 32] = unhex(public)
        .and_then(|bytes| bytes.try_into().ok())
        .ok_or_else(|| refused("key does not read"))?;
    let delegation = unhex(delegation).ok_or_else(|| refused("delegation does not read"))?;
    let signature = unhex(signature).ok_or_else(|| refused("signature does not read"))?;
    let name = agent.to_string();
    let der = with_store(state, |store| {
        Ok(store
            .certificate(serial)
            .filter(|entered| entered.issued.agent == name && entered.withdrawn.is_none())
            .map(|entered| entered.issued.der.clone()))
    })?
    .ok_or_else(|| refused("names no standing certificate of this AI"))?;
    // Decoded after the store lock is released, as every certificate is.
    let key = SigningCertificate::new(&der)
        .key_at(now())?
        .ok_or_else(|| refused("names a certificate that is not valid now"))?;
    let delegated = delegation_bytes(&name, serial, session, &public, not_after);
    verify_attestation_bytes_by_signer(&delegation, &delegated, &key).map_err(|_wrong| {
        refused("was not delegated by the certificate it names, for this AI and this run")
    })?;
    verify_attestation_bytes_by_signer(&signature, &pass_bytes(&name, session, pass), &public)
        .map_err(|_wrong| refused("did not sign this pass"))?;
    Ok(())
}
