//! Appending issuance, revocation and attestation-entry leaves.
//!
//! Every leaf goes through lys-log-store's `Log::append`, which writes with
//! `LeafStore::append` at the store's extent, with its pin, and refuses any other index. A
//! revocation adds exactly one leaf and removes, rewrites, truncates or
//! replaces nothing: the store trait has no operation that could.
//!
//! The append layer does not decide who asked for a revocation. It signs with
//! the issuing authority identity it is given, so until the request path lands
//! the only way to revoke is a caller that holds that identity.
//!
//! When the store refuses or fails a write, the refusal comes back with the
//! operation and the index it was attempted at, the write is not retried, and
//! the leaf is not reported as written.

use lys_core::Ed25519Identity;
use lys_log_store::{LeafStore, Log};

use super::error::{AppendOperation, RevocationError};
use super::leaf::{CertificateHash, CertificateLeaf, Revocation};

/// Appends the issuance leaf of the certificate whose DER is `der` and
/// returns the index it was written at.
///
/// # Errors
///
/// [`RevocationError::CertificateLeafMalformed`] when `der` is empty, before
/// anything is written, and [`RevocationError::StoreWriteFailed`] with the
/// store's refusal when the leaf was not written.
pub fn append_issuance<S: LeafStore>(log: &mut Log<S>, der: &[u8]) -> Result<u64, RevocationError> {
    if der.is_empty() {
        return Err(RevocationError::CertificateLeafMalformed {
            reason: "an issuance leaf carries no DER byte",
        });
    }
    append_leaf(
        log,
        AppendOperation::Issuance,
        &CertificateLeaf::Issuance { der: der.to_vec() },
    )
}

/// Appends a revocation of `certificate`, signed with `issuer` for the log's
/// origin, and returns the index it was written at.
///
/// # Errors
///
/// [`RevocationError::StoreWriteFailed`] with the store's refusal when the
/// leaf was not written.
pub fn append_revocation<S: LeafStore>(
    log: &mut Log<S>,
    certificate: CertificateHash,
    issuer: &Ed25519Identity,
) -> Result<u64, RevocationError> {
    let revocation = Revocation::sign(log.origin(), certificate, issuer);
    append_leaf(
        log,
        AppendOperation::Revocation,
        &CertificateLeaf::Revocation(revocation),
    )
}

/// Appends an attestation entry placing the attestation `cose` by the key of
/// `certificate` in log order, and returns the index it was written at.
///
/// # Errors
///
/// [`RevocationError::CertificateLeafMalformed`] when `cose` is empty, before
/// anything is written, and [`RevocationError::StoreWriteFailed`] with the
/// store's refusal when the leaf was not written.
pub fn append_attestation_entry<S: LeafStore>(
    log: &mut Log<S>,
    certificate: CertificateHash,
    cose: &[u8],
) -> Result<u64, RevocationError> {
    if cose.is_empty() {
        return Err(RevocationError::CertificateLeafMalformed {
            reason: "an attestation entry carries no COSE byte after its hash",
        });
    }
    append_leaf(
        log,
        AppendOperation::AttestationEntry,
        &CertificateLeaf::AttestationEntry {
            certificate,
            cose: cose.to_vec(),
        },
    )
}

/// One `Log::append` of `leaf`, at the index the log's tree names next.
fn append_leaf<S: LeafStore>(
    log: &mut Log<S>,
    operation: AppendOperation,
    leaf: &CertificateLeaf,
) -> Result<u64, RevocationError> {
    let index = log.tree().len();
    log.append(&leaf.encode())
        .map(|(written, ..)| written)
        .map_err(|refusal| RevocationError::StoreWriteFailed {
            operation,
            index,
            reason: refusal.to_string(),
        })
}
