//! [`RevocationError`], the refusals of the certificate log, and
//! [`LeafRefusal`], the names the fold records against a leaf it walked.
//!
//! Every refusal's message begins with its name from the contract's refusal
//! table, so a caller can say which rule refused and act on it. No variant
//! carries a default and none carries key material.

use std::fmt;

/// The append that was attempted, named in a write refusal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppendOperation {
    /// An issuance leaf.
    Issuance,
    /// A revocation leaf.
    Revocation,
    /// An attestation entry.
    AttestationEntry,
}

impl fmt::Display for AppendOperation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Issuance => "issuance",
            Self::Revocation => "revocation",
            Self::AttestationEntry => "attestation entry",
        })
    }
}

/// Refusals raised by the leaf layer, the fold, the append and the two
/// verification calls.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RevocationError {
    /// Bytes are not one of the three certificate-log leaves.
    #[error("certificate_leaf_malformed: {reason}")]
    CertificateLeafMalformed {
        /// The rule the bytes broke.
        reason: &'static str,
    },
    /// A revocation's signature does not verify under the issuing authority's
    /// key over the signed bytes for this log's origin.
    #[error(
        "revocation_not_signed_by_issuer: the revocation's signature does not verify under the issuing authority's key for this log's origin"
    )]
    RevocationNotSignedByIssuer,
    /// The store failed to read a leaf within its extent; no partial set is answered.
    #[error("store_read_failed: the store could not read leaf {index}: {reason}")]
    StoreReadFailed {
        /// The index the read was attempted at.
        index: u64,
        /// What the store reported.
        reason: String,
    },
    /// The store refused or failed a write; the leaf is not written.
    #[error("store_write_failed: the {operation} leaf was not written at index {index}: {reason}")]
    StoreWriteFailed {
        /// The append that was attempted.
        operation: AppendOperation,
        /// The index the write was attempted at.
        index: u64,
        /// What the store reported.
        reason: String,
    },
    /// The certificate does not verify under the issuing authority's key at
    /// the caller's instant, or its DER cannot be read.
    #[error("certificate_chain_invalid: {reason}")]
    CertificateChainInvalid {
        /// What lys-core's verification reported.
        reason: String,
    },
    /// The fold holds the certificate revoked.
    #[error(
        "certificate_revoked: revoked by the leaf at index {index}, in a fold of {folded_size} leaves"
    )]
    CertificateRevoked {
        /// The index of the revocation leaf.
        index: u64,
        /// The number of leaves the fold walked.
        folded_size: u64,
    },
    /// The fold holds a leaf it could not decode, and no permit is given.
    #[error(
        "fold_unreadable_leaf: the leaf at index {index} does not decode, in a fold of {folded_size} leaves, and no permit is given"
    )]
    FoldUnreadableLeaf {
        /// The index of the leaf that does not decode.
        index: u64,
        /// The number of leaves the fold walked.
        folded_size: u64,
    },
    /// The folded size is below the caller's evidence by more than the tolerance.
    #[error(
        "fold_stale: the fold walked {folded_size} leaves, below the {n} the caller has evidence of by more than the tolerance of {tolerance}"
    )]
    FoldStale {
        /// The number of leaves the fold walked.
        folded_size: u64,
        /// The size the caller has evidence of.
        n: u64,
        /// The tolerance the caller gave, counted in entries.
        tolerance: u64,
    },
    /// No valid issuance leaf names the certificate.
    #[error(
        "certificate_not_in_log: no valid issuance leaf names the certificate, in a fold of {folded_size} leaves"
    )]
    CertificateNotInLog {
        /// The number of leaves the fold walked.
        folded_size: u64,
    },
    /// The attestation does not verify under the certificate's subject key.
    #[error(
        "attestation_signature_invalid: the attestation does not verify under the certificate's subject key for the payload"
    )]
    AttestationSignatureInvalid,
    /// No attestation entry with these bytes precedes the revocation leaf.
    #[error(
        "attestation_after_revocation: no attestation entry with these bytes precedes the revocation leaf at index {revocation_index}, in a fold of {folded_size} leaves"
    )]
    AttestationAfterRevocation {
        /// The index of the revocation leaf.
        revocation_index: u64,
        /// The number of leaves the fold walked.
        folded_size: u64,
    },
}

/// The refusal the fold records against one leaf it walked. The leaf stays
/// in the log; it issues and revokes nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LeafRefusal {
    /// An issuance leaf's DER does not verify under the issuing authority's
    /// key at the certificate's own notBefore.
    CertificateChainInvalid,
    /// A revocation leaf's signature does not verify under the issuing
    /// authority's key for this log's origin.
    RevocationNotSignedByIssuer,
    /// A revocation leaf names a certificate with no valid issuance leaf at a
    /// lower index.
    RevocationBeforeIssuance,
    /// A valid issuance leaf names a certificate already revoked at a lower index.
    CertificateReinstatementRefused,
    /// The leaf does not decode.
    FoldUnreadableLeaf,
}

impl LeafRefusal {
    /// The refusal's name in the contract's table.
    pub fn name(self) -> &'static str {
        match self {
            Self::CertificateChainInvalid => "certificate_chain_invalid",
            Self::RevocationNotSignedByIssuer => "revocation_not_signed_by_issuer",
            Self::RevocationBeforeIssuance => "revocation_before_issuance",
            Self::CertificateReinstatementRefused => "certificate_reinstatement_refused",
            Self::FoldUnreadableLeaf => "fold_unreadable_leaf",
        }
    }
}

impl fmt::Display for LeafRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}
