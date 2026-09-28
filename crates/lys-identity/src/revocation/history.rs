//! A revoked certificate's history: the attestations its key made before the
//! revocation keep verifying, and nothing after it does.
//!
//! The log order is the evidence of time. An attestation by a revoked
//! certificate's key verifies if and only if an attestation entry naming the
//! certificate's hash with exactly these COSE bytes sits at an index below the
//! revocation leaf, so an attestation made after a compromise cannot be placed
//! before the revocation. An attestation with no such entry, an entry above
//! the revocation included, is refused as `attestation_after_revocation`
//! naming the revocation leaf.
//!
//! An attestation by a certificate that is not revoked needs no log entry of
//! its own: its signature is checked under the certificate's subject key and
//! the fold is checked for a revocation covering the certificate, subject to
//! the same `fold_unreadable_leaf` and `fold_stale` refusals as
//! [`super::verify`], and the answer is not revoked.
//!
//! The log's inclusion and consistency proofs and the issuance leaf keep
//! verifying after a revocation with lys-core's `verify_inclusion_raw` and
//! `verify_consistency`: nothing here removes, rewrites or hides a leaf.
//! Nothing here reads a clock.

use lys_core::attestation::verify_attestation_bytes_by_signer;
use lys_core::ca::certificate_subject_public_key;
use lys_log_store::LeafStore;

use super::error::RevocationError;
use super::fold::fold;
use super::leaf::CertificateHash;
use super::verify::{Evidence, refuse_unreadable};

/// Where an attestation stands against the certificate log.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttestationStanding {
    /// The certificate is revoked and the attestation's entry precedes the
    /// revocation leaf.
    Verified {
        /// The index of the attestation entry.
        entry_index: u64,
        /// The index of the revocation leaf.
        revocation_index: u64,
        /// The number of leaves the fold walked.
        folded_size: u64,
    },
    /// The certificate is not revoked in the log, so the attestation stands on
    /// its signature alone.
    NotRevoked {
        /// The number of leaves the fold walked.
        folded_size: u64,
    },
}

/// Verifies the attestation `cose` over `payload` by the key of the
/// certificate whose DER is `cert_der`, against the log in `store` under
/// `issuer_public_key`, with the caller's `evidence` of the log's size.
///
/// # Errors
///
/// [`RevocationError::CertificateChainInvalid`] when the DER carries no
/// Ed25519 subject key; [`RevocationError::AttestationSignatureInvalid`] when
/// lys-core's `verify_attestation_bytes_by_signer` refuses the attestation
/// under that key; [`RevocationError::StoreReadFailed`] when the store fails
/// to read a leaf. When the fold holds the certificate revoked:
/// [`RevocationError::FoldUnreadableLeaf`] before any verified answer, then
/// [`RevocationError::AttestationAfterRevocation`] naming the revocation leaf
/// when no entry with these bytes precedes it. Otherwise
/// [`RevocationError::FoldUnreadableLeaf`] and [`RevocationError::FoldStale`]
/// as [`super::verify_certificate`] refuses them.
pub fn verify_attestation_against_log<S: LeafStore>(
    store: &S,
    issuer_public_key: &[u8; 32],
    cert_der: &[u8],
    cose: &[u8],
    payload: &[u8],
    evidence: Evidence,
) -> Result<AttestationStanding, RevocationError> {
    let subject_key = certificate_subject_public_key(cert_der).map_err(|refusal| {
        RevocationError::CertificateChainInvalid {
            reason: refusal.to_string(),
        }
    })?;
    if verify_attestation_bytes_by_signer(cose, payload, &subject_key).is_err() {
        return Err(RevocationError::AttestationSignatureInvalid);
    }
    let set = fold(store, issuer_public_key)?;
    let certificate = CertificateHash::of_der(cert_der);
    let Some(&revocation_index) = set.revoked.get(&certificate) else {
        refuse_unreadable(&set)?;
        evidence.check(set.folded_size)?;
        return Ok(AttestationStanding::NotRevoked {
            folded_size: set.folded_size,
        });
    };
    refuse_unreadable(&set)?;
    let entry = set.attestation_entries.iter().find(|entry| {
        entry.index < revocation_index && entry.certificate == certificate && entry.cose == cose
    });
    match entry {
        Some(entry) => Ok(AttestationStanding::Verified {
            entry_index: entry.index,
            revocation_index,
            folded_size: set.folded_size,
        }),
        None => Err(RevocationError::AttestationAfterRevocation {
            revocation_index,
            folded_size: set.folded_size,
        }),
    }
}
