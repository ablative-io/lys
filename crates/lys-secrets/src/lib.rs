//! The lys secrets broker.
//!
//! The broker keeps credentials sealed at rest in a [`SecretStore`] whose key
//! lives outside the store directory, hands holders handles instead of
//! credentials, and swaps a handle for the credential only inside
//! [`Broker::use_handle`], where the credential is passed to a caller-supplied
//! forwarding closure and never returned. Every issue, use, refusal and drop
//! writes one signed line into an append-only transparency log ([`AuditLog`]).
//!
//! Permission is asked through the [`PermissionCheck`] trait. [`LocalGrants`]
//! answers it in process; a `SpiceDB`-backed check plugs into the same trait.

pub mod audit;
pub mod broker;
mod encoding;
pub mod error;
mod fsutil;
pub mod handle;
pub mod keys;
pub mod local_grants;
pub mod permission;
pub mod secret;
pub mod store;

pub use audit::{AuditKind, AuditLine, AuditLog, RecordedLine};
pub use broker::{Broker, BrokerPaths, Clock, PRESENTATION_SKEW_MS, RevokeOutcome, UseError, Used};
pub use error::SecretsError;
pub use handle::{HandleId, HandleToken, Holder, IssuedHandle, Presentation, new_operation_id};
pub use keys::{KeyId, StoreKey};
pub use local_grants::{LocalGrants, SecretRelation};
pub use permission::{Denied, PermissionCheck, Permitted};
pub use secret::Secret;
pub use store::{EntryClass, EntryView, SecretStore};

/// `bytes` as lowercase hex.
pub fn to_hex(bytes: &[u8]) -> String {
    encoding::hex(bytes)
}

/// The bytes of a hex string, or `None` when it is not hex.
pub fn from_hex(text: &str) -> Option<Vec<u8>> {
    encoding::unhex(text)
}
