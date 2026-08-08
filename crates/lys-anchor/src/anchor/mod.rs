//! The anchor itself — one log, its genesis leaf, and the facts it forwards.

pub mod append;
pub mod artifact;
pub mod checkpoint;
#[cfg(feature = "unstable-anchor")]
pub mod genesis;
pub mod open;
pub mod proof_nodes;
pub mod read_only;
pub mod status;
#[cfg(feature = "unstable-anchor")]
pub mod submit;

pub use checkpoint::PublishedCheckpoint;
#[cfg(feature = "unstable-anchor")]
pub use genesis::GENESIS_SEQUENCE;
pub use open::Anchor;
pub use proof_nodes::proof_nodes;
pub use read_only::{NoPolicy, NoSigner, ReadOnlyAnchor};
pub use status::{AnchorStatus, STANDALONE_DISCLOSURE, WitnessPosture};
