//! `lys inspect` — read-only viewers for attestation artifacts and
//! certificates.
//!
//! Both subcommands decode one local file and print its fields. They verify
//! **nothing**: no signature is checked, no issuer key is consulted, no
//! validity window is evaluated. Everything they print is therefore
//! attacker-chosen until the corresponding verifying command says otherwise —
//! `lys verify` for attestations, `lys ca verify` for certificates — so every
//! output opens with an `UNVERIFIED` banner naming that command, and
//! capability claims carry a second marker immediately before they are
//! echoed. Reading precedes trusting; these commands are the reading half and
//! must never be mistaken for the trusting half.
//!
//! # Invariants
//!
//! - **Local files only. Never expose these through a network service.**
//!   Decoding is strict — [`Attestation::from_cose_bytes`] rejects
//!   non-canonical artifacts — so a remote caller told whether a decode
//!   succeeded would hold a parsing oracle over bytes it chose. The exit
//!   code and the printed fields are for an operator looking at a file on
//!   their own disk, not a service response.
//! - **No verification, ever.** These commands must not grow a signature or
//!   issuer check: `lys verify` and `lys ca verify` are the only commands
//!   that pronounce on trust, and keeping the two semantics apart is the
//!   whole reason this viewer exists separately.
//! - **Failures reuse the existing non-oracle messages.** An undecodable
//!   artifact collapses into the same
//!   [`CliError::VerificationFailed`](crate::commands::error::CliError::VerificationFailed)
//!   message `lys verify` prints; no decode-specific detail is invented here.
//! - **Nothing is echoed raw unless it is terminal-safe.** Subject names and
//!   capability claims come from files nothing has vouched for, so they pass
//!   the same control-character screen `lys ca verify` applies before any
//!   byte reaches a terminal.

use std::path::Path;

use chrono::{DateTime, Utc};
use lys_core::TrustError;
use lys_core::attestation::Attestation;
use lys_core::ca::decode_extension;
use x509_parser::prelude::{FromDer, X509Certificate};

use crate::commands::ca::{capability_claims_oid, is_terminal_safe};
use crate::commands::error::{CliError, CliResult};
use crate::commands::files::read_file;
use crate::commands::hex::hex_lower;
use crate::commands::output::Emitter;
use crate::commands::pem;

/// Opening line of `lys inspect attestation`, naming the command that does
/// check the signature.
const ATTESTATION_BANNER: &str = "UNVERIFIED — fields read from the artifact; the signature was \
                                  NOT checked. Run 'lys verify' to verify.";

/// Opening line of `lys inspect cert`, naming the command that does check the
/// certificate against an issuer.
const CERTIFICATE_BANNER: &str = "UNVERIFIED — this certificate has NOT been checked against any \
                                  issuer. Run 'lys ca verify' to verify.";

/// Printed immediately before any capability claims are echoed, so claims can
/// never be read without first being labelled unverified.
const CLAIMS_MARKER: &str = "UNVERIFIED CLAIMS — anyone can put anything here; trustworthy only \
                             after 'lys ca verify':";

/// Shown in place of an RFC 3339 rendering when the source value is outside
/// the representable date range.
const UNREPRESENTABLE_INSTANT: &str = "out of representable range";

/// `lys inspect attestation --attestation <file>`.
///
/// Decodes a tagged `COSE_Sign1` artifact and prints its fields. **The
/// signature is not checked** — every value printed is unverified, which the
/// first line of the output says. Run `lys verify --attestation <file>
/// --payload <file>` to establish that the fields mean anything.
///
/// Local files only: never offer this through a network service. Decoding is
/// canonical-strict, so telling a remote caller whether a decode succeeded
/// would hand it a parsing oracle over bytes it supplied.
///
/// # Errors
///
/// Returns [`CliError::Io`] if the artifact file cannot be read,
/// [`CliError::VerificationFailed`] — the same single non-oracle message
/// `lys verify` uses — if the bytes are not a canonical artifact, and
/// [`CliError::Trust`] for any other library failure. There is deliberately
/// no distinct decode error.
pub fn attestation(attestation_path: &Path, json: bool) -> CliResult<()> {
    let artifact_bytes = read_file(attestation_path, "attestation file")?;
    // Decode before printing anything: a rejected artifact must produce the
    // generic failure and no fields at all, banner included.
    let attestation = match Attestation::from_cose_bytes(&artifact_bytes) {
        Ok(attestation) => attestation,
        Err(TrustError::InvalidSignature) => return Err(CliError::VerificationFailed),
        Err(other) => return Err(CliError::Trust(other)),
    };

    let signed_at = render_millis(attestation.timestamp);
    let mut emit = Emitter::new(json);
    // The banner is prose, but a machine consumer must not be able to
    // mistake this for verified data — so in JSON it becomes an explicit
    // `verified: false` plus the warning text, not a dropped sentence.
    emit.note(ATTESTATION_BANNER);
    if emit.is_json() {
        emit.field("verified", "verified", false);
        emit.field("warning", "warning", ATTESTATION_BANNER);
    }
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
    emit.field("signed at (rfc3339)", "signed_at_rfc3339", signed_at);
    emit.finish();
    Ok(())
}

/// `lys inspect cert --cert <file>`.
///
/// Decodes a PEM certificate and prints its subject, validity window, subject
/// public key, and any capability claims. **Nothing is verified** — no issuer
/// key is consulted, no signature is checked, and the validity window is
/// printed rather than enforced. Run `lys ca verify --cert <file>
/// --issuer-public-key <hex>` to establish that any of it means anything.
///
/// Local files only: never offer this through a network service, for the same
/// oracle reason as [`attestation`].
///
/// # Errors
///
/// Returns [`CliError::Io`] if the certificate file cannot be read,
/// [`CliError::PemParse`] if it is not a PEM `CERTIFICATE` block or its body
/// is not a parseable X.509 certificate, and [`CliError::Trust`] if the
/// claims extension cannot be read back.
pub fn cert(cert_path: &Path, json: bool) -> CliResult<()> {
    let pem_bytes = read_file(cert_path, "certificate file")?;
    let der = pem::decode_certificate(&pem_bytes, cert_path)?;
    let certificate = parse_certificate(&der, cert_path)?;

    let subject = certificate
        .subject()
        .iter_common_name()
        .next()
        .and_then(|attribute| attribute.as_str().ok());
    let validity = certificate.validity();
    let not_before = render_seconds(validity.not_before.timestamp());
    let not_after = render_seconds(validity.not_after.timestamp());
    let subject_key = hex_lower(&certificate.public_key().subject_public_key.data);

    let mut emit = Emitter::new(json);
    emit.note(CERTIFICATE_BANNER);
    if emit.is_json() {
        emit.field("verified", "verified", false);
        emit.field("warning", "warning", CERTIFICATE_BANNER);
    }
    match subject {
        Some(name) if is_terminal_safe(name) => emit.field("subject", "subject", name),
        // The subject of an unverified certificate is attacker-chosen text:
        // control characters (terminal escape injection) go out as hex, under
        // a distinct key so a consumer cannot mistake hex for the name.
        Some(unsafe_name) => emit.field(
            "subject (hex)",
            "subject_hex",
            hex_lower(unsafe_name.as_bytes()),
        ),
        None => emit.field("subject", "subject", "none"),
    }
    emit.field("not before (rfc3339)", "not_before", not_before);
    emit.field("not after (rfc3339)", "not_after", not_after);
    emit.field(
        "subject public key (ed25519)",
        "subject_public_key",
        subject_key,
    );

    match decode_extension(&der, &capability_claims_oid())? {
        Some(bytes) => {
            // The marker precedes the claims unconditionally — no reader ever
            // meets the claims before being told they are worthless. In JSON
            // it rides as a field for the same reason.
            emit.note(CLAIMS_MARKER);
            if emit.is_json() {
                emit.field(
                    "capability claims warning",
                    "capability_claims_warning",
                    CLAIMS_MARKER,
                );
            }
            match String::from_utf8(bytes) {
                Ok(text) if is_terminal_safe(&text) => {
                    emit.field("capability claims", "capability_claims", text);
                }
                // Non-UTF-8 or control characters: echo the bytes as hex,
                // never raw, exactly as `lys ca verify` does.
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
            }
        }
        None => emit.field("capability claims", "capability_claims", "none"),
    }
    emit.finish();
    Ok(())
}

/// Parses certificate DER, mapping a rejection onto the certificate file's
/// existing PEM-parse error so the failure names the path without inventing a
/// new error kind.
fn parse_certificate<'a>(der: &'a [u8], path: &Path) -> CliResult<X509Certificate<'a>> {
    X509Certificate::from_der(der)
        .map(|(_, certificate)| certificate)
        .map_err(|error| CliError::PemParse {
            path: path.to_path_buf(),
            reason: format!("certificate body is not a valid X.509 certificate: {error:?}"),
        })
}

/// Renders a Unix-millisecond timestamp as RFC 3339.
///
/// The timestamp is an unverified, attacker-chosen field, so an unrepresentable
/// value yields [`UNREPRESENTABLE_INSTANT`] rather than a panic.
fn render_millis(timestamp: i64) -> String {
    match DateTime::<Utc>::from_timestamp_millis(timestamp) {
        Some(instant) => instant.to_rfc3339(),
        None => UNREPRESENTABLE_INSTANT.to_string(),
    }
}

/// Renders a Unix-second timestamp (the ASN.1 validity encoding) as RFC 3339.
///
/// Certificate validity bounds are unverified, attacker-chosen fields, so an
/// unrepresentable value yields [`UNREPRESENTABLE_INSTANT`] rather than a
/// panic.
fn render_seconds(timestamp: i64) -> String {
    match DateTime::<Utc>::from_timestamp(timestamp, 0) {
        Some(instant) => instant.to_rfc3339(),
        None => UNREPRESENTABLE_INSTANT.to_string(),
    }
}
