//! `lys ca` subcommands — issue and verify Ed25519-rooted X.509 certificates.
//!
//! Issuance wraps [`lys_core::ca::CertificateAuthority`]: the issuer identity
//! at `--key` signs a certificate for a named subject, valid from now for a
//! whole number of days (the library's TTL model — there is no backdating).
//! Capability claims, when supplied, are validated as JSON and embedded
//! byte-for-byte as a non-critical extension under the lys OID arc with
//! sub-component `1` (`1.3.6.1.4.1.58888.1`); the library carries them as
//! opaque DER and this CLI defines no further semantics. Certificates are
//! written as PEM, the X.509 interop norm.
//!
//! Invariants: the issuer key file must already exist — only `lys key
//! generate` creates key material — and the subject keypair the library
//! generates during issuance is discarded, never written to disk or printed;
//! only its public half is reported. Verification failures collapse to one
//! non-oracle message, mirroring `lys verify`.

use std::path::Path;
use std::time::Duration;

use chrono::{DateTime, Utc};
use lys_core::TrustError;
use lys_core::ca::{
    CertificateAuthority, LYS_OID_ARC, decode_extension, encode_extension,
    verify_certificate_chain_at,
};

use crate::commands::error::{CliError, CliResult};
use crate::commands::files::{read_file, write_file};
use crate::commands::hex::{hex_lower, parse_hex_32};
use crate::commands::key::load_identity;
use crate::commands::pem;

/// Sub-component appended to [`LYS_OID_ARC`] for the CLI's capability-claims
/// extension. Part of the wire contract: certificates issued by this CLI
/// carry claims under `1.3.6.1.4.1.58888.1`, and `lys ca verify` reads them
/// back from the same OID.
const CAPABILITY_CLAIMS_COMPONENT: u64 = 1;

/// Seconds in one day, for the `--validity-days` conversion.
const SECONDS_PER_DAY: u64 = 86_400;

/// The full OID under which this CLI transports capability claims.
fn capability_claims_oid() -> Vec<u64> {
    let mut oid = LYS_OID_ARC.to_vec();
    oid.push(CAPABILITY_CLAIMS_COMPONENT);
    oid
}

/// `lys ca issue --key <path> --subject <name> [--claims <file>]
/// --validity-days <n> --out <file>`.
///
/// # Errors
///
/// Returns [`CliError::KeyFileMissing`] if the issuer key file does not
/// exist, [`CliError::Io`] if the claims file cannot be read or the
/// certificate cannot be written, [`CliError::ClaimsJsonParse`] if the
/// claims file is not valid JSON, and [`CliError::Trust`] if the library
/// rejects the issuance parameters or signing fails.
pub fn issue(
    key: &Path,
    subject: &str,
    claims: Option<&Path>,
    validity_days: u32,
    out: &Path,
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

    let ttl = Duration::from_secs(u64::from(validity_days) * SECONDS_PER_DAY);
    let authority = CertificateAuthority::new(identity);
    // `issued` carries the freshly generated subject signing key; it is
    // deliberately never persisted or printed and drops with this binding.
    let issued = authority.issue_certificate(subject, ttl, extensions)?;

    let pem_text = pem::encode_certificate(&issued.der_bytes);
    write_file(out, pem_text.as_bytes(), "certificate file")?;

    println!("issued certificate for subject: {subject}");
    println!(
        "subject public key (ed25519): {}",
        hex_lower(&issued.subject_verifying_key.to_bytes())
    );
    println!(
        "issuer public key (ed25519): {}",
        hex_lower(&issued.issuer_public_key)
    );
    println!("fingerprint (sha256): {}", hex_lower(&issued.fingerprint));
    println!("expires at (rfc3339): {}", issued.expires_at.to_rfc3339());
    match claims {
        Some(claims_path) => println!("capability claims embedded from: {}", claims_path.display()),
        None => println!("capability claims: none"),
    }
    println!("certificate written: {}", out.display());
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
pub fn verify(cert: &Path, issuer_public_key: &str, at: Option<&str>) -> CliResult<()> {
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

    println!("certificate verified");
    println!("issuer public key (ed25519): {}", hex_lower(&issuer));
    println!("checked at (rfc3339): {}", checked_at.to_rfc3339());
    match claims {
        Some(bytes) => match String::from_utf8(bytes) {
            Ok(text) => println!("capability claims: {text}"),
            Err(non_utf8) => println!(
                "capability claims (hex): {}",
                hex_lower(non_utf8.as_bytes())
            ),
        },
        None => println!("capability claims: none"),
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn capability_claims_oid_extends_the_lys_arc_by_one() {
        let oid = capability_claims_oid();
        assert_eq!(oid, vec![1, 3, 6, 1, 4, 1, 58888, 1]);
        assert_eq!(&oid[..LYS_OID_ARC.len()], LYS_OID_ARC);
    }
}
