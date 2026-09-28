//! The three leaves of a certificate log and the revocation signature.
//!
//! Every leaf begins with the ASCII domain tag
//! `lys-identity/certificate-log/v1`, one zero byte and one kind byte, 33
//! bytes together. The body that follows is fixed by the kind:
//!
//! - `0x01`, an issuance leaf: the certificate's DER to the end of the leaf,
//!   with no length prefix, at least one byte;
//! - `0x02`, a revocation leaf: the 32-byte SHA-256 of the revoked
//!   certificate's DER, then a 64-byte Ed25519 signature; the leaf is exactly
//!   129 bytes long;
//! - `0x03`, an attestation entry: the 32-byte SHA-256 of the certificate's
//!   DER, then the attestation's COSE bytes to the end of the leaf, with no
//!   length prefix, at least one byte.
//!
//! A certificate is named by the SHA-256 of its DER, computed from the bytes
//! supplied and never from a serial or a name the issuer chose. Anything else
//! is `certificate_leaf_malformed` and decodes to nothing. There is no claim
//! selector and no reinstatement kind: the leaf layer carries nothing the
//! contract does not lay out.
//!
//! A revocation is signed over `lys-identity/certificate-revocation/v1`, one
//! zero byte, the SHA-256 of the log's origin and the certificate hash, so a
//! revocation for one log does not verify in another and these bytes are never
//! the signed bytes of any other lys format.

use std::fmt;

use lys_core::Ed25519Identity;
use sha2::{Digest, Sha256};

use super::error::RevocationError;

/// The domain tag every certificate-log leaf begins with, before its zero byte.
pub const LEAF_DOMAIN_TAG: &[u8; 31] = b"lys-identity/certificate-log/v1";

/// The domain tag a revocation's signed bytes begin with, before their zero byte.
pub const REVOCATION_DOMAIN_TAG: &[u8; 38] = b"lys-identity/certificate-revocation/v1";

/// The exact length of a revocation leaf: the header, the hash and the signature.
pub const REVOCATION_LEAF_LEN: usize = HEADER_LEN + 32 + 64;

/// The tag, its zero byte and the kind byte.
const HEADER_LEN: usize = LEAF_DOMAIN_TAG.len() + 1 + 1;

const KIND_ISSUANCE: u8 = 0x01;
const KIND_REVOCATION: u8 = 0x02;
const KIND_ATTESTATION_ENTRY: u8 = 0x03;

/// The SHA-256 of a certificate's DER: the name a certificate log knows it by.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CertificateHash(pub [u8; 32]);

impl CertificateHash {
    /// The hash of `der`, computed from these bytes and nothing else.
    pub fn of_der(der: &[u8]) -> Self {
        Self(Sha256::digest(der).into())
    }

    /// The 32 hash bytes.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for CertificateHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CertificateHash({})", hex(&self.0))
    }
}

impl fmt::Display for CertificateHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&hex(&self.0))
    }
}

/// A revocation of one certificate, signed by the issuing authority for one
/// log's origin.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Revocation {
    /// The hash of the revoked certificate's DER.
    pub certificate: CertificateHash,
    /// The issuing authority's Ed25519 signature over
    /// [`revocation_signed_bytes`].
    pub signature: [u8; 64],
}

impl fmt::Debug for Revocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Revocation")
            .field("certificate", &self.certificate)
            .field("signature", &hex(&self.signature))
            .finish()
    }
}

impl Revocation {
    /// Signs the revocation of `certificate` for the log whose origin is
    /// `origin`, with the issuing authority's identity.
    pub fn sign(origin: &str, certificate: CertificateHash, issuer: &Ed25519Identity) -> Self {
        let signature = issuer.sign(&revocation_signed_bytes(origin, certificate));
        Self {
            certificate,
            signature,
        }
    }

    /// Checks the signature under `issuer_public_key` over the signed bytes
    /// rebuilt from this revocation and `origin`.
    ///
    /// # Errors
    ///
    /// [`RevocationError::RevocationNotSignedByIssuer`] when the signature does
    /// not verify: it was made by another key, for another log's origin or
    /// over another certificate's hash.
    pub fn verify(
        &self,
        origin: &str,
        issuer_public_key: &[u8; 32],
    ) -> Result<(), RevocationError> {
        let signed = revocation_signed_bytes(origin, self.certificate);
        if Ed25519Identity::verify(issuer_public_key, &signed, &self.signature).is_ok() {
            Ok(())
        } else {
            Err(RevocationError::RevocationNotSignedByIssuer)
        }
    }
}

/// The bytes a revocation signature covers: the revocation domain tag, one
/// zero byte, the SHA-256 of the origin's UTF-8 bytes and the certificate
/// hash, with no length prefix anywhere.
pub fn revocation_signed_bytes(origin: &str, certificate: CertificateHash) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(REVOCATION_DOMAIN_TAG.len() + 1 + 32 + 32);
    bytes.extend_from_slice(REVOCATION_DOMAIN_TAG);
    bytes.push(0);
    bytes.extend_from_slice(&Sha256::digest(origin.as_bytes()));
    bytes.extend_from_slice(certificate.as_bytes());
    bytes
}

/// One leaf of a certificate log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CertificateLeaf {
    /// Kind `0x01`: a certificate the issuing authority issued.
    Issuance {
        /// The certificate's DER, at least one byte.
        der: Vec<u8>,
    },
    /// Kind `0x02`: a revocation signed by the issuing authority.
    Revocation(Revocation),
    /// Kind `0x03`: an attestation by the certificate's key, placed in log order.
    AttestationEntry {
        /// The hash of the certificate whose key made the attestation.
        certificate: CertificateHash,
        /// The attestation's COSE bytes, at least one byte.
        cose: Vec<u8>,
    },
}

impl CertificateLeaf {
    /// The kind byte of this leaf.
    pub fn kind(&self) -> u8 {
        match self {
            Self::Issuance { .. } => KIND_ISSUANCE,
            Self::Revocation(..) => KIND_REVOCATION,
            Self::AttestationEntry { .. } => KIND_ATTESTATION_ENTRY,
        }
    }

    /// The leaf's bytes: the header, then the body the kind fixes.
    pub fn encode(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(HEADER_LEN + self.body_len());
        bytes.extend_from_slice(LEAF_DOMAIN_TAG);
        bytes.push(0);
        bytes.push(self.kind());
        match self {
            Self::Issuance { der } => bytes.extend_from_slice(der),
            Self::Revocation(revocation) => {
                bytes.extend_from_slice(revocation.certificate.as_bytes());
                bytes.extend_from_slice(&revocation.signature);
            }
            Self::AttestationEntry { certificate, cose } => {
                bytes.extend_from_slice(certificate.as_bytes());
                bytes.extend_from_slice(cose);
            }
        }
        bytes
    }

    fn body_len(&self) -> usize {
        match self {
            Self::Issuance { der } => der.len(),
            Self::Revocation(..) => 32 + 64,
            Self::AttestationEntry { cose, .. } => 32 + cose.len(),
        }
    }

    /// Reads a leaf from its bytes.
    ///
    /// # Errors
    ///
    /// [`RevocationError::CertificateLeafMalformed`] naming the reason for a
    /// wrong domain tag, a missing zero byte, an unknown kind byte, a
    /// revocation leaf that is not exactly [`REVOCATION_LEAF_LEN`] bytes long,
    /// an issuance leaf with no DER byte and an attestation entry with no COSE
    /// byte after its hash. Nothing partial is returned.
    pub fn decode(bytes: &[u8]) -> Result<Self, RevocationError> {
        let tag_len = LEAF_DOMAIN_TAG.len();
        if bytes.len() < tag_len || &bytes[..tag_len] != LEAF_DOMAIN_TAG {
            return Err(malformed(
                "the leaf does not begin with the domain tag lys-identity/certificate-log/v1",
            ));
        }
        if bytes.get(tag_len) != Some(&0) {
            return Err(malformed("no zero byte follows the domain tag"));
        }
        let Some(&kind) = bytes.get(tag_len + 1) else {
            return Err(malformed("no kind byte follows the zero byte"));
        };
        let body = &bytes[HEADER_LEN..];
        match kind {
            KIND_ISSUANCE => {
                if body.is_empty() {
                    return Err(malformed("an issuance leaf carries no DER byte"));
                }
                Ok(Self::Issuance { der: body.to_vec() })
            }
            KIND_REVOCATION => {
                if bytes.len() != REVOCATION_LEAF_LEN {
                    return Err(malformed("a revocation leaf is exactly 129 bytes long"));
                }
                let certificate = CertificateHash(
                    array(&body[..32])
                        .ok_or_else(|| malformed("a revocation leaf carries a 32-byte hash"))?,
                );
                let signature = array(&body[32..])
                    .ok_or_else(|| malformed("a revocation leaf carries a 64-byte signature"))?;
                Ok(Self::Revocation(Revocation {
                    certificate,
                    signature,
                }))
            }
            KIND_ATTESTATION_ENTRY => {
                if body.len() <= 32 {
                    return Err(malformed(
                        "an attestation entry carries no COSE byte after its hash",
                    ));
                }
                let certificate = CertificateHash(
                    array(&body[..32])
                        .ok_or_else(|| malformed("an attestation entry carries a 32-byte hash"))?,
                );
                Ok(Self::AttestationEntry {
                    certificate,
                    cose: body[32..].to_vec(),
                })
            }
            _ => Err(malformed(
                "the kind byte is not 0x01 issuance, 0x02 revocation or 0x03 attestation entry",
            )),
        }
    }
}

fn malformed(reason: &'static str) -> RevocationError {
    RevocationError::CertificateLeafMalformed { reason }
}

fn array<const N: usize>(bytes: &[u8]) -> Option<[u8; N]> {
    bytes.try_into().ok()
}

/// Lowercase hex of `bytes`, for names and Debug renderings; never key material.
fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        out.push(char::from(DIGITS[usize::from(byte >> 4)]));
        out.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    out
}
