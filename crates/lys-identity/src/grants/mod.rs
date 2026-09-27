//! Human-rooted grants: the application grant contract, its model, its
//! admission and the one authority owner every route calls.

pub mod admission;
pub mod authority;
pub mod codec;
pub mod error;
pub mod events;
pub mod lineage;
pub mod model;
pub mod projection;
pub mod types;

pub use admission::{DelegateRequest, RootRequest, Route};
pub use authority::{ExerciseRequest, Grants, Permit, Recorded, RevokeRequest};
pub use codec::{GRANT_ENVELOPE, MEMBERS, decode_grant, encode_grant};
pub use error::GrantError;
pub use events::{GrantChange, GrantEvent};
pub use lineage::Lineage;
pub use model::{Model, Within};
pub use projection::{GrantBook, GrantRecord, Revocation};
pub use types::{
    Action, Grant, GrantId, GrantParts, PassOn, RecipientKind, Relation, Resource, Source, Window,
};
