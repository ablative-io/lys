//! The profile declarations collected from a seat import.

use crate::launch_permissions::Permissions;
use crate::provisioning_store::McpServer;

/// The profile members one or more sources contributed, before they become
/// the profile version destination.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProfileParts {
    /// The models it may use.
    pub models: Vec<String>,
    /// Its instructions, appended to the harness's own prompt.
    pub instructions: Option<String>,
    /// The permissions its settings carried.
    pub permissions: Option<Permissions>,
    /// Its MCP servers.
    pub mcp_servers: Vec<McpServer>,
    /// The source entries that contributed.
    pub source_ids: Vec<String>,
}

impl ProfileParts {
    /// Whether no source contributed a member.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.models.is_empty()
            && self.instructions.is_none()
            && self.permissions.is_none()
            && self.mcp_servers.is_empty()
    }

    /// Takes `other`'s members alongside these.
    pub fn absorb(&mut self, other: Self) {
        self.models.extend(other.models);
        if other.instructions.is_some() {
            self.instructions = other.instructions;
        }
        if other.permissions.is_some() {
            self.permissions = other.permissions;
        }
        self.mcp_servers.extend(other.mcp_servers);
        self.source_ids.extend(other.source_ids);
    }
}
