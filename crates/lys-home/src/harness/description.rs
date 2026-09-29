//! The operator's pinned harness description. Names are labels; capabilities
//! and a rendering contract describe what a build can represent.
use serde::{Deserialize, Serialize};

use super::launch_fields::Channel;

/// How a build carries models after the first one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum FurtherModels {
    /// Each model is a separate array member.
    Array,
    /// Models share a string, separated by these exact bytes.
    Delimited {
        /// The separator; a model containing it cannot be represented.
        separator: String,
    },
}

/// The model count and representation the described build accepts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Models {
    /// The fewest models that may be supplied.
    pub minimum: usize,
    /// The upper bound, explicitly null when the build declares none.
    #[serde(deserialize_with = "Option::<usize>::deserialize")]
    pub maximum: Option<usize>,
    /// How further models are carried, without dropping any.
    pub further_encoding: FurtherModels,
}

/// Permission values and rule grammars the described build accepts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Permissions {
    /// The mode names, exactly as the build accepts them.
    pub modes: Vec<String>,
    /// The rule grammars supported by its rendering contract.
    pub rule_forms: Vec<String>,
}

/// MCP capabilities; none are inferred from the harness's display name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Mcp {
    /// The transport names the build accepts.
    pub transports: Vec<String>,
    /// Whether a command server may set its own working directory.
    pub working_directory: bool,
    /// Whether a command server may receive secret-handle variables.
    pub handle_variables: bool,
    /// Which channel policies the build supports.
    pub channel_policies: Vec<Channel>,
}

/// Versioned with the profile, and checked against the actual installed
/// build before launching. Describing a capability does not prove it exists.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Description {
    /// The model cardinality and serialization contract.
    pub models: Models,
    /// The permissions contract.
    pub permissions: Permissions,
    /// The MCP transport and channel contract.
    pub mcp: Mcp,
    /// The identifier resolved only by the home's rendering registry.
    pub rendering_contract: String,
}
