//! The lys directory of people and agents.
//!
//! One signed committed directory event is both an identity change and its
//! audit record. This crate holds the typed records those events carry, the
//! versioned envelope they are signed in, and its canonical encoding. The
//! envelope lives here, outside `lys-core`, whose primitives and published
//! formats it uses and never changes.

pub mod binding;
pub mod checkpoints;
pub mod directory;
pub mod directory_migration;
mod directory_state;
pub mod draft_event;
pub mod encoding;
pub mod error;
pub mod event;
pub mod grants;
pub mod id;
pub mod import_document;
pub mod install_event;
pub mod lifecycle;
pub mod link_audit;
pub mod log;
pub mod message_service;
pub mod operation;
pub mod profile;
pub mod projection;
pub mod provenance;
pub mod receipt;
pub mod restart;
pub mod revocation;
pub mod signer;
pub mod start;
mod state_value;

pub use binding::LoginBinding;
pub use directory::Directory;
pub use error::IdentityError;
pub use event::{Change, IdentityEvent, LinkChange, LinkObservation};
pub use id::{AgentId, IdentityId, PersonId, ServiceAccountId};
pub use install_event::{InstallChange, InstallEvent};
pub use lifecycle::{LifecycleState, Transition};
pub use operation::OperationId;
pub use profile::Profile;
pub use provenance::{Actor, AuthMethod, Provenance};
pub use restart::SNAPSHOT_EVERY;
pub use signer::{
    Entry, SignedEvent, sign_draft_event, sign_event, sign_install_event, verify_event,
};
