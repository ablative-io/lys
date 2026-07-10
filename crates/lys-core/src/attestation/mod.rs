//! Signed attestations: the `lys/attestation/v2` tagged `COSE_Sign1`
//! artifact (RFC 9052) binding a payload's SHA-256 hash and a
//! unix-millisecond timestamp to an Ed25519 signer key.
//!
//! # Invariants
//!
//! - **The artifact is the only durable form.** [`Attestation`] implements
//!   no `serde`; the wire shape is exactly the tagged `COSE_Sign1` emitted by
//!   [`Attestation::to_cose_bytes`] — protected headers
//!   `{1: -8 (EdDSA), 3: "application/vnd.lys.attestation.v2+cbor",
//!   4: <raw 32-byte signer key>}`, an empty unprotected map, a claims
//!   payload `{1: <32-byte SHA-256 hash>, 2: <unix-ms timestamp>}`, and a
//!   64-byte Ed25519 signature over the RFC 9052 §4.4 `Sig_structure` with
//!   empty `external_aad`. Off-the-shelf COSE libraries verify it directly.
//! - **Canonical-encoding strictness.** Encoding is RFC 8949 §4.2 core
//!   deterministic, and [`Attestation::from_cose_bytes`] rejects any input
//!   that is not byte-identical to the canonical re-encoding of its parsed
//!   fields — even inputs whose signature is cryptographically valid
//!   (unprotected-header smuggling, indefinite lengths, oversized integer
//!   heads, reordering, tag stripping, trailing garbage).
//! - **Size window.** Canonical artifacts are always 191–199 bytes; the
//!   parser caps input at 1024 bytes before any CBOR work.
//! - **Non-oracle verification.** Every failure — parse, canonicality,
//!   header pins, payload mismatch, signature — collapses to the single
//!   [`TrustError::InvalidSignature`] value.
//! - **Byte-0 domain disjointness.** The signed preimage always begins
//!   `0x84 0x6A "Signature1"`; every other signing context in this crate
//!   starts differently (signed-note bodies are UTF-8, where `0x84` cannot
//!   appear at position 0; X.509 TBS bytes begin `0x30`), so attestation
//!   signatures are structurally non-interchangeable with any other lys
//!   signature (see [`sign`]).
//!
//! Domain meaning (execution receipt, audit entry, dispatch attestation) is
//! applied by consumers; the trust crate only provides the sign/verify and
//! the artifact shape.
//!
//! [`TrustError::InvalidSignature`]: crate::error::TrustError::InvalidSignature

pub mod artifact;
mod encoding;
pub mod sign;

pub use artifact::Attestation;
pub use sign::{sign_attestation, verify_attestation, verify_attestation_bytes};
