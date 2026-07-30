//! `lys ca` subcommands — issue and verify Ed25519-rooted X.509 certificates.
//!
//! Issuance wraps [`lys_core::ca::CertificateAuthority`]: the issuer identity
//! at `--key` signs a certificate for a named subject, valid from now for the
//! window `--validity` (or `--validity-days`) asks for — the library's TTL
//! model, which takes a `Duration` and does not backdate. Sub-day windows are
//! expressible because short-lived scoped grants need them; see
//! [`crate::commands::duration`].
//! Capability claims, when supplied, are validated as JSON and embedded
//! byte-for-byte as a non-critical extension under the lys OID arc with
//! sub-component `1` (`1.3.6.1.4.1.66364.1`); the library carries them as
//! opaque DER and this CLI defines no further semantics. Certificates are
//! written as PEM, the X.509 interop norm.
//!
//! Two issuance paths are offered, and they mean different things. Without
//! `--request`, the library generates the subject keypair and discards the
//! private half: the certificate names a key nobody ever held, which is fine
//! for testing the plumbing and useless for concluding anything about a
//! holder. With `--request`, the subject presents a PKCS#10 request produced by
//! `lys ca request` (or `openssl req`) and self-signed by a key they already
//! control; that proof of possession is verified before issuance and the
//! certificate binds their key. The reported `subject_key_origin` says which
//! path produced the certificate, because the distinction is the whole
//! difference in what the certificate is evidence of.
//!
//! Invariants: the issuer key file must already exist — only `lys key
//! generate` creates key material — and the subject keypair the library
//! generates during issuance is discarded, never written to disk or printed;
//! only its public half is reported. `ca request` writes only a public
//! artifact: a request carries a public key and a signature, never the seed.
//! Verification failures collapse to one non-oracle message, mirroring `lys
//! verify`. Claims echoed by `ca verify` are printed verbatim only when free
//! of control characters; anything else is shown as hex, so certificate
//! contents can never inject terminal escape sequences into the verification
//! output.

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use lys_core::TrustError;
use lys_core::ca::{
    CertificateAuthority, LYS_OID_ARC, create_certificate_request, decode_extension,
    encode_extension, verify_certificate_chain_at,
};

use crate::commands::error::{CliError, CliResult};
use crate::commands::files::{read_file, write_file};
use crate::commands::hex::{hex_lower, parse_hex_32};
use crate::commands::key::load_identity;
use crate::commands::output::Emitter;
use crate::commands::pem;

/// Sub-component appended to [`LYS_OID_ARC`] for the CLI's capability-claims
/// extension. Part of the wire contract: certificates issued by this CLI
/// carry claims under `1.3.6.1.4.1.66364.1`, and `lys ca verify` reads them
/// back from the same OID.
const CAPABILITY_CLAIMS_COMPONENT: u64 = 1;

/// The full OID under which this CLI transports capability claims.
///
/// Shared with `lys inspect cert`, which reads claims back from the same OID
/// without verifying the certificate that carries them.
pub(crate) fn capability_claims_oid() -> Vec<u64> {
    let mut oid = LYS_OID_ARC.to_vec();
    oid.push(CAPABILITY_CLAIMS_COMPONENT);
    oid
}

/// Whether claim text can be echoed to a terminal verbatim.
///
/// Certificate contents are attacker-influenceable (any issuer under the
/// trusted key can embed arbitrary bytes, and JSON strings may carry raw
/// control characters), so anything containing control characters beyond
/// newline and tab — including ANSI escape sequences that could spoof the
/// surrounding verification output — falls back to hex.
///
/// Shared with `lys inspect cert`, which reads the same fields out of
/// certificates nothing has vouched for and so needs the identical screen.
pub(crate) fn is_terminal_safe(text: &str) -> bool {
    text.chars()
        .all(|character| !character.is_control() || character == '\n' || character == '\t')
}

/// What an issuance produced, independent of which path produced it.
struct Issuance {
    der_bytes: Vec<u8>,
    subject_public_key: [u8; 32],
    issuer_public_key: [u8; 32],
    fingerprint: [u8; 32],
    expires_at: DateTime<Utc>,
    /// Human-readable provenance of the subject key, reported so an operator
    /// can tell whether the certificate binds a key its holder proved control
    /// of or one this command minted and threw away.
    subject_key_origin: &'static str,
}

/// `lys ca request --key <path> --subject <name> --out <file>`.
///
/// Produces the holder's side of a certificate exchange: a PKCS#10 request
/// carrying the identity's public key, self-signed by that identity. The
/// signature is the proof of possession `lys ca issue --request` verifies. The
/// written file is public — it contains no private material.
///
/// # Errors
///
/// Returns [`CliError::KeyFileMissing`] if the identity key file does not
/// exist, [`CliError::Io`] if the request cannot be written, and
/// [`CliError::Trust`] if the subject is empty or the library cannot sign the
/// request.
pub fn request(key: &Path, subject: &str, out: &Path, json: bool) -> CliResult<()> {
    let identity = Arc::new(load_identity(key)?);
    let der = create_certificate_request(&identity, subject)?;

    let pem_text = pem::encode_certificate_request(&der);
    write_file(out, pem_text.as_bytes(), "certificate-signing request file")?;

    let mut emit = Emitter::new(json);
    emit.field(
        "certificate-signing request for subject",
        "subject",
        subject,
    );
    emit.field(
        "subject public key (ed25519)",
        "subject_public_key",
        hex_lower(&identity.public_key_bytes()),
    );
    emit.field("request written", "request_path", out.display().to_string());
    emit.note("give this to the authority; it carries no private key material");
    emit.finish();
    Ok(())
}

/// `lys ca issue --key <path> --subject <name> [--request <file>]
/// [--claims <file>] (--validity <window> | --validity-days <n>) --out <file>`.
///
/// `ttl` is the already-resolved validity window; the two flags are reconciled
/// in [`crate::commands::duration::validity_window`] so this function has one
/// notion of the window rather than two.
///
/// With `--request`, the subject key comes from the holder's PKCS#10 request
/// and is certified only after its proof of possession verifies; `--subject`
/// must equal the common name the request asked for. Without `--request`, a
/// subject keypair is generated here and its private half discarded.
///
/// # Errors
///
/// Returns [`CliError::KeyFileMissing`] if the issuer key file does not
/// exist, [`CliError::Io`] if the claims file, request file, or output
/// certificate cannot be read or written, [`CliError::ClaimsJsonParse`] if the
/// claims file is not valid JSON, [`CliError::PemParse`] if the request is not
/// a PEM `CERTIFICATE REQUEST` block, and [`CliError::Trust`] if the library
/// rejects the issuance parameters, rejects the request's proof of possession,
/// or signing fails.
pub fn issue(
    key: &Path,
    subject: &str,
    claims: Option<&Path>,
    ttl: Duration,
    out: &Path,
    request_path: Option<&Path>,
    json: bool,
) -> CliResult<()> {
    let identity = load_identity(key)?;

    let extensions = match claims {
        Some(claims_path) => {
            let claims_bytes = read_file(claims_path, "claims file")?;
            // Validate — but embed the original bytes verbatim, so the signed
            // extension is exactly what the operator reviewed on disk.
            serde_json::from_slice::<serde_json::Value>(&claims_bytes).map_err(|source| {
                CliError::ClaimsJsonParse {
                    path: claims_path.to_path_buf(),
                    source,
                }
            })?;
            vec![encode_extension(&capability_claims_oid(), claims_bytes)]
        }
        None => Vec::new(),
    };

    let authority = CertificateAuthority::new(identity);

    let issued = if let Some(path) = request_path {
        let pem_bytes = read_file(path, "certificate-signing request file")?;
        let request_der = pem::decode_certificate_request(&pem_bytes, path)?;
        let certified =
            authority.issue_certificate_for_request(&request_der, subject, ttl, extensions)?;
        Issuance {
            der_bytes: certified.der_bytes,
            subject_public_key: certified.subject_public_key,
            issuer_public_key: certified.issuer_public_key,
            fingerprint: certified.fingerprint,
            expires_at: certified.expires_at,
            subject_key_origin: "presented by the holder, proof of possession verified",
        }
    } else {
        // `generated` carries the freshly generated subject signing key; it is
        // deliberately never persisted or printed and drops here.
        let generated = authority.issue_certificate(subject, ttl, extensions)?;
        Issuance {
            der_bytes: generated.der_bytes,
            subject_public_key: generated.subject_verifying_key.to_bytes(),
            issuer_public_key: generated.issuer_public_key,
            fingerprint: generated.fingerprint,
            expires_at: generated.expires_at,
            subject_key_origin: "generated by the issuer and discarded — the holder never proved \
                                 possession",
        }
    };

    let pem_text = pem::encode_certificate(&issued.der_bytes);
    write_file(out, pem_text.as_bytes(), "certificate file")?;

    let mut emit = Emitter::new(json);
    emit.field("issued certificate for subject", "subject", subject);
    emit.field(
        "subject public key (ed25519)",
        "subject_public_key",
        hex_lower(&issued.subject_public_key),
    );
    emit.field(
        "subject key origin",
        "subject_key_origin",
        issued.subject_key_origin,
    );
    emit.field(
        "issuer public key (ed25519)",
        "issuer_public_key",
        hex_lower(&issued.issuer_public_key),
    );
    emit.field(
        "fingerprint (sha256)",
        "fingerprint",
        hex_lower(&issued.fingerprint),
    );
    emit.field(
        "expires at (rfc3339)",
        "expires_at",
        issued.expires_at.to_rfc3339(),
    );
    match claims {
        Some(claims_path) => emit.field(
            "capability claims embedded from",
            "capability_claims_path",
            claims_path.display().to_string(),
        ),
        None => emit.field("capability claims", "capability_claims", "none"),
    }
    emit.field(
        "certificate written",
        "certificate_path",
        out.display().to_string(),
    );
    emit.finish();
    Ok(())
}

/// `lys ca verify --cert <file> --issuer-public-key <hex> [--at <rfc3339>]`.
///
/// # Errors
///
/// Returns [`CliError::Io`] if the certificate file cannot be read,
/// [`CliError::PemParse`] if it is not a PEM `CERTIFICATE` block,
/// [`CliError::InvalidIssuerPublicKey`] / [`CliError::InvalidTimestamp`] for
/// malformed arguments, [`CliError::CertificateVerificationFailed`] — the
/// single non-oracle message — if any verification check rejects the
/// certificate, and [`CliError::Trust`] if the DER cannot be parsed as a
/// certificate at all.
pub fn verify(cert: &Path, issuer_public_key: &str, at: Option<&str>, json: bool) -> CliResult<()> {
    let pem_bytes = read_file(cert, "certificate file")?;
    let der = pem::decode_certificate(&pem_bytes, cert)?;
    let issuer = parse_hex_32(issuer_public_key).ok_or(CliError::InvalidIssuerPublicKey)?;
    let checked_at = match at {
        Some(value) => DateTime::parse_from_rfc3339(value)
            .map(|instant| instant.with_timezone(&Utc))
            .map_err(|source| CliError::InvalidTimestamp {
                value: value.to_string(),
                source,
            })?,
        None => Utc::now(),
    };

    match verify_certificate_chain_at(&der, &issuer, checked_at) {
        Ok(()) => {}
        // Non-oracle by design: every rejected check — signature, issuer
        // key, self-signature screen, validity window — surfaces as the one
        // indistinguishable message.
        Err(TrustError::CertificateVerification { .. }) => {
            return Err(CliError::CertificateVerificationFailed);
        }
        Err(other) => return Err(CliError::Trust(other)),
    }

    // Read claims only after verification succeeded, so nothing from an
    // unverified certificate is ever echoed.
    let claims = decode_extension(&der, &capability_claims_oid())?;

    let mut emit = Emitter::new(json);
    emit.flag("certificate verified", "verified");
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
    match claims {
        Some(bytes) => match String::from_utf8(bytes) {
            Ok(text) if is_terminal_safe(&text) => {
                emit.field("capability claims", "capability_claims", text);
            }
            // Non-UTF-8 or control characters (terminal escape injection):
            // echo the bytes as hex, never raw. The JSON key differs too, so
            // a consumer can tell it received hex rather than the claims
            // text and cannot mistake one encoding for the other.
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
    emit.finish();
    Ok(())
}

#[cfg(test)]
#[path = "ca_tests.rs"]
mod tests;
