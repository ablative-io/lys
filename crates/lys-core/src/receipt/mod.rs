//! Anchor inclusion receipts: the `lys/anchor-receipt/v1` tagged `COSE_Sign1`
//! artifact (RFC 9052) carrying an RFC 9942 verifiable data proof, by which a
//! transparency-log anchor vouches that a leaf was included in its tree.
//!
//! A receipt says: *"the leaf that hashes to this path's base was included at
//! index N in my tree of size S, whose root I vouch for."* When the leaf is a
//! child log's checkpoint bytes, that is a durable, ordered, independently
//! provable record that the child's root existed at the anchor's index N —
//! portable to a stranger years later, even if the anchor is gone.
//!
//! # Invariants
//!
//! - **The artifact is the only durable form.** [`AnchorReceipt`] implements no
//!   `serde`; the wire shape is exactly the tagged `COSE_Sign1` emitted by
//!   [`AnchorReceipt::to_cose_bytes`] — protected headers
//!   `{1: -8 (EdDSA), 3: "application/vnd.lys.receipt.v1+cbor",
//!   4: <raw 32-byte anchor key>, 395: 1 (RFC9162_SHA256)}`, an unprotected
//!   `{396: {-1: [<bstr .cbor [tree_size, leaf_index, path]>]}}`, a **`nil`
//!   payload**, and a 64-byte Ed25519 signature over the RFC 9052 §4.4
//!   `Sig_structure` whose payload is the detached 32-byte Merkle root.
//! - **Canonical-encoding strictness.** Encoding is RFC 8949 §4.2 core
//!   deterministic, and [`AnchorReceipt::from_cose_bytes`] rejects any input
//!   that is not byte-identical to the canonical re-encoding of its parsed
//!   fields — even inputs whose signature is cryptographically valid.
//! - **Non-oracle verification.** Every failure — parse, canonicality, header
//!   pins, wrong anchor, inconsistent proof, signature — collapses to the
//!   single [`TrustError::InvalidSignature`] value.
//! - **Verification names its anchor.** [`verify_receipt`] requires the caller
//!   to supply the anchor key they trust. There is no unattributed verify, for
//!   the reason in [`sign`].
//! - **Exactly one inclusion proof** is issued and accepted, though RFC 9942
//!   permits an array — a receipt carries one signature over one root, so a
//!   multi-proof receipt would need a rule for what disagreement between them
//!   means.
//!
//! # Why the proof sits in an *unprotected* header
//!
//! Because that is where RFC 9942 puts it — and it is safe, for a reason worth
//! stating rather than asserting, since "unprotected" reads as
//! "unauthenticated" to every reviewer who meets it.
//!
//! The signature covers the Merkle **root**, as a detached payload that never
//! appears in the artifact. A verifier therefore cannot read the root; it must
//! *recompute* it from the leaf and the proof. Tamper with the path, the index
//! or the size and the recomputation yields a different root, and the signature
//! over the original root fails. The proof is **authenticated by consequence
//! rather than by coverage**: it cannot be altered without producing bytes the
//! anchor never signed.
//!
//! Two properties follow that the alternative would have lost. The receipt is
//! **self-verifying** — a stranger holding the receipt and the leaf needs no
//! checkpoint, no published root and no service to ask. And the format matches
//! RFC 9942 exactly, so off-the-shelf COSE tooling verifies the signature
//! without knowing anything about lys.
//!
//! One limitation follows too, and it is documented rather than hidden:
//! `tree_size` is authenticated only as far as it changes the reconstruction,
//! which is not always. See [`sign`] for the exact case and its mitigation.
//!
//! # What a valid receipt does not establish
//!
//! That the anchor is honest, or that its tree is append-only. A receipt is one
//! anchor's signed statement about its own tree. Detecting an anchor that
//! rewrote history needs a consistency proof between two checkpoints, or a
//! second witness — never the receipt alone. The receipt's job is to make the
//! statement durable and attributable, so that misbehaviour is *provable* after
//! the fact rather than deniable.
//!
//! [`TrustError::InvalidSignature`]: crate::error::TrustError::InvalidSignature

pub mod artifact;
mod encoding;
pub mod sign;

pub use artifact::AnchorReceipt;
pub use sign::{sign_receipt, verify_receipt, verify_receipt_bytes};
