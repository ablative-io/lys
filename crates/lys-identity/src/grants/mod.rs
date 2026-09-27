//! Human-rooted grants: the application grant contract, its model, its
//! admission, its signed events and their projections, and the one authority
//! owner every route calls.

pub mod admission;
pub mod authority;
pub mod codec;
pub mod error;
pub mod events;
pub mod expiry;
pub mod lineage;
pub mod model;
pub mod permission;
pub mod projection;
pub mod receipt;
pub mod recovery;
pub mod revocation;
pub mod types;

pub use admission::{DelegateRequest, RootRequest, Route};
pub use authority::{ExerciseRequest, Grants, Permit, Recorded, RevokeRequest};
pub use codec::{GRANT_ENVELOPE, MEMBERS, decode_grant, encode_grant};
pub use error::GrantError;
pub use events::{
    GrantChange, GrantEvent, SignedGrantEvent, decode_event_body, encode_event_body,
    sign_grant_event, verify_grant_event,
};
pub use lineage::Lineage;
pub use model::{Model, Within};
pub use permission::{MemoryRelationships, ObjectRef, Relationship, RelationshipStore, SCHEMA};
pub use projection::{GrantBook, GrantRecord, Revocation};
pub use receipt::{GrantReceipt, verify_grant_receipt};
pub use recovery::{GrantLedger, Uncertain};
pub use types::{
    Action, Grant, GrantId, GrantParts, PassOn, RecipientKind, Relation, Resource, Source, Window,
};
