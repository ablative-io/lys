//! Human-rooted grants: the application grant contract, its model and its encoding.

pub mod codec;
pub mod error;
pub mod model;
pub mod types;

pub use codec::{GRANT_ENVELOPE, MEMBERS, decode_grant, encode_grant};
pub use error::GrantError;
pub use model::{Model, Within};
pub use types::{
    Action, Grant, GrantId, GrantParts, PassOn, RecipientKind, Relation, Resource, Source, Window,
};
