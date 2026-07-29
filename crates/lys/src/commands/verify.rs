//! `lys verify` — verify a `COSE_Sign1` attestation artifact against a
//! payload, optionally against the certificate that vouches for its signer.
//!
//! Reads the raw artifact bytes written by `lys attest`, re-reads the
//! candidate payload, and delegates to
//! [`lys_core::attestation::verify_attestation_bytes`]. Success prints the
//! verified attestation details and exits 0; any failure — malformed or
//! non-canonical artifact, tampered payload, tampered timestamp, forged
//! signature, wrong signer key — exits 1 with a single indistinguishable
//! message, matching the library's deliberate non-oracle behaviour. There
//! is no separate parse error for the attestation file: an unparseable
//! artifact is indistinguishable from an unverifiable one by design.
//!
//! # The join, with `--cert`
//!
//! On its own, this command answers "some key signed this payload" and prints
//! which key. That is a weaker statement than people read it as. Verifying a
//! certificate proves an authority issued it; verifying an attestation proves a
//! key signed a payload. **Neither says the two concern the same identity** —
//! and a reader comparing two hex strings by eye is a reader who will
//! eventually not.
//!
//! `--cert` with `--issuer-public-key` closes that: the certificate must verify
//! against the trusted issuer at the checked instant, the attestation must
//! verify against the payload, **and** the attestation's signer key must equal
//! the key the certificate certifies. Only then are the certificate's
//! capability claims printed — so the sentence a counterparty actually wants,
//! "this named subject, holding these capabilities, made this statement",
//! becomes one command with one exit code.
//!
//! This is only meaningful for certificates issued over a presented key (`lys
//! ca issue --request`). A certificate whose subject keypair was generated at
//! issuance binds a key nobody ever held, so nothing can ever match it and the
//! join fails closed — correctly, since there is no identity there to attest.
//!
//! The composition lives here rather than in `lys-core` on purpose: it is two
//! library verifications and an equality check, and the library's own composed
//! verifier is the one the anchor's verification bundle will need. Designing
//! that API now, before the bundle format is settled, would freeze a guess.

use std::path::Path;

use chrono::{DateTime, Utc};
use lys_core::TrustError;
use lys_core::attestation::{Attestation, verify_attestation_bytes};
use lys_core::ca::{certificate_subject_public_key, decode_extension, verify_certificate_chain_at};

use crate::commands::ca::{capability_claims_oid, is_terminal_safe};
use crate::commands::error::{CliError, CliResult};
use crate::commands::files::read_file;
use crate::commands::hex::{hex_lower, parse_hex_32};
use crate::commands::output::Emitter;
use crate::commands::pem;

/// `lys verify --attestation <file> --payload <file> [--cert <file>
/// --issuer-public-key <hex>] [--at <rfc3339>]`.
///
/// # Errors
///
/// Returns [`CliError::Io`] if any file cannot be read,
/// [`CliError::VerificationFailed`] — the single generic message — if the
/// artifact is malformed or non-canonical, the payload does not match, or
/// the signature is invalid, and [`CliError::Trust`] for any other library
/// failure.
///
/// With `--cert`, returns [`CliError::PemParse`] if the certificate is not a
/// PEM `CERTIFICATE` block, [`CliError::InvalidIssuerPublicKey`] or
/// [`CliError::InvalidTimestamp`] for malformed arguments, and
/// [`CliError::CertifiedVerificationFailed`] — one message covering all three
/// halves — if the certificate, the attestation, or the binding between them
/// fails.
pub fn run(
    attestation_path: &Path,
    payload_path: &Path,
    cert: Option<&Path>,
    issuer_public_key: Option<&str>,
    at: Option<&str>,
    json: bool,
) -> CliResult<()> {
    let artifact_bytes = read_file(attestation_path, "attestation file")?;
    let payload = read_file(payload_path, "payload file")?;

    // Clap guarantees these arrive together; treat a missing pair as the
    // uncertified path rather than assuming, so a future flag rearrangement
    // cannot silently drop the certificate half of the check.
    match (cert, issuer_public_key) {
        (Some(cert_path), Some(issuer_hex)) => {
            verify_certified(&artifact_bytes, &payload, cert_path, issuer_hex, at, json)
        }
        _ => verify_attestation_only(&artifact_bytes, &payload, json),
    }
}

/// Verifies the attestation alone, reporting the signer key without claiming
/// anything about who holds it.
fn verify_attestation_only(artifact_bytes: &[u8], payload: &[u8], json: bool) -> CliResult<()> {
    match verify_attestation_bytes(artifact_bytes, payload) {
        Ok(attestation) => {
            let mut emit = Emitter::new(json);
            emit.flag("attestation verified", "verified");
            emit_attestation_fields(&mut emit, &attestation);
            emit.note(
                "this proves a key signed this payload, not who holds that key — pass --cert \
                 with --issuer-public-key to check the certificate that vouches for it",
            );
            emit.finish();
            Ok(())
        }
        Err(TrustError::InvalidSignature) => Err(CliError::VerificationFailed),
        Err(other) => Err(CliError::Trust(other)),
    }
}

/// Verifies the certificate, the attestation, and that both concern one key.
fn verify_certified(
    artifact_bytes: &[u8],
    payload: &[u8],
    cert_path: &Path,
    issuer_hex: &str,
    at: Option<&str>,
    json: bool,
) -> CliResult<()> {
    let pem_bytes = read_file(cert_path, "certificate file")?;
    let cert_der = pem::decode_certificate(&pem_bytes, cert_path)?;
    let issuer = parse_hex_32(issuer_hex).ok_or(CliError::InvalidIssuerPublicKey)?;
    let checked_at = match at {
        Some(value) => DateTime::parse_from_rfc3339(value)
            .map(|instant| instant.with_timezone(&Utc))
            .map_err(|source| CliError::InvalidTimestamp {
                value: value.to_string(),
                source,
            })?,
        None => Utc::now(),
    };

    // Every rejection below collapses to one message. Argument-shape problems
    // above do not — a malformed hex issuer key is the caller's typo, not a
    // verification result, and hiding it would only waste their time.
    match verify_certificate_chain_at(&cert_der, &issuer, checked_at) {
        Ok(()) => {}
        Err(TrustError::CertificateVerification { .. }) => {
            return Err(CliError::CertifiedVerificationFailed);
        }
        Err(other) => return Err(CliError::Trust(other)),
    }

    let attestation = match verify_attestation_bytes(artifact_bytes, payload) {
        Ok(attestation) => attestation,
        Err(TrustError::InvalidSignature) => return Err(CliError::CertifiedVerificationFailed),
        Err(other) => return Err(CliError::Trust(other)),
    };

    // The join. Both halves verified independently above and would each report
    // success on their own; this is the only check that makes them one
    // statement rather than two unrelated true facts.
    let certified_key = certificate_subject_public_key(&cert_der)?;
    if certified_key != attestation.signer_public_key {
        return Err(CliError::CertifiedVerificationFailed);
    }

    // Claims are read only now — nothing from a certificate is echoed before
    // the certificate, the attestation, and the binding have all passed.
    let claims = decode_extension(&cert_der, &capability_claims_oid())?;

    let mut emit = Emitter::new(json);
    emit.flag("certified attestation verified", "verified");
    emit.flag(
        "attestation signer is the key this certificate certifies",
        "signer_matches_certificate",
    );
    emit_attestation_fields(&mut emit, &attestation);
    emit.field(
        "issuer public key (ed25519)",
        "issuer_public_key",
        hex_lower(&issuer),
    );
    emit.field(
        "checked at (rfc3339)",
        "checked_at",
        checked_at.to_rfc3339(),
    );
    emit_claims(&mut emit, claims);
    emit.finish();
    Ok(())
}

/// Emits the fields common to both paths.
fn emit_attestation_fields(emit: &mut Emitter, attestation: &Attestation) {
    emit.field(
        "signer public key (ed25519)",
        "signer_public_key",
        hex_lower(&attestation.signer_public_key),
    );
    emit.field(
        "payload hash (sha256)",
        "payload_hash",
        hex_lower(&attestation.payload_hash),
    );
    emit.field(
        "signed at (unix ms)",
        "signed_at_unix_ms",
        attestation.timestamp,
    );
}

/// Emits capability claims, falling back to hex for anything a terminal must
/// not be handed verbatim — the same screen `lys ca verify` applies, since the
/// bytes come from the same attacker-influenceable place.
fn emit_claims(emit: &mut Emitter, claims: Option<Vec<u8>>) {
    match claims {
        Some(bytes) => match String::from_utf8(bytes) {
            Ok(text) if is_terminal_safe(&text) => {
                emit.field("capability claims", "capability_claims", text);
            }
            Ok(unsafe_text) => emit.field(
                "capability claims (hex)",
                "capability_claims_hex",
                hex_lower(unsafe_text.as_bytes()),
            ),
            Err(non_utf8) => emit.field(
                "capability claims (hex)",
                "capability_claims_hex",
                hex_lower(non_utf8.as_bytes()),
            ),
        },
        None => emit.field("capability claims", "capability_claims", "none"),
    }
}
