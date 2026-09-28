//! Verifying a certificate against its log, with the caller's evidence of the
//! log's size and the tolerance as inputs.
//!
//! Verification without a log does not check revocation: lys-core's
//! `verify_certificate_chain` and `verify_certificate_chain_at` and the
//! `lys ca verify` command check the issuer's signature and the validity
//! window and keep passing a revoked certificate. [`verify_certificate`] is
//! the form that checks revocation, because it folds the log.
//!
//! # Invariants
//!
//! - Every input is required: the instant to check the validity window at, the
//!   size N the caller has evidence of and the tolerance counted in entries.
//!   Nothing here supplies a default for any of them and nothing reads a clock.
//! - The answers come in this order: `certificate_chain_invalid` before any
//!   fold answer; `certificate_revoked` naming the revocation leaf's index,
//!   whatever the folded size; `fold_unreadable_leaf` naming the index;
//!   `fold_stale` when the folded size is below N by more than the tolerance;
//!   `certificate_not_in_log`; and otherwise live.
//! - Every fold answer, permit and refusal alike, carries the folded size. A
//!   refusal stands on the fold's own authority whatever its size; a permit
//!   is never given beyond what the fold has walked.
//! - The fold detects a truncated log only against the caller's N: a log
//!   shown truncated has a tip too, and an N no larger than the truncated
//!   size is shown a consistent, shorter log.

use chrono::{DateTime, Utc};
use lys_core::ca::verify_certificate_chain_at;
use lys_log_store::LeafStore;

use super::error::RevocationError;
use super::fold::{LiveSet, fold};
use super::leaf::CertificateHash;

/// The caller's evidence of the log's size and the tolerance it accepts. Both
/// are required; there is no default for either.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Evidence {
    /// The log size the caller has evidence of.
    pub n: u64,
    /// How many entries below `n` the folded size may be and still permit.
    pub tolerance: u64,
}

impl Evidence {
    /// Refuses `fold_stale` when `folded_size` is below `n` by more than the
    /// tolerance.
    ///
    /// # Errors
    ///
    /// [`RevocationError::FoldStale`] naming the folded size, N and the tolerance.
    pub fn check(self, folded_size: u64) -> Result<(), RevocationError> {
        if folded_size.saturating_add(self.tolerance) < self.n {
            return Err(RevocationError::FoldStale {
                folded_size,
                n: self.n,
                tolerance: self.tolerance,
            });
        }
        Ok(())
    }
}

/// A certificate the log holds issued and not revoked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Live {
    /// The number of leaves the fold walked.
    pub folded_size: u64,
    /// The size the caller had evidence of.
    pub n: u64,
    /// The index of the certificate's first valid issuance leaf.
    pub issuance_index: u64,
}

/// Verifies the certificate whose DER is `cert_der` against the log in
/// `store` under `issuer_public_key`, checking its validity window at `at`,
/// with the caller's `evidence` of the log's size.
///
/// # Errors
///
/// In this order: [`RevocationError::CertificateChainInvalid`] when lys-core's
/// `verify_certificate_chain_at` refuses the certificate at `at`;
/// [`RevocationError::StoreReadFailed`] when the store fails to read a leaf;
/// [`RevocationError::CertificateRevoked`] naming the revocation leaf;
/// [`RevocationError::FoldUnreadableLeaf`] naming the leaf that does not
/// decode; [`RevocationError::FoldStale`]; and
/// [`RevocationError::CertificateNotInLog`] when no valid issuance leaf names
/// the certificate.
pub fn verify_certificate<S: LeafStore>(
    store: &S,
    issuer_public_key: &[u8; 32],
    cert_der: &[u8],
    at: DateTime<Utc>,
    evidence: Evidence,
) -> Result<Live, RevocationError> {
    verify_certificate_chain_at(cert_der, issuer_public_key, at).map_err(|refusal| {
        RevocationError::CertificateChainInvalid {
            reason: refusal.to_string(),
        }
    })?;
    let set = fold(store, issuer_public_key)?;
    let certificate = CertificateHash::of_der(cert_der);
    refuse_revoked_or_unreadable(&set, &certificate)?;
    evidence.check(set.folded_size)?;
    let Some(&issuance_index) = set.issued.get(&certificate) else {
        return Err(RevocationError::CertificateNotInLog {
            folded_size: set.folded_size,
        });
    };
    Ok(Live {
        folded_size: set.folded_size,
        n: evidence.n,
        issuance_index,
    })
}

/// The two refusals that stand on the fold's own authority whatever its size.
///
/// # Errors
///
/// [`RevocationError::CertificateRevoked`] when the fold holds `certificate`
/// revoked, then [`RevocationError::FoldUnreadableLeaf`] when it holds a leaf
/// it could not decode.
pub(super) fn refuse_revoked_or_unreadable(
    set: &LiveSet,
    certificate: &CertificateHash,
) -> Result<(), RevocationError> {
    if let Some(&index) = set.revoked.get(certificate) {
        return Err(RevocationError::CertificateRevoked {
            index,
            folded_size: set.folded_size,
        });
    }
    refuse_unreadable(set)
}

/// # Errors
///
/// [`RevocationError::FoldUnreadableLeaf`] naming the first leaf the fold
/// could not decode.
pub(super) fn refuse_unreadable(set: &LiveSet) -> Result<(), RevocationError> {
    if let Some(index) = set.unreadable_leaf() {
        return Err(RevocationError::FoldUnreadableLeaf {
            index,
            folded_size: set.folded_size,
        });
    }
    Ok(())
}
