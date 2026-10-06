//! Managed evidence shares the existing feed and its durable cursors.

use serde::{Deserialize, Serialize};

use crate::peer::Leader;

/// A launched executable, identified by its resolved path and bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Executable {
    /// The executable's resolved path.
    pub path: String,
    /// SHA-256 of its bytes.
    pub sha256: String,
}

/// The identities a managed channel is held under.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    /// The runner's session.
    pub session: String,
    /// The session's process generation.
    pub generation: u64,
    /// The proved process start identity.
    pub leader: Leader,
    /// The harness's conversation.
    pub conversation: String,
    /// The child-entry executable.
    pub entry: Executable,
    /// The harness executable selected for that child.
    pub harness: Executable,
    /// The version reported by that selected executable.
    pub harness_version: String,
    /// The versioned managed wire adapter.
    pub adapter: String,
}

/// An observation from the channel's single reader.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManagedEvent {
    /// The channel that supplied this observation.
    pub binding: Binding,
    /// The observation's kind.
    pub event: String,
    /// The harness turn, when it names one.
    pub turn: Option<String>,
    /// The stable operation, when this observation correlates it.
    pub operation: Option<String>,
}
