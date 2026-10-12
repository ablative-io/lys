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
use rcgen::{
    BasicConstraints, Certificate, CertificateParams, CustomExtension, IsCa, KeyPair,
    KeyUsagePurpose, PKCS_ED25519,
};
use time::OffsetDateTime;

use crate::ca::certificate::{CertifiedKey, IssuedCertificate};
use crate::ca::rcgen_bridge::{IdentitySigner, PresentedKey, distinguished_name};
use crate::ca::request::verify_certificate_request;
use crate::clock::{Clock, ClockSource};
use crate::error::{TrustError, TrustResult};
use crate::hex_lower;
use crate::keys::Ed25519Identity;

mod verify;

pub use verify::{verify_certificate_chain, verify_certificate_chain_at};

/// Issues X.509 certificates signed by an Ed25519 root identity.
#[derive(Debug)]
pub struct CertificateAuthority {
    identity: Arc<Ed25519Identity>,
    clock: ClockSource,
}

impl CertificateAuthority {
    /// Wraps an [`Ed25519Identity`] as a certificate authority.
    #[must_use]
    pub fn new(identity: Ed25519Identity) -> Self {
        Self {
            identity: Arc::new(identity),
            clock: ClockSource::System,
        }
    }

    /// Wraps an identity with an instance-owned certificate creation clock.
    ///
    /// Verification continues to use its separately supplied instant.
    #[must_use]
    pub fn with_clock(identity: Ed25519Identity, clock: Arc<dyn Clock>) -> Self {
        Self {
            identity: Arc::new(identity),
            clock: ClockSource::Supplied(clock),
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
    /// fails, creation time is unavailable, or rcgen cannot build or sign the
    /// certificate.
    pub fn issue_certificate(
        &self,
        subject: &str,
        ttl: Duration,
        extensions: Vec<CustomExtension>,
    ) -> TrustResult<IssuedCertificate> {
        validate_issuance(subject, ttl)?;
        let (issued_at, expires_at) = validity_window(ttl, self.creation_time()?)?;

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
    /// of representable range, creation time is unavailable, or rcgen cannot
    /// build or sign the certificate.
    pub fn issue_certificate_for_request(
        &self,
        request_der: &[u8],
        subject: &str,
        ttl: Duration,
        extensions: Vec<CustomExtension>,
    ) -> TrustResult<CertifiedKey> {
        validate_issuance(subject, ttl)?;
        let (issued_at, expires_at) = validity_window(ttl, self.creation_time()?)?;

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

    /// The DER of this authority's self-signed issuer certificate, whose
    /// subject is the issuer name every certificate it issues carries.
    ///
    /// It lets a third party check an issued certificate with standard X.509
    /// tooling, such as `openssl verify -CAfile`, holding nothing from lys.
    /// It is public: it carries the issuer's public key and a signature, never
    /// the seed.
    ///
    /// # Errors
    ///
    /// Returns [`TrustError::CertificateGeneration`] if the issuer
    /// certificate cannot be built or signed, or creation time is unavailable.
    pub fn issuer_certificate_der(&self) -> TrustResult<Vec<u8>> {
        let issuer_key = self.issuer_key_pair()?;
        Ok(self.issuer_certificate(&issuer_key)?.der().to_vec())
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

    /// Builds the self-signed issuer certificate whose subject DN is derived
    /// from this authority's public key.
    ///
    /// It is the trust anchor [`Self::issuer_certificate_der`] hands out, so
    /// every field that matters to a relying party is set explicitly rather
    /// than left to rcgen's defaults:
    ///
    /// - basic constraints: a CA with path length 0, so it can sign end-entity
    ///   certificates and nothing that could itself sign;
    /// - key usage, marked critical: `keyCertSign` and `cRLSign` only;
    /// - validity: from the moment it is built until [`ISSUER_NOT_AFTER`].
    ///
    /// None of this reaches the certificates it signs: rcgen takes only the
    /// issuer's distinguished name and key identifier method from it.
    fn issuer_certificate(&self, issuer_key: &KeyPair) -> TrustResult<Certificate> {
        let created_at = self.creation_time()?;
        let mut params = CertificateParams::new(Vec::<String>::new()).map_err(|e| {
            TrustError::CertificateGeneration {
                reason: format!("failed to build issuer parameters: {e}"),
            }
        })?;
        let common_name = hex_lower(&self.identity.public_key_bytes());
        params.distinguished_name = distinguished_name(&common_name);
        params.is_ca = IsCa::Ca(BasicConstraints::Constrained(0));
        params.key_usages = vec![KeyUsagePurpose::KeyCertSign, KeyUsagePurpose::CrlSign];
        params.not_before = to_offset_date_time(created_at)?;
        params.not_after = OffsetDateTime::from_unix_timestamp(ISSUER_NOT_AFTER).map_err(|e| {
            TrustError::CertificateGeneration {
                reason: format!("issuer certificate notAfter is out of range: {e}"),
            }
        })?;
        params
            .self_signed(issuer_key)
            .map_err(|e| TrustError::CertificateGeneration {
                reason: format!("failed to build issuer certificate: {e}"),
            })
    }

    fn creation_time(&self) -> TrustResult<DateTime<Utc>> {
        self.clock.now().map_err(|error| TrustError::CertificateGeneration {
            reason: format!("certificate creation {error}"),
        })
    }
}

/// The issuer certificate's `notAfter`, in seconds since the Unix epoch:
/// `99991231235959Z`, the value RFC 5280 section 4.1.2.5 gives a certificate
/// with no well-defined expiration date.
///
/// `lys ca issue` puts no ceiling on a certificate's window of its own; the
/// only ceiling is the encoding, since `GeneralizedTime` carries a four-digit
/// year, so no certificate this authority signs can be valid past this
/// instant. An issuer that expired first would make `openssl verify` refuse
/// certificates that are still inside their own windows. The issuer stays
/// bounded in practice by the certificates it signs, each of which carries
/// its own window.
const ISSUER_NOT_AFTER: i64 = 253_402_300_799;

/// Rejects malformed inputs before consulting a creation clock.
fn validate_issuance(subject: &str, ttl: Duration) -> TrustResult<()> {
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

    Ok(())
}

/// Computes the shared validity window from an explicit creation instant.
fn validity_window(
    ttl: Duration,
    issued_at: DateTime<Utc>,
) -> TrustResult<(DateTime<Utc>, DateTime<Utc>)> {
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
