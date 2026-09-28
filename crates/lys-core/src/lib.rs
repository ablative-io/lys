//! `lys-core` — domain-agnostic cryptographic trust primitives.
//!
//! A focused library of trust primitives: certificate authority operations,
//! Merkle transparency-log operations, signed attestations, sealed payload
//! transport, and Ed25519 key management. Consumers compose domain meaning
//! on top — this crate knows nothing about any higher-level concepts.
//!
//! The foundation laid here is [`TrustError`], [`TrustResult`], and
//! [`Ed25519Identity`]. The [`ca`], [`merkle`], [`attestation`], `receipt`,
//! `bundle`, `delegation`, [`seal`], [`checkpoint`], and [`tlog`] modules are
//! implemented on top of these primitives. (`receipt`, `bundle` and
//! `delegation` are named without intra-doc links because they exist only under
//! `unstable-anchor`; a link from ungated docs to a gated item resolves under
//! `--all-features` and breaks in the default doc build, which is the shape
//! consumers get.)
//!
//! ```
//! use lys_core::TrustResult;
//!
//! fn fallible_op() -> TrustResult<()> {
//!     Ok(())
//! }
//!
//! assert!(fallible_op().is_ok());
//! ```

// Library code contains no unsafe whatsoever, and no test in this crate uses
// unsafe code either. Non-test builds forbid it outright; test builds are held
// by the workspace-level `deny`, which no test overrides.
#![cfg_attr(not(test), forbid(unsafe_code))]

pub mod agent_request;
pub mod attestation;
#[cfg(feature = "unstable-anchor")]
pub mod bundle;
pub mod ca;
mod cbor;
pub mod checkpoint;
#[cfg(feature = "unstable-anchor")]
pub mod delegation;
pub mod error;
pub mod keys;
pub mod merkle;
#[cfg(feature = "unstable-anchor")]
pub mod receipt;
pub mod seal;
pub mod tlog;

pub use error::{TrustError, TrustResult};
pub use keys::Ed25519Identity;

/// Lowercase hex encoding of a byte slice.
pub(crate) fn hex_lower(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        s.push(char::from(DIGITS[usize::from(b >> 4)]));
        s.push(char::from(DIGITS[usize::from(b & 0x0f)]));
    }
    s
}
