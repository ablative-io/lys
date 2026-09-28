//! Verifying a certificate against the issuer key that signed it: the
//! signature, the algorithm, the self-signed rule, and the validity window at
//! a given instant.
//!
//! Self-signed is judged by keys and signature, never by names: a certificate
//! is self-signed when its subject public key is the supplied issuer key and
//! its signature verifies under that key.

use chrono::{DateTime, Utc};
use ed25519_dalek::{Signature, VerifyingKey};
use x509_parser::oid_registry::OID_SIG_ED25519;
use x509_parser::prelude::{FromDer, X509Certificate};

use crate::ca::certificate::certificate_subject_public_key;
use crate::error::{TrustError, TrustResult};

/// Verifies a certificate's Ed25519 signature against an expected issuer key
/// and checks that `at` falls within the certificate's validity window.
///
/// The checks run in this order: `cert_der` parses as an X.509 certificate;
/// its signature algorithm is Ed25519; its signature is 64 bytes; the
/// supplied issuer key decodes to an Ed25519 point; the signature verifies
/// under that key with `ed25519-dalek` **strict** verification; the
/// certificate is not self-signed; and `at` lies inside its
/// `notBefore`/`notAfter` window (boundaries inclusive, per X.509).
/// x509-parser's `verify_signature` is deliberately not used — for Ed25519 it
/// routes to ring's non-strict verification.
///
/// Strict verification (`verify_strict`) rejects signature malleability and
/// small-order/torsion issuer keys, which plain `verify` accepts. This crate
/// is an audit trust foundation: non-repudiation requires that a certificate
/// has a unique valid signature under the issuer key, and weak keys — for
/// which signatures can be forged for arbitrary payloads — must be
/// categorically rejected.
///
/// A certificate is self-signed when its subject public key is the supplied
/// issuer key and its signature verifies under that key. Two keys are the
/// same key when both decode to Ed25519 points and the points are equal, so a
/// non-canonical encoding of the issuer's point is the issuer's key. The
/// judgement is made only after the signature has verified, so a certificate
/// whose signature fails is refused on its signature and never reported as
/// self-signed. A subject key that cannot be read as an Ed25519 key, or does
/// not decode to a point, is not the issuer's key. No name takes part: the
/// subject and issuer distinguished names neither refuse nor accept a
/// certificate.
///
/// What a successful verification proves, and what it leaves to the caller —
/// revocation, trust in the supplied key, possession of the subject key,
/// anything beyond one level, and the meaning of any extension — is stated
/// on [`verify_certificate_chain`].
///
/// # Errors
///
/// Returns [`TrustError::CertificateParsing`] if `cert_der` cannot be parsed,
/// and [`TrustError::CertificateVerification`] if the certificate is not
/// Ed25519-signed, carries a malformed signature or issuer key, the
/// signature does not strictly verify, the certificate is self-signed, or
/// `at` is outside the validity window (the reason distinguishes `expired`
/// from `not yet valid` and names the violated boundary instant).
///
/// [`verify_certificate_chain`]: super::verify_certificate_chain
pub fn verify_certificate_chain_at(
    cert_der: &[u8],
    issuer_public_key: &[u8; 32],
    at: DateTime<Utc>,
) -> TrustResult<()> {
    let (_, certificate) =
        X509Certificate::from_der(cert_der).map_err(|e| TrustError::CertificateParsing {
            reason: format!("failed to parse certificate DER: {e:?}"),
        })?;

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
            .map_err(|_err| TrustError::CertificateVerification {
                reason: format!(
                    "certificate signature must be 64 bytes for Ed25519, got {}",
                    signature_bytes.len()
                ),
            })?;
    let signature = Signature::from_bytes(signature_array);

    let verifying_key = VerifyingKey::from_bytes(issuer_public_key).map_err(|_err| {
        TrustError::CertificateVerification {
            reason: "issuer public key is not a valid Ed25519 point".to_string(),
        }
    })?;

    verifying_key
        .verify_strict(tbs, &signature)
        .map_err(|_err| TrustError::CertificateVerification {
            reason: "certificate signature did not verify against the issuer public key"
                .to_string(),
        })?;

    if subject_key_is_issuer_key(cert_der, issuer_public_key) {
        return Err(TrustError::CertificateVerification {
            reason: "self-signed certificate rejected (subject key is the issuer key)".to_string(),
        });
    }

    check_validity_window(&certificate, at)
}

/// Whether the certificate's subject public key is `issuer_public_key`.
///
/// The subject key is read as [`certificate_subject_public_key`] reads it.
/// One that cannot be read that way — another algorithm, parameters, unused
/// bits, a length other than 32 — is not the issuer's key, which is an
/// Ed25519 key by construction.
fn subject_key_is_issuer_key(cert_der: &[u8], issuer_public_key: &[u8; 32]) -> bool {
    certificate_subject_public_key(cert_der)
        .is_ok_and(|subject_key| super::is_same_ed25519_key(&subject_key, issuer_public_key))
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
