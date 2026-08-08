//! The anchor's key custody: a boundary that is reserved, and the one
//! implementation that exists behind it.
//!
//! [`Signer`] is the surface an anchor is meant to sign through. [`FileSigner`]
//! is its only implementation. [`InProcessSigner`] is the extra bound that every
//! signing operation *except one* carries, and it is there to record — in the
//! type system, where it cannot be overlooked — that **a remote signer can
//! implement [`Signer`] today and still cannot publish a checkpoint or a
//! receipt.** [`signer`] states why, what would have to change, and what must not
//! be done instead.
//!
//! The exception is the genesis delegation, whose signer is the offline root key
//! and which therefore takes only [`Signer`]. It is the one place where this
//! boundary is a working boundary rather than a reserved one.

pub mod file_signer;
pub mod signer;

pub use file_signer::FileSigner;
pub use signer::{InProcessSigner, Signer};
