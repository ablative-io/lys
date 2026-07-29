//! [`CertificateAuthority`] — Ed25519-rooted X.509 issuance and chain
//! verification.
//!
//! The authority wraps an [`Ed25519Identity`] and issues short-lived X.509
//! certificates for arbitrary subjects. All certificates use Ed25519
//! exclusively, and the certificate is signed by the authority's Ed25519 key,
//! surfaced to rcgen through a [`RemoteKeyPair`](rcgen::RemoteKeyPair) adapter
//! so the authority's private seed is never exposed.
//!
//! # Two issuance paths, one certificate shape
//!
//! [`CertificateAuthority::issue_certificate`] generates the subject keypair
//! itself and returns both halves. That is convenient, but the resulting
//! certificate binds a key the *authority* produced: it says nothing about
//! what the holder controls, so anything layered on top of it — logging
//! issuance transparently, gating a write path on a recognised chain —
//! authenticates the authority's willingness to issue rather than the
//! identity of the subject.
//!
//! [`CertificateAuthority::issue_certificate_for_request`] is the path that
//! carries a real binding: the holder presents a PKCS#10 request self-signed
//! by a key they already hold, that proof of possession is verified (see
//! [`super::request`]), and the certificate is signed over the presented key.
//! The authority never sees the private half.
//!
//! Both paths build their certificate through the same `leaf_params` and
//! `validity_window` helpers, so the two cannot drift into issuing
//! differently shaped certificates.
//!
//! Chain verification is performed directly with `ed25519-dalek`:
//! x509-parser is used only to parse the certificate and recover its
//! to-be-signed bytes and signature. x509-parser's own `verify_signature` is
//! never called — for Ed25519 it routes to ring's non-strict verification,
//! which accepts small-order keys and malleable signatures.

use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Timelike, Utc};
use ed25519_dalek::{Signature, VerifyingKey};
use rcgen::{
    BasicConstraints, Certificate, CertificateParams, CustomExtension, IsCa, KeyPair, PKCS_ED25519,
};
use time::OffsetDateTime;
use x509_parser::oid_registry::OID_SIG_ED25519;
use x509_parser::prelude::{FromDer, X509Certificate};

use crate::ca::certificate::{CertifiedKey, IssuedCertificate};
use crate::ca::rcgen_bridge::{IdentitySigner, PresentedKey, distinguished_name};
use crate::ca::request::verify_certificate_request;
use crate::error::{TrustError, TrustResult};
use crate::hex_lower;
use crate::keys::Ed25519Identity;

/// Issues X.509 certificates signed by an Ed25519 root identity.
#[derive(Debug)]
pub struct CertificateAuthority {
    identity: Arc<Ed25519Identity>,
}

impl CertificateAuthority {
    /// Wraps an [`Ed25519Identity`] as a certificate authority.
    pub fn new(identity: Ed25519Identity) -> Self {
        Self {
            identity: Arc::new(identity),
        }
    }

    /// Returns the authority's 32-byte Ed25519 public key.
    pub fn public_key_bytes(&self) -> [u8; 32] {
        self.identity.public_key_bytes()
    }

    /// Issues a certificate for `subject`, valid for `ttl` from now, carrying
    /// the supplied non-critical custom extensions.
    ///
    /// A fresh Ed25519 subject keypair is generated for the certificate and
    /// returned with it. The certificate is signed by this authority's Ed25519
    /// key; the issuer distinguished name is derived from the authority's
    /// public key so the issued certificate's issuer field is tied to this
    /// authority.
    ///
    /// Because the subject key originates here, the certificate binds a key
    /// the holder never proved control of. Where that binding matters — any
    /// use where a verifier must conclude something about the *subject* rather
    /// than about this authority — use
    /// [`Self::issue_certificate_for_request`] instead.
    ///
    /// # Errors
    ///
    /// Returns [`TrustError::CertificateGeneration`] if `subject` is empty,
    /// `ttl` is zero or out of representable range, subject key generation
    /// fails, or rcgen cannot build or sign the certificate.
    pub fn issue_certificate(
        &self,
        subject: &str,
        ttl: Duration,
        extensions: Vec<CustomExtension>,
    ) -> TrustResult<IssuedCertificate> {
        let (issued_at, expires_at) = validity_window(subject, ttl)?;

        let issuer_key = self.issuer_key_pair()?;
        let issuer_cert = self.issuer_certificate(&issuer_key)?;

        let subject_key = KeyPair::generate_for(&PKCS_ED25519).map_err(|e| {
            TrustError::CertificateGeneration {
                reason: format!("failed to generate Ed25519 subject keypair: {e}"),
            }
        })?;

        let params = leaf_params(subject, issued_at, expires_at, extensions)?;
        let certificate = params
            .signed_by(&subject_key, &issuer_cert, &issuer_key)
            .map_err(|e| TrustError::CertificateGeneration {
                reason: format!("failed to sign certificate: {e}"),
            })?;

        IssuedCertificate::from_der_and_keypair(
            certificate.der().to_vec(),
            &subject_key,
            expires_at,
            self.identity.public_key_bytes(),
        )
    }

    /// Issues a certificate over the subject key carried by a PKCS#10
    /// certificate-signing request, after verifying the request's proof of
    /// possession.
    ///
    /// This is the issuance path that produces a certificate a verifier can
    /// reason about: the subject key is one the holder demonstrably controls,
    /// because the request is self-signed by it and that signature is checked
    /// with strict Ed25519 verification before anything is issued. No private
    /// subject material exists here, which is why the result is a
    /// [`CertifiedKey`] rather than an [`IssuedCertificate`].
    ///
    /// `subject` is the name **this authority** chooses to certify, and it must
    /// equal the common name the request asked for. Requiring both to agree
    /// keeps two distinct properties: the holder cannot name themselves, since
    /// the authority supplies the name it will vouch for; and the authority
    /// cannot certify a holder under a name the holder never asked for, since
    /// a mismatch is refused. Every other certificate field — validity window,
    /// extensions, basic constraints — comes from this authority's inputs and
    /// never from the request; a request carrying requested extensions is
    /// rejected outright by [`verify_certificate_request`].
    ///
    /// # Errors
    ///
    /// Returns [`TrustError::CertificateParsing`] if `request_der` is not a
    /// well-formed Ed25519 PKCS#10 request,
    /// [`TrustError::CertificateVerification`] if its proof of possession does
    /// not verify, and [`TrustError::CertificateGeneration`] if `subject` is
    /// empty, disagrees with the request's common name, `ttl` is zero or out
    /// of representable range, or rcgen cannot build or sign the certificate.
    pub fn issue_certificate_for_request(
        &self,
        request_der: &[u8],
        subject: &str,
        ttl: Duration,
        extensions: Vec<CustomExtension>,
    ) -> TrustResult<CertifiedKey> {
        let (issued_at, expires_at) = validity_window(subject, ttl)?;

        let request = verify_certificate_request(request_der)?;
        if request.common_name() != subject {
            return Err(TrustError::CertificateGeneration {
                reason: format!(
                    "certificate-signing request asks for common name {:?} but issuance was \
                     requested for subject {subject:?}",
                    request.common_name()
                ),
            });
        }

        let issuer_key = self.issuer_key_pair()?;
        let issuer_cert = self.issuer_certificate(&issuer_key)?;

        let presented = PresentedKey::new(*request.subject_public_key());
        let params = leaf_params(subject, issued_at, expires_at, extensions)?;
        let certificate = params
            .signed_by(&presented, &issuer_cert, &issuer_key)
            .map_err(|e| TrustError::CertificateGeneration {
                reason: format!("failed to sign certificate: {e}"),
            })?;

        CertifiedKey::from_der_and_public_key(
            certificate.der().to_vec(),
            request.subject_public_key(),
            expires_at,
            self.identity.public_key_bytes(),
        )
    }

    /// Verifies that `cert_der` was issued by this authority and is within
    /// its validity window at the current time.
    ///
    /// Convenience over the free [`verify_certificate_chain`] using this
    /// authority's public key as the expected issuer. The validity window is
    /// evaluated against `Utc::now()`; use the free
    /// [`verify_certificate_chain_at`] to verify at an explicit instant.
    ///
    /// # Errors
    ///
    /// See [`verify_certificate_chain`].
    pub fn verify_certificate_chain(&self, cert_der: &[u8]) -> TrustResult<()> {
        verify_certificate_chain(cert_der, &self.identity.public_key_bytes())
    }

    /// Builds an rcgen [`KeyPair`] backed by this authority's identity through
    /// a [`RemoteKeyPair`](rcgen::RemoteKeyPair) adapter, so the private seed
    /// is never serialised.
    fn issuer_key_pair(&self) -> TrustResult<KeyPair> {
        let remote = IdentitySigner::new(Arc::clone(&self.identity));
        KeyPair::from_remote(Box::new(remote)).map_err(|e| TrustError::CertificateGeneration {
            reason: format!("failed to construct issuer key from identity: {e}"),
        })
    }

    /// Builds the self-signed in-memory issuer certificate whose subject DN is
    /// derived from this authority's public key.
    fn issuer_certificate(&self, issuer_key: &KeyPair) -> TrustResult<Certificate> {
        let mut params = CertificateParams::new(Vec::<String>::new()).map_err(|e| {
            TrustError::CertificateGeneration {
                reason: format!("failed to build issuer parameters: {e}"),
            }
        })?;
        let common_name = hex_lower(&self.identity.public_key_bytes());
        params.distinguished_name = distinguished_name(&common_name);
        params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        params
            .self_signed(issuer_key)
            .map_err(|e| TrustError::CertificateGeneration {
                reason: format!("failed to build issuer certificate: {e}"),
            })
    }
}

/// Validates the issuance inputs and computes the certificate's validity
/// window as `(notBefore, notAfter)`.
///
/// Shared by both issuance paths so a certificate's validity semantics cannot
/// differ depending on where its subject key came from.
fn validity_window(subject: &str, ttl: Duration) -> TrustResult<(DateTime<Utc>, DateTime<Utc>)> {
    if subject.trim().is_empty() {
        return Err(TrustError::CertificateGeneration {
            reason: "certificate subject must not be empty".to_string(),
        });
    }
    if ttl.is_zero() {
        return Err(TrustError::CertificateGeneration {
            reason: "certificate TTL must be positive".to_string(),
        });
    }

    let issued_at = Utc::now();
    let ttl = chrono::Duration::from_std(ttl).map_err(|e| TrustError::CertificateGeneration {
        reason: format!("certificate TTL is out of representable range: {e}"),
    })?;
    let expires_at =
        issued_at
            .checked_add_signed(ttl)
            .ok_or_else(|| TrustError::CertificateGeneration {
                reason: "certificate expiry overflowed the supported date range".to_string(),
            })?;
    // The DER `notAfter` is encoded at whole-second granularity (see
    // `to_offset_date_time`), so truncate the reported expiry to the same
    // instant — otherwise `expires_at` could run up to a second past the
    // certificate's actual validity.
    let expires_at =
        expires_at
            .with_nanosecond(0)
            .ok_or_else(|| TrustError::CertificateGeneration {
                reason: "certificate expiry could not be truncated to whole seconds".to_string(),
            })?;

    Ok((issued_at, expires_at))
}

/// Builds the leaf certificate parameters shared by both issuance paths.
///
/// Every field here is chosen by the authority. Nothing a certificate-signing
/// request carries reaches these parameters — the request contributes only the
/// subject public key, which is supplied separately to `signed_by`.
fn leaf_params(
    subject: &str,
    issued_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
    extensions: Vec<CustomExtension>,
) -> TrustResult<CertificateParams> {
    let mut params = CertificateParams::new(Vec::<String>::new()).map_err(|e| {
        TrustError::CertificateGeneration {
            reason: format!("failed to build certificate parameters: {e}"),
        }
    })?;
    params.distinguished_name = distinguished_name(subject);
    params.is_ca = IsCa::ExplicitNoCa;
    params.not_before = to_offset_date_time(issued_at)?;
    params.not_after = to_offset_date_time(expires_at)?;
    params.custom_extensions = extensions;
    Ok(params)
}

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

/// Converts a chrono UTC instant into the `time` type rcgen's validity fields
/// require, preserving second-granularity.
fn to_offset_date_time(instant: DateTime<Utc>) -> TrustResult<OffsetDateTime> {
    OffsetDateTime::from_unix_timestamp(instant.timestamp()).map_err(|e| {
        TrustError::CertificateGeneration {
            reason: format!("certificate validity instant is out of range: {e}"),
        }
    })
}

#[cfg(test)]
#[path = "authority_tests.rs"]
mod tests;
