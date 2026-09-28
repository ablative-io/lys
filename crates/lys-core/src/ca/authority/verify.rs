//! Verifying a certificate against the issuer key that signed it: the
//! signature, the algorithm, and the validity window at a given instant.

use chrono::{DateTime, Utc};
use ed25519_dalek::{Signature, VerifyingKey};
use x509_parser::oid_registry::OID_SIG_ED25519;
use x509_parser::prelude::{FromDer, X509Certificate};

use crate::error::{TrustError, TrustResult};

/// Verifies a certificate's Ed25519 signature against an expected issuer key
/// and checks the validity window at the current time (`Utc::now()`).
///
/// Thin wrapper over [`verify_certificate_chain_at`]; see it for the full
/// list of checks.
///
/// # Errors
///
/// See [`verify_certificate_chain_at`].
pub fn verify_certificate_chain(cert_der: &[u8], issuer_public_key: &[u8; 32]) -> TrustResult<()> {
    verify_certificate_chain_at(cert_der, issuer_public_key, Utc::now())
}

/// Verifies a certificate's Ed25519 signature against an expected issuer key
/// and checks that `at` falls within the certificate's validity window.
///
/// Parses `cert_der`, rejects self-signed certificates (issuer equal to
/// subject), confirms the signature algorithm is Ed25519, recovers the
/// to-be-signed DER and 64-byte signature, verifies the signature with
/// `ed25519-dalek` **strict** verification, and finally rejects the
/// certificate if `at` lies outside its `notBefore`/`notAfter` window
/// (boundaries inclusive, per X.509). x509-parser's `verify_signature` is
/// deliberately not used — for Ed25519 it routes to ring's non-strict
/// verification.
///
/// Strict verification (`verify_strict`) rejects signature malleability and
/// small-order/torsion issuer keys, which plain `verify` accepts. This crate
/// is an audit trust foundation: non-repudiation requires that a certificate
/// has a unique valid signature under the issuer key, and weak keys — for
/// which signatures can be forged for arbitrary payloads — must be
/// categorically rejected.
///
/// The self-signed rejection compares the raw subject and issuer DN bytes.
/// That is a heuristic defence-in-depth screen, not a security boundary —
/// the Ed25519 signature check against the caller-supplied issuer key is the
/// real boundary. The heuristic has a known false positive: a certificate
/// legitimately issued by the authority for a caller-chosen subject equal to
/// the authority's hex-pubkey common name is rejected here even though its
/// signature would verify.
///
/// This function says nothing about who controls the certificate's *subject*
/// key. It verifies that this issuer signed this certificate. Concluding that
/// the subject key is held by the named subject additionally requires that the
/// certificate was issued through
/// [`CertificateAuthority::issue_certificate_for_request`], where possession
/// was proven at issuance time.
///
/// # Errors
///
/// Returns [`TrustError::CertificateParsing`] if `cert_der` cannot be parsed,
/// and [`TrustError::CertificateVerification`] if the certificate is
/// self-signed, is not Ed25519-signed, carries a malformed signature or
/// issuer key, the signature does not strictly verify, or `at` is outside
/// the validity window (the reason distinguishes `expired` from
/// `not yet valid` and names the violated boundary instant).
///
/// [`CertificateAuthority::issue_certificate_for_request`]: super::CertificateAuthority::issue_certificate_for_request
pub fn verify_certificate_chain_at(
    cert_der: &[u8],
    issuer_public_key: &[u8; 32],
    at: DateTime<Utc>,
) -> TrustResult<()> {
    let (_, certificate) =
        X509Certificate::from_der(cert_der).map_err(|e| TrustError::CertificateParsing {
            reason: format!("failed to parse certificate DER: {e:?}"),
        })?;

    // Heuristic screen only — see the rustdoc above. The signature check
    // below is the actual security boundary.
    if certificate.subject().as_raw() == certificate.issuer().as_raw() {
        return Err(TrustError::CertificateVerification {
            reason: "self-signed certificate rejected (issuer equals subject)".to_string(),
        });
    }

    if certificate.signature_algorithm.algorithm != OID_SIG_ED25519 {
        return Err(TrustError::CertificateVerification {
            reason: "certificate signature algorithm is not Ed25519".to_string(),
        });
    }

    let tbs = certificate.tbs_certificate.as_ref();
    let signature_bytes: &[u8] = &certificate.signature_value.data;
    let signature_array: &[u8; 64] =
        signature_bytes
            .try_into()
            .ok()
            .ok_or_else(|| TrustError::CertificateVerification {
                reason: format!(
                    "certificate signature must be 64 bytes for Ed25519, got {}",
                    signature_bytes.len()
                ),
            })?;
    let signature = Signature::from_bytes(signature_array);

    let verifying_key =
        VerifyingKey::from_bytes(issuer_public_key)
            .ok()
            .ok_or_else(|| TrustError::CertificateVerification {
                reason: "issuer public key is not a valid Ed25519 point".to_string(),
            })?;

    verifying_key
        .verify_strict(tbs, &signature)
        .ok()
        .ok_or_else(|| TrustError::CertificateVerification {
            reason: "certificate signature did not verify against the issuer public key"
                .to_string(),
        })?;

    check_validity_window(&certificate, at)
}

/// Rejects `at` instants outside the certificate's `notBefore`/`notAfter`
/// window (boundaries inclusive, per X.509 semantics).
fn check_validity_window(certificate: &X509Certificate<'_>, at: DateTime<Utc>) -> TrustResult<()> {
    let validity = certificate.validity();
    let not_before = datetime_from_asn1_timestamp(validity.not_before.timestamp(), "notBefore")?;
    let not_after = datetime_from_asn1_timestamp(validity.not_after.timestamp(), "notAfter")?;

    if at < not_before {
        return Err(TrustError::CertificateVerification {
            reason: format!(
                "certificate not yet valid: notBefore is {not_before}, checked at {at}"
            ),
        });
    }
    if at > not_after {
        return Err(TrustError::CertificateVerification {
            reason: format!("certificate expired: notAfter was {not_after}, checked at {at}"),
        });
    }
    Ok(())
}

/// Converts an ASN.1 validity timestamp (seconds since the Unix epoch) into a
/// chrono UTC instant.
fn datetime_from_asn1_timestamp(timestamp: i64, field: &str) -> TrustResult<DateTime<Utc>> {
    DateTime::<Utc>::from_timestamp(timestamp, 0).ok_or_else(|| TrustError::CertificateParsing {
        reason: format!(
            "certificate {field} timestamp {timestamp} is outside the representable date range"
        ),
    })
}
