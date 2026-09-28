//! Roles with versions: grant templates copied into grants when a holding
//! is granted, and a holder moved to a newer version only by a deliberate,
//! recorded act.
//!
//! A role is defined in exactly one project, and every holding of it sits
//! there. Only a change to its grant templates makes a version, and a
//! committed version never changes. A holding copies its version's templates
//! into grants at grant time, keeps its end date on those grants and never
//! extends it, and moves only by a move that shows what changes first, or at
//! a renewal under its own policy. Every act is one signed role event naming
//! the authenticated actor and the capacity it acted in, and every check is
//! answered through the one seam in [`check`]. Holdings are made for agents
//! only.
//!
//! The seven role event kinds are drafted in
//! docs/design/identity/IDENTITY-EVENTS.md for the envelope's joint review,
//! and until that review accepts them [`Roles`] holds its signed events in
//! memory: no role event is durably signed before then.

pub mod check;
pub mod edit;
pub mod error;
pub mod events;
pub mod holders;
pub mod holding;
pub mod move_holder;
pub mod ownership;
pub mod policy;
pub mod renewal;
#[cfg(any(test, feature = "test-support"))]
pub mod test_support;
pub mod types;

pub use check::{Asked, Check, Facts, RecordCheck, RoleCheck};
pub use edit::{ChangeTitle, EditTemplates, MakeRole};
pub use error::{RoleError, TemplateReason, TemplateRefusal};
pub use events::{
    Opening, RoleChange, RoleEvent, SignedRoleEvent, decode_event_body, decode_holding,
    decode_template, encode_event_body, encode_holding, encode_template, sign_role_event,
    verify_role_event,
};
pub use holders::{Holder, RoleBook, VersionHolders};
pub use holding::{Acting, Assign, Roles};
pub use move_holder::{ConfirmMove, GrantView, Preview};
pub use ownership::{GiveOwner, give_owner};
pub use policy::{ChangeDefault, ChangePolicy, HoldingPolicy, RolePolicies};
pub use renewal::Renew;
pub use types::{
    Capacity, Holding, HoldingId, MovePolicy, Role, RoleId, Template, TemplateParts, Timing,
    Version,
};
