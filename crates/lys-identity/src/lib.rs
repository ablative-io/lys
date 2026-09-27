//! The lys directory of people and agents.
//!
//! One signed committed directory event is both an identity change and its
//! audit record. This crate holds the typed records those events carry, the
//! versioned envelope they are signed in, and its canonical encoding. The
//! envelope lives here, outside `lys-core`, whose primitives and published
//! formats it uses and never changes.

pub mod binding;
pub mod directory;
pub mod encoding;
pub mod error;
pub mod event;
pub mod grants;
pub mod id;
pub mod lifecycle;
pub mod link_audit;
pub mod log;
pub mod operation;
pub mod profile;
pub mod projection;
pub mod provenance;
pub mod receipt;
pub mod signer;

pub use binding::LoginBinding;
pub use directory::Directory;
pub use error::IdentityError;
pub use event::{Change, IdentityEvent, LinkChange, LinkObservation};
pub use id::{AgentId, IdentityId, PersonId};
pub use lifecycle::{LifecycleState, Transition};
pub use operation::OperationId;
pub use profile::Profile;
pub use provenance::{Actor, AuthMethod, Provenance};
pub use signer::{SignedEvent, sign_event, verify_event};
