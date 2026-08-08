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
//! - **Domain separation, by two different mechanisms — ⛔ corrected.** This
//!   invariant used to read "byte-0 domain disjointness … every other signing
//!   context in this crate starts differently … so attestation signatures are
//!   structurally non-interchangeable with any other lys signature." **The
//!   stated mechanism is false**, and each new COSE format in this crate has
//!   falsified it again. The conclusion still holds; the argument for it does
//!   not, so it is replaced rather than patched:
//!
//!   - **Against non-COSE contexts**, byte-0 disjointness is genuine and is the
//!     mechanism: signed-note bodies are UTF-8, where `0x84` cannot appear at
//!     position 0, and X.509 TBS bytes begin `0x30`.
//!   - **Against the crate's other COSE artifacts it is no mechanism at all.**
//!     Every `COSE_Sign1` preimage in lys begins with the same twelve bytes,
//!     `0x84 0x6A "Signature1"` — the receipt, the consistency receipt and the
//!     anchor delegation included. What separates them is *inside* the signed
//!     bytes: a distinct protected content type, and protected buckets of
//!     different lengths and shapes (an attestation's is a 3-entry map of 80
//!     bytes; a receipt's a 4-entry map; a delegation's a 3-entry map of 86).
//!     Every one of those verifiers additionally pins **its own** content type
//!     rather than reading the wire's, so none will parse another's artifact.
//!
//!   The correction is recorded rather than silently applied because the
//!   original phrasing survived two new formats: `receipt/sign.rs` was written
//!   with the accurate argument and this module was not updated alongside it.
//!
//! # ⚠️ `signer_public_key` is a claim, not an authority
//!
//! [`verify_attestation`] and [`verify_attestation_bytes`] read the signer key
//! **out of the artifact** and verify against it. They take no expected-key
//! argument, so they establish that *somebody holding some key* signed this
//! payload — never that the signer is anyone the caller trusts. An attacker
//! signs an attestation over any payload with their own key, puts their own key
//! in `kid`, and the result is cryptographically perfect and vouches for
//! nothing. This is the same trap as a self-signed certificate.
//!
//! **Every caller must compare `attestation.signer_public_key` against a key it
//! independently trusts.** Nothing in this module does it for you, and the
//! signature check passing is not that comparison.
//!
//! The newer artifact classes close this in their APIs — `receipt` and
//! `delegation` both *require* the expected key as an argument and have no
//! unattributed verify at all. This module cannot follow them: it is ungated,
//! shipped and semver-bound, so the fix is a `v3` signature rather than a
//! change here, and until then the warning is the mitigation.
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
