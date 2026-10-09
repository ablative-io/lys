//! Human-rooted grants: the application grant contract, its model, its
//! admission, its signed events and their projections, the one authority
//! owner every route calls, and what a person cannot give from it.

pub mod admission;
pub mod authority;
pub mod cannot_give;
pub mod channel_membership;
pub mod codec;
mod commit;
pub mod error;
pub mod events;
pub mod expiry;
pub mod frame;
pub mod lineage;
mod mode;
pub mod model;
pub mod permission;
pub mod projection;
pub mod receipt;
pub mod recovery;
mod refusal_codec;
pub mod revocation;
pub mod schema;
pub mod schema_diff;
#[cfg(test)]
mod schema_tests;
mod settlement;
pub mod shipped;
mod state;
mod tail_witness;
#[cfg(any(test, feature = "test-support"))]
pub mod test_support;
pub mod types;
pub mod usage;

pub use admission::{DelegateRequest, RootRequest, Route};
pub use authority::{ExerciseRequest, Grants, ONE_TIME_SPENT, Permit, Recorded, RevokeRequest};
pub use cannot_give::{
    CannotGiveItem, CannotGiveList, CannotGiveReason, CannotGiveRequest, CannotGiveSubject,
    SERVICE_ACCOUNT,
};
pub use channel_membership::{identity_kind, placed_within};
pub use codec::{GRANT_ENVELOPE, MEMBERS, decode_grant, encode_grant};
pub use error::GrantError;
pub use events::{
    GrantChange, GrantEvent, SignedGrantEvent, decode_event_body, encode_event_body,
    sign_grant_event, verify_grant_event,
};
pub use frame::Frame;
pub use lineage::Lineage;
pub use model::{KindModel, Model, Within};
pub use permission::{
    MemoryRelationships, ObjectRef, Relationship, RelationshipStore, SCHEMA, parent_relation,
    placement,
};
pub use projection::{GrantBook, GrantRecord, LastUse, Revocation};
pub use receipt::{GrantReceipt, verify_grant_receipt};
pub use recovery::{GrantLedger, Uncertain};
pub use schema::{ANY_KIND, AppSchema, KindSchema, LYS_APP, SchemaError, app_id, owner_of};
pub use schema_diff::{Named, RoleChange, SchemaDiff, Standing, diff, stranded};
pub use settlement::ProjectionDegraded;
pub use shipped::{
    SHIPPED_VERSION, WITHHELD_FROM_AGENTS, agent_may_hold, machine_may_hold, shipped_model,
};
pub use types::{
    Action, Grant, GrantId, GrantParts, Mode, PassOn, RecipientKind, Relation, Resource, Source,
    Window,
};
pub use usage::{Unreported, Usage};
