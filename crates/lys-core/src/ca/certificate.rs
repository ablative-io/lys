//! The two results a certificate-authority issuance can produce.
//!
//! [`IssuedCertificate`] comes from the path where the authority generated the
//! subject keypair, so it carries both halves. The subject signing key is
//! private material: the manual [`fmt::Debug`] impl redacts it unconditionally
//! and the type never derives `Debug` (which would expose key internals).
//!
//! [`CertifiedKey`] comes from the path where the holder presented a key and
//! proved possession of it. There is no private material to hold or redact —
//! its absence is the point, and it is why the two outcomes are separate types
//! rather than one type with an optional signing key. A caller cannot
//! accidentally treat a presented-key issuance as though it handed back a
//! usable private key.
//!
//! [`CertifiedKey::from_der_and_public_key`] re-parses the DER it is handed and
//! confirms it binds the presented key. Nothing else in the crate checks that,
//! so without it a certificate-generation bug could return a
//! successfully-signed certificate over the wrong key — a success report for
//! work not done, which is the failure class this crate exists to make
//! impossible. The binding is the entire value of the presented-key path, so it
//! is read back from the bytes rather than assumed.
//!
//! [`IssuedCertificate::from_der_and_keypair`] deliberately does **not** do the
//! same. It is published API whose accepted inputs cannot be narrowed without
//! breaking callers that hand it DER for their own purposes, and the property
//! matters less there: the subject key on that path came from this process, so
//! a certificate over the wrong key misbinds a key nobody controls rather than
//! misattributing one somebody does. The equivalent guarantee is asserted for
//! that path in the authority's tests instead.

use std::fmt;

use chrono::{DateTime, Utc};
use ed25519_dalek::pkcs8::DecodePrivateKey;
use ed25519_dalek::{SigningKey, VerifyingKey};
use sha2::{Digest, Sha256};
use x509_parser::oid_registry::OID_SIG_ED25519;
use x509_parser::prelude::{FromDer, X509Certificate};

use crate::error::{TrustError, TrustResult};
use crate::hex_lower;

/// A certificate issued by a [`crate::ca::CertificateAuthority`].
///
/// The subject keypair is generated during issuance and travels with the
/// certificate so the holder can prove possession of the subject identity.
/// The signing half is private — guard it accordingly; it is redacted from
/// `Debug` output.
pub struct IssuedCertificate {
    /// The signed certificate in DER encoding.
    pub der_bytes: Vec<u8>,
    /// The subject's Ed25519 signing (private) key. **Sensitive material** —
    /// redacted from `Debug`.
    pub subject_signing_key: SigningKey,
    /// The subject's Ed25519 verifying (public) key, matching
    /// [`Self::subject_signing_key`].
    pub subject_verifying_key: VerifyingKey,
    /// SHA-256 fingerprint of [`Self::der_bytes`].
    pub fingerprint: [u8; 32],
    /// Instant after which the certificate is no longer valid.
    pub expires_at: DateTime<Utc>,
    /// The 32-byte Ed25519 public key of the issuing authority.
    pub issuer_public_key: [u8; 32],
}

impl IssuedCertificate {
    /// Builds an [`IssuedCertificate`] from signed DER and the subject keypair.
    ///
    /// Computes the SHA-256 fingerprint over `der_bytes`, converts the rcgen
    /// Ed25519 subject keypair into dalek signing/verifying keys via its
    /// PKCS#8 serialization, and validates that the raw public key is exactly
    /// 32 bytes and consistent with the recovered signing key.
    ///
    /// # Errors
    ///
    /// Returns [`TrustError::CertificateGeneration`] if the subject public key
    /// is not 32 bytes, the PKCS#8 private key cannot be decoded into a dalek
    /// signing key, or the recovered public key does not match the keypair.
    pub fn from_der_and_keypair(
        der_bytes: Vec<u8>,
        subject_keypair: &rcgen::KeyPair,
        expires_at: DateTime<Utc>,
        issuer_public_key: [u8; 32],
    ) -> TrustResult<Self> {
        let raw_public = subject_keypair.public_key_raw();
        if raw_public.len() != 32 {
            return Err(TrustError::CertificateGeneration {
                reason: format!(
                    "subject public key must be 32 bytes for Ed25519, got {}",
                    raw_public.len()
                ),
            });
        }

        let pkcs8 = subject_keypair.serialize_der();
        let subject_signing_key =
            SigningKey::from_pkcs8_der(&pkcs8).map_err(|e| TrustError::CertificateGeneration {
                reason: format!("failed to load subject signing key from PKCS#8: {e}"),
            })?;
        let subject_verifying_key = subject_signing_key.verifying_key();

        if subject_verifying_key.to_bytes().as_slice() != raw_public {
            return Err(TrustError::CertificateGeneration {
                reason: "subject keypair public and private halves do not match".to_string(),
            });
        }

        let fingerprint: [u8; 32] = Sha256::digest(&der_bytes).into();

        Ok(Self {
            der_bytes,
            subject_signing_key,
            subject_verifying_key,
            fingerprint,
            expires_at,
            issuer_public_key,
        })
    }
}

/// A certificate issued over a subject key its holder presented and proved
/// possession of.
///
/// Produced by
/// [`CertificateAuthority::issue_certificate_for_request`](crate::ca::CertificateAuthority::issue_certificate_for_request).
/// Carries no private material: the subject's signing key stays with the
/// holder and was never present in this process. That is what makes this
/// certificate meaningful to a third party — it binds a key someone
/// demonstrably controls, rather than one the authority minted.
#[derive(Clone, PartialEq, Eq)]
pub struct CertifiedKey {
    /// The signed certificate in DER encoding.
    pub der_bytes: Vec<u8>,
    /// The subject's 32-byte Ed25519 public key, as carried in the
    /// certificate's `subjectPublicKeyInfo` and proved possessed at issuance.
    pub subject_public_key: [u8; 32],
    /// SHA-256 fingerprint of [`Self::der_bytes`].
    pub fingerprint: [u8; 32],
    /// Instant after which the certificate is no longer valid.
    pub expires_at: DateTime<Utc>,
    /// The 32-byte Ed25519 public key of the issuing authority.
    pub issuer_public_key: [u8; 32],
}

impl CertifiedKey {
    /// Builds a [`CertifiedKey`] from signed DER and the presented subject key.
    ///
    /// Computes the SHA-256 fingerprint over `der_bytes` and confirms that the
    /// certificate actually binds `subject_public_key` — see
    /// [`assert_der_binds_subject_key`].
    ///
    /// # Errors
    ///
    /// Returns [`TrustError::CertificateGeneration`] if the certificate cannot
    /// be parsed back, does not carry an Ed25519 subject key, or binds a key
    /// other than `subject_public_key`.
    pub fn from_der_and_public_key(
        der_bytes: Vec<u8>,
        subject_public_key: &[u8; 32],
        expires_at: DateTime<Utc>,
        issuer_public_key: [u8; 32],
    ) -> TrustResult<Self> {
        assert_der_binds_subject_key(&der_bytes, subject_public_key)?;
        let fingerprint: [u8; 32] = Sha256::digest(&der_bytes).into();

        Ok(Self {
            der_bytes,
            subject_public_key: *subject_public_key,
            fingerprint,
            expires_at,
            issuer_public_key,
        })
    }
}

impl fmt::Debug for CertifiedKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CertifiedKey")
            .field("der_bytes_len", &self.der_bytes.len())
            .field("subject_public_key", &hex_lower(&self.subject_public_key))
            .field("fingerprint", &hex_lower(&self.fingerprint))
            .field("expires_at", &self.expires_at)
            .field("issuer_public_key", &hex_lower(&self.issuer_public_key))
            .finish()
    }
}

/// Confirms that a freshly issued certificate's `subjectPublicKeyInfo` carries
/// exactly `expected` under the Ed25519 algorithm identifier.
///
/// This is a self-check on our own output, not a verification of a stranger's
/// artifact: it exists so that "issuance succeeded" cannot be reported for a
/// certificate that binds a different key than the one the caller asked to
/// certify. The whole value of the presented-key path is that the certified
/// key is the holder's, so that binding is the one thing worth re-reading from
/// the bytes rather than assuming.
fn assert_der_binds_subject_key(der_bytes: &[u8], expected: &[u8; 32]) -> TrustResult<()> {
    let (_, certificate) =
        X509Certificate::from_der(der_bytes).map_err(|e| TrustError::CertificateGeneration {
            reason: format!("freshly issued certificate could not be parsed back: {e:?}"),
        })?;

    let spki = &certificate.tbs_certificate.subject_pki;
    if spki.algorithm.algorithm != OID_SIG_ED25519 {
        return Err(TrustError::CertificateGeneration {
            reason: "freshly issued certificate does not carry an Ed25519 subject key".to_string(),
        });
    }
    if spki.subject_public_key.data.as_ref() != expected.as_slice() {
        return Err(TrustError::CertificateGeneration {
            reason: "freshly issued certificate binds a different subject key than requested"
                .to_string(),
        });
    }
    Ok(())
}

impl fmt::Debug for IssuedCertificate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("IssuedCertificate")
            .field("der_bytes_len", &self.der_bytes.len())
            .field("subject_signing_key", &"[REDACTED]")
            .field(
                "subject_verifying_key",
                &hex_lower(&self.subject_verifying_key.to_bytes()),
            )
            .field("fingerprint", &hex_lower(&self.fingerprint))
            .field("expires_at", &self.expires_at)
            .field("issuer_public_key", &hex_lower(&self.issuer_public_key))
            .finish()
    }
}

#[cfg(test)]
#[path = "certificate_tests.rs"]
mod tests;
