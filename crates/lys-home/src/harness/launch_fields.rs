//! What a recorded profile gives a launch, in words neither harness owns.
//!
//! The service converts a recorded profile version into these; each
//! harness's render reads them and writes what its own build takes, or
//! refuses by name what it cannot carry. Nothing here names a harness's
//! program, wrapper, package, version or file layout: those come from the
//! declared build. A secret appears only as a handle id, never a value.

use serde::{Deserialize, Serialize};

/// The harness build a profile declares, as the operator declared it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeclaredHarness {
    /// Its operator-chosen name, which never selects behavior.
    pub name: String,
    /// The pinned capabilities and rendering contract.
    pub description: super::description::Description,
    /// The program the operator declared, run as given.
    pub program: String,
    /// The package or build identity the program is verified against.
    pub package: String,
}

/// A nonsecret setting as the operator recorded it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Literal {
    /// A boolean.
    Boolean(bool),
    /// An integer.
    Integer(i64),
    /// Text.
    Text(String),
}

impl Literal {
    /// The environment text of this setting, by the one rule every render
    /// uses: text byte for byte, an integer in plain decimal, a boolean as
    /// `true` or `false`. Empty text stays set and empty.
    pub fn text(&self) -> String {
        match self {
            Self::Boolean(value) => value.to_string(),
            Self::Integer(value) => value.to_string(),
            Self::Text(value) => value.clone(),
        }
    }
}

/// Whether messages a server delivers wake an idle seat.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Channel {
    /// Its messages never wake the seat.
    #[default]
    Off,
    /// Its messages wake an idle seat.
    Wake,
}

impl Channel {
    /// An omitted policy in an older profile already means off.
    pub fn is_off(&self) -> bool {
        *self == Self::Off
    }
}

/// How reviewed instructions affect the program's own prompt.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstructionsMode {
    /// Keep the program's prompt without adding the profile's instructions.
    Keep,
    /// Add the profile's instructions to the program's prompt.
    #[default]
    Append,
    /// Use the profile's instructions in place of the program's prompt.
    Replace,
}

impl InstructionsMode {
    /// An omitted mode in an older profile already means append.
    pub fn is_append(&self) -> bool {
        *self == Self::Append
    }
}

/// One environment variable a command server is started with, as text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnvText {
    /// The variable.
    pub name: String,
    /// Its text, already turned from its literal by [`Literal::text`].
    pub text: String,
}

/// One environment variable a command server is given a handle in.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HandleEnv {
    /// The variable.
    pub name: String,
    /// The handle id the variable is set to, which is not the secret.
    pub handle_id: String,
}

/// How an MCP server is reached.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Transport {
    /// At an address.
    Http {
        /// The address.
        url: String,
    },
    /// By starting a program and speaking over its standard streams.
    Stdio {
        /// The program.
        program: String,
        /// Its arguments, in order.
        args: Vec<String>,
        /// The directory it is started in, when the profile names one.
        cwd: Option<String>,
        /// Its nonsecret settings, as text, by name.
        env: Vec<EnvText>,
        /// Its secrets, as handle ids, by name.
        handles: Vec<HandleEnv>,
    },
}

/// One MCP server a launch gives the session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LaunchMcp {
    /// Its name.
    pub name: String,
    /// How it is reached.
    pub transport: Transport,
    /// Whether its messages wake an idle seat.
    pub channel: Channel,
}

/// One skill Lys keeps, as the launch writes it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeptSkill {
    /// Its name.
    pub name: String,
    /// Where it is written, relative to the session's config directory.
    pub path: String,
    /// Its length in bytes.
    pub len: u64,
    /// The SHA-256 of its bytes, as lowercase hex.
    pub sha256: String,
}

/// Who a launch is for and what it is started from, as every render sets
/// it in the session's environment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LaunchIdentity {
    /// The agent.
    pub agent: String,
    /// The session it reports under.
    pub session: String,
    /// The machine it starts on.
    pub machine: String,
    /// The profile version it starts from.
    pub version: u32,
}

/// Everything a recorded profile gives a launch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LaunchFields {
    /// Who the launch is for.
    pub identity: LaunchIdentity,
    /// The declared build.
    pub harness: DeclaredHarness,
    /// The instructions; may be empty.
    pub instructions: String,
    /// The models, the session's own first and every further one in order.
    pub models: Vec<String>,
    /// The MCP servers.
    pub mcp_servers: Vec<LaunchMcp>,
    /// The kept skills.
    pub skills: Vec<KeptSkill>,
    /// Lys's model proxy for this harness's provider, as a base URL the run
    /// sends its model calls to; absent, the run reaches its provider as the
    /// machine's own setup says.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_proxy: Option<String>,
    /// The key minted for this start alone. With a model proxy, the run is
    /// given the proxy's address with this key first on its path, and the
    /// key itself in its environment, so the proxy says which run made each
    /// call. It says nothing without a model proxy.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run: Option<String>,
}
