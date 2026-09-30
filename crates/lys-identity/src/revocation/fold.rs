//! The fold: one pass over a certificate log that answers its live set.
//!
//! The fold reads every leaf from index 0 to the store's extent, in order,
//! through `LeafStore::extent`, `LeafStore::leaf` and `LeafStore::origin`
//! alone. It writes nothing.
//!
//! # Invariants
//!
//! - An issuance leaf is valid when its DER verifies under the issuing
//!   authority's public key with lys-core's `verify_certificate_chain_at` at
//!   the certificate's own notBefore instant, read from that DER, so the check
//!   is of the issuer's signature and reads no clock. The validity window at
//!   the caller's instant is the verification call's check, not the fold's,
//!   and a certificate used before its window begins is refused at the moment
//!   of use by the claim verifier, not here. An issuance leaf that is not
//!   valid records `certificate_chain_invalid`, issues nothing and counts as
//!   no issuance. A certificate whose subject equals the issuing authority's
//!   hex public key fails lys-core's self-signed screen, a known false
//!   positive the fold records as `certificate_chain_invalid` and does not
//!   work around.
//! - A revocation leaf revokes only when its signature verifies under the
//!   issuing authority's key for this store's origin
//!   (`revocation_not_signed_by_issuer` otherwise) and its certificate has a
//!   valid issuance leaf at a lower index (`revocation_before_issuance`
//!   otherwise).
//! - A valid issuance leaf naming a certificate already revoked is a
//!   reinstatement and is refused as `certificate_reinstatement_refused`; the
//!   certificate stays revoked. A later leaf never wins over a revocation.
//! - A second valid revocation of a revoked certificate and a second issuance
//!   leaf of a live certificate change nothing and refuse nothing.
//! - A leaf that does not decode records `fold_unreadable_leaf`; it issues and
//!   revokes nothing, and no permit is given from a fold that holds one.
//! - A leaf the store fails to read is `store_read_failed`, and no partial set
//!   is answered.
//! - A revoked certificate stays in the answer as revoked with its leaf index.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use lys_core::ca::verify_certificate_chain_at;
use lys_log_store::LeafStore;
use x509_parser::prelude::{FromDer, X509Certificate};

use super::error::{LeafRefusal, RevocationError};
use super::leaf::{CertificateHash, CertificateLeaf};

/// A leaf the fold walked and refused; it stays in the log and does nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RefusedLeaf {
    /// The leaf's index.
    pub index: u64,
    /// The refusal recorded against it.
    pub refusal: LeafRefusal,
}

/// An attestation entry the fold walked: where an attestation by a
/// certificate's key sits in log order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttestationEntry {
    /// The leaf's index.
    pub index: u64,
    /// The certificate whose key made the attestation.
    pub certificate: CertificateHash,
    /// The attestation's COSE bytes, exactly as the leaf carries them.
    pub cose: Vec<u8>,
}

/// The live set of a certificate log at the size the fold walked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveSet {
    /// The number of leaves the fold walked: the store's extent.
    pub folded_size: u64,
    /// Each issued certificate with the index of its first valid issuance leaf.
    pub issued: BTreeMap<CertificateHash, u64>,
    /// Each revoked certificate with the index of its first valid revocation
    /// leaf. A revoked certificate is never dropped from here.
    pub revoked: BTreeMap<CertificateHash, u64>,
    /// Every refused leaf with its index and refusal, in log order.
    pub refused: Vec<RefusedLeaf>,
    /// Every attestation entry, in log order.
    pub attestation_entries: Vec<AttestationEntry>,
}

impl LiveSet {
    /// Whether `certificate` has a valid issuance leaf and no valid revocation leaf.
    pub fn is_live(&self, certificate: &CertificateHash) -> bool {
        self.issued.contains_key(certificate) && !self.revoked.contains_key(certificate)
    }

    /// The index of the first leaf the fold could not decode, if it holds one.
    pub fn unreadable_leaf(&self) -> Option<u64> {
        self.refused
            .iter()
            .find(|refused| refused.refusal == LeafRefusal::FoldUnreadableLeaf)
            .map(|refused| refused.index)
    }

    /// The indexes of the leaves refused as `refusal`, in log order.
    pub fn refused_as(&self, refusal: LeafRefusal) -> Vec<u64> {
        self.refused
            .iter()
            .filter(|refused| refused.refusal == refusal)
            .map(|refused| refused.index)
            .collect()
    }

    fn refuse(&mut self, index: u64, refusal: LeafRefusal) {
        self.refused.push(RefusedLeaf { index, refusal });
    }
}

/// Folds the certificate log in `store` under `issuer_public_key`.
///
/// # Errors
///
/// [`RevocationError::StoreReadFailed`] naming the index when the store fails
/// to read a leaf within its extent; nothing partial is answered.
pub fn fold<S: LeafStore>(
    store: &S,
    issuer_public_key: &[u8; 32],
) -> Result<LiveSet, RevocationError> {
    let extent = store.extent();
    let origin = store.origin();
    let mut set = LiveSet {
        folded_size: extent,
        issued: BTreeMap::new(),
        revoked: BTreeMap::new(),
        refused: Vec::new(),
        attestation_entries: Vec::new(),
    };
    for index in 0..extent {
        let bytes = read_leaf(store, index)?;
        let Ok(leaf) = CertificateLeaf::decode(&bytes) else {
            set.refuse(index, LeafRefusal::FoldUnreadableLeaf);
            continue;
        };
        match leaf {
            CertificateLeaf::Issuance { der } => {
                if !issuance_is_valid(&der, issuer_public_key) {
                    set.refuse(index, LeafRefusal::CertificateChainInvalid);
                    continue;
                }
                let certificate = CertificateHash::of_der(&der);
                if set.revoked.contains_key(&certificate) {
                    set.refuse(index, LeafRefusal::CertificateReinstatementRefused);
                    continue;
                }
                set.issued.entry(certificate).or_insert(index);
            }
            CertificateLeaf::Revocation(revocation) => {
                if revocation.verify(origin, issuer_public_key).is_err() {
                    set.refuse(index, LeafRefusal::RevocationNotSignedByIssuer);
                    continue;
                }
                if !set.issued.contains_key(&revocation.certificate) {
                    set.refuse(index, LeafRefusal::RevocationBeforeIssuance);
                    continue;
                }
                set.revoked.entry(revocation.certificate).or_insert(index);
            }
            CertificateLeaf::AttestationEntry { certificate, cose } => {
                set.attestation_entries.push(AttestationEntry {
                    index,
                    certificate,
                    cose,
                });
            }
        }
    }
    Ok(set)
}

/// The bytes of leaf `index`, or `store_read_failed` naming it.
fn read_leaf<S: LeafStore>(store: &S, index: u64) -> Result<Vec<u8>, RevocationError> {
    match store.leaf(index) {
        Ok(Some(bytes)) => Ok(bytes),
        Ok(None) => Err(RevocationError::StoreReadFailed {
            index,
            reason: "the store reports no leaf at an index within its extent".to_owned(),
        }),
        Err(failure) => Err(RevocationError::StoreReadFailed {
            index,
            reason: failure.to_string(),
        }),
    }
}

/// Whether `der` verifies under `issuer_public_key` at its own notBefore
/// instant, the check of the issuer's signature that reads no clock.
fn issuance_is_valid(der: &[u8], issuer_public_key: &[u8; 32]) -> bool {
    not_before(der)
        .is_some_and(|at| verify_certificate_chain_at(der, issuer_public_key, at).is_ok())
}

/// The certificate's notBefore instant, read from its DER.
fn not_before(der: &[u8]) -> Option<DateTime<Utc>> {
    let (_, certificate) = X509Certificate::from_der(der).ok()?;
    DateTime::<Utc>::from_timestamp(certificate.validity().not_before.timestamp(), 0)
}
