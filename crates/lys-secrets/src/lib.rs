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
//!
//! The secrets list and a lease's read, revoke and relinquish ask one seam
//! ([`access`]): what an [`Asker`] may see, and who may discover and revoke
//! a [`Lease`]. A person's team ids come from the group claims on their
//! token, through [`team_ids`] alone.

pub mod access;
pub mod audit;
pub mod broker;
mod encoding;
pub mod error;
mod fsutil;
pub mod handle;
pub mod keys;
pub mod local_grants;
pub mod oauth;
pub mod permission;
pub mod secret;
pub mod service;
pub mod store;
pub mod teams;

pub use access::{Asker, AskerKind};
pub use audit::{AuditKind, AuditLine, AuditLog, Opened, RecordedLine, STATE_DOMAIN};
pub use broker::{
    Admitted, Ask, Broker, BrokerPaths, Checked, Clock, Discovery, EndAct, EndWay, Ended,
    HandleEnded, HandleRecord as Lease, HeldHandle, LeaseEnd, LeaseView, ListScope, OwnerChanged,
    PRESENTATION_SKEW_MS, RevocationState, RevokeOutcome, SNAPSHOT_EVERY, SecretSettings, Settled,
    Signature, SigningRefusal, SnapshotReport, SystemBehind, Ticket, UpstreamRevocation, UseError,
    Used,
};
pub use error::{
    AccountsRefusal, BoundsRefusal, LeaseRefusal, ListRefusal, OAuthRefusal, OwnerChangeRefusal,
    RevocationRefusal, SecretsError, ServiceRefusal,
};
pub use handle::{
    HandleId, HandleToken, Holder, IssuedHandle, Presentation, new_operation_id, request_digest,
};
pub use keys::{KeyId, StoreKey};
pub use local_grants::{LocalGrants, SecretRelation};
pub use lys_log_store::{SnapshotRefusal, Start};
pub use oauth::{OAuthGrant, Provenance, REFRESH_MARGIN_MS};
pub use permission::{Denied, PermissionCheck, Permitted, Relation};
pub use secret::Secret;
pub use service::{OnBehalf, SERVICE_DOMAIN, ServiceKey, ServiceWindow};
pub use store::{AccountView, EntryClass, EntryView, Recipients, Retired, Scope, SecretStore};
pub use teams::team_ids;

/// `bytes` as lowercase hex.
pub fn to_hex(bytes: &[u8]) -> String {
    encoding::hex(bytes)
}

/// The bytes of a hex string, or `None` when it is not hex.
pub fn from_hex(text: &str) -> Option<Vec<u8>> {
    encoding::unhex(text)
}
