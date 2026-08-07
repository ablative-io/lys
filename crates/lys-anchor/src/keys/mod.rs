//! The anchor's key custody: a boundary that is reserved, and the one
//! implementation that exists behind it.
//!
//! [`Signer`] is the surface an anchor is meant to sign through. [`FileSigner`]
//! is its only implementation. [`InProcessSigner`] is the extra bound every
//! signing operation currently carries, and it is there to record — in the type
//! system, where it cannot be overlooked — that **a remote signer can implement
//! [`Signer`] today and still cannot drive this crate.** [`signer`] states why,
//! what would have to change, and what must not be done instead.

pub mod file_signer;
pub mod signer;

pub use file_signer::FileSigner;
pub use signer::{InProcessSigner, Signer};
