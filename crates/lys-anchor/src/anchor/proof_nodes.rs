//! [`proof_nodes`] — an RFC 6962 inclusion proof's byte form, split into the
//! fixed-width nodes a receipt's path is made of.
//!
//! # Invariants
//!
//! - **A length that is not a whole number of digests is an error, never a
//!   truncation.** `chunks_exact` silently drops a short tail; a receipt built
//!   over a silently shortened path would be internally consistent, would
//!   verify, and would attest to a root nobody's tree ever held. The remainder
//!   is therefore checked and refused.
//! - **The order is the proof's order, unchanged.** RFC 6962's `PATH(m, D[n])`
//!   is leaf-ward first, and that is what
//!   [`root_from_inclusion_path`](lys_core::merkle::root_from_inclusion_path)
//!   consumes. This function reorders nothing; it is a re-shaping of the same
//!   bytes and the concatenation of its output is its input.
//! - **It parses no encoding.** There is no base64 here, no CBOR, no framing —
//!   nothing that could constitute a second copy of a format.
//!
//! # Why this is not `lys-core`'s chunker
//!
//! `lys-core` has one already, in `tlog::build`, and it is private and returns
//! `Vec<String>` — base64 text for the JSON proof artifact. Receipts need
//! `[u8; 32]` values. Reaching the private one would mean adding a public
//! function to a published, semver-bound crate for the benefit of one caller
//! in an unpublished one, and this plan declines that. The duplication is
//! twelve lines of `chunks_exact`, which is not the kind of duplication that
//! drifts: there is no format here to drift *from*. The tests below hold this
//! copy against `lys-core`'s own root reconstruction rather than against
//! itself.

use crate::error::{AnchorError, AnchorResult};

/// The SHA-256 digest size, which is the width of every node in an RFC 6962
/// inclusion path.
const DIGEST_LEN: usize = 32;

/// Splits an RFC 6962 inclusion proof's byte encoding into its 32-byte nodes,
/// in order.
///
/// The input is what
/// [`InclusionProof::as_bytes`](lys_core::merkle::InclusionProof::as_bytes)
/// returns; the output is what `lys_core::receipt::sign_receipt` takes as its
/// `inclusion_path` (named here rather than linked: the receipt module is
/// behind `lys-core`'s off-by-default feature, and this function is not).
/// An empty input yields an empty path, which is the correct
/// answer for the sole leaf of a one-leaf tree — and is separately refused by
/// `sign_receipt`, which will not issue a receipt over an empty path.
///
/// # Errors
///
/// [`AnchorError::MalformedInclusionPath`] if `proof_bytes.len()` is not a
/// multiple of 32. That cannot arise from a proof this crate produced — a tree
/// emits whole digests — which is exactly why it is checked rather than
/// assumed: a precondition nothing currently violates is the one whose
/// violation nobody would notice.
pub fn proof_nodes(proof_bytes: &[u8]) -> AnchorResult<Vec<[u8; DIGEST_LEN]>> {
    let chunks = proof_bytes.chunks_exact(DIGEST_LEN);
    if !chunks.remainder().is_empty() {
        return Err(AnchorError::MalformedInclusionPath {
            byte_len: proof_bytes.len(),
        });
    }
    chunks
        .map(|chunk| {
            // Total rather than asserted: `chunks_exact` yields slices of
            // exactly `DIGEST_LEN`, so this conversion cannot fail today, and
            // it is mapped to a refusal instead of an `unwrap` so that it
            // still cannot panic if that ever stops being true.
            <[u8; DIGEST_LEN]>::try_from(chunk).map_err(|_err| {
                AnchorError::MalformedInclusionPath {
                    byte_len: proof_bytes.len(),
                }
            })
        })
        .collect()
}

#[cfg(test)]
#[path = "proof_nodes_tests.rs"]
mod tests;
