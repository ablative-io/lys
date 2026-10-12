//! The reviewed profile shape, including optional session declarations.

use super::McpServer;
#[cfg(test)]
use super::PROFILE_COPIES;
use crate::launch_permissions::Permissions;
use lys_home::harness::launch_fields::{DeclaredHarness, InstructionsMode, SeatFields};
use serde::{Deserialize, Serialize};

/// The skill a profile version names, pinned to the text it was recorded with.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SkillPin {
    /// The skill's name.
    pub name: String,
    /// The length of that text in bytes.
    pub len: u64,
    /// The SHA-256 of the text the version was recorded with.
    pub sha256: String,
}

/// What an agent is set up with.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    /// The models it may use.
    pub model_access: Vec<String>,
    /// The tools it is given.
    pub tools: Vec<String>,
    /// The skills it is given.
    pub skills: Vec<String>,
    /// The MCP servers it is given.
    pub mcp_servers: Vec<McpServer>,
    /// The harness plugins its settings turn on, each `name@marketplace`,
    /// reviewed with the rest of the version, so the person's config cannot
    /// load an unreviewed plugin.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub plugins: Vec<String>,
    /// Commands appended to each declared Claude hook event.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hooks: Option<
        std::collections::BTreeMap<String, Vec<lys_home::harness::launch_fields::HookCommand>>,
    >,
    /// The seat's own status command, preserved as declared.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status_line: Option<String>,
    /// Extra native settings without the separately governed keys.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settings: Option<serde_json::Map<String, serde_json::Value>>,
    /// The existing conversation explicitly selected for resume.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resume: Option<String>,
    /// The context window explicitly declared for this seat, in tokens.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_window: Option<u32>,
    /// The absolute harness config directory explicitly chosen by a person.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config_dir: Option<String>,
    /// Its instructions; may be empty.
    pub instructions: String,
    /// Whether to keep, append to or replace the program's prompt.
    #[serde(default, skip_serializing_if = "InstructionsMode::is_append")]
    pub instructions_mode: InstructionsMode,
    /// Why this version was set; may be empty.
    pub note: String,
    /// How its sessions are driven through a runner, when the profile says.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<SessionSettings>,
    /// The harness build it is started with, as the operator declared it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub harness: Option<DeclaredHarness>,
    /// Each named skill's text as it was when this version was recorded.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub skill_pins: Vec<SkillPin>,
    /// The permissions its settings file carries, when the profile sets them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub permissions: Option<Permissions>,
    /// The machine the reviewed version starts on when no machine is requested.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runs_on: Option<String>,
    /// The single folder the reviewed version permits the seat to write.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub writable: Option<String>,
    /// The folder the agent's runs start in when a launch names none: this
    /// agent's own default. Absent, a launch that names no folder is refused by name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub working_folder: Option<String>,
}

impl Settings {
    /// Assemble the declared seat fields, preserving the existing session form.
    ///
    /// # Errors
    /// Names a field declared differently at profile and session level.
    pub fn seat_fields(&self) -> Result<SeatFields, String> {
        let session = self.session.as_ref();
        Ok(SeatFields {
            hooks: self.hooks.clone(),
            status_line: self.status_line.clone(),
            settings: self.settings.clone(),
            resume: chosen(
                "resume",
                self.resume.as_ref(),
                session.and_then(|session| session.resume.as_ref()),
            )?,
            context_window: chosen(
                "context_window",
                self.context_window.as_ref(),
                session.and_then(|session| session.context_window.as_ref()),
            )?,
            config_dir: chosen(
                "config_dir",
                self.config_dir.as_ref(),
                session.and_then(|session| session.config_dir.as_ref()),
            )?,
        })
    }
}

fn chosen<T: Clone + PartialEq>(
    field: &str,
    profile: Option<&T>,
    session: Option<&T>,
) -> Result<Option<T>, String> {
    match (profile, session) {
        (Some(profile), Some(session)) if profile != session => {
            Err(format!("{field}: conflicts with session.{field}"))
        }
        (Some(value), _) | (None, Some(value)) => Ok(Some(value.clone())),
        (None, None) => Ok(None),
    }
}

/// How an agent's sessions are driven through a runner.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SessionSettings {
    /// The existing Claude session or Codex thread explicitly selected for resume.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resume: Option<String>,
    /// The context window explicitly declared for this seat, in tokens.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_window: Option<u32>,
    /// The absolute harness config directory explicitly chosen by a person.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config_dir: Option<String>,
    /// Whether starts require a qualified managed control adapter.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub requires_controls: bool,
    /// The harness's compaction command, typed as one line; none when the
    /// harness has none, and then a compaction is refused by name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compact: Option<String>,
    /// What is typed before a message to deliver it, as the harness takes
    /// one; empty for none. The message follows it, then Enter.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub message_prefix: String,
    /// Whether the session is sensitive. No receipt carries typed text for
    /// any session: text is kept as its length and digest alone.
    #[serde(default)]
    pub sensitive: bool,
    /// The account handles the session moves between at a usage limit,
    /// the variable the handle in use is set in, and what says a limit.
    /// Handles only: the broker swaps each for its account on the way out.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<Object>)]
    pub accounts: Option<lys_runner::Rotation>,
}

/// One version of an agent's profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Version {
    /// Its number, from 1.
    pub number: u32,
    /// The operation id it was set with.
    pub operation: String,
    /// What it sets.
    pub settings: Settings,
    /// The person who set it.
    pub set_by: String,
    /// When it was set, in seconds since the Unix epoch.
    pub set_at: u64,
    /// Its review, null until someone answering for the agent reviewed it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reviewed: Option<Review>,
}

/// A profile version's review.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Review {
    /// The operation id it was recorded with.
    pub operation: String,
    /// The person who reviewed it.
    pub by: String,
    /// When, in seconds since the Unix epoch.
    pub at: u64,
}

/// An agent's profile: every version, in order.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    /// The agent.
    pub agent: String,
    /// Its versions, in order from 1.
    pub versions: Vec<Version>,
}

impl Clone for Profile {
    fn clone(&self) -> Self {
        #[cfg(test)]
        PROFILE_COPIES.with(|copies| copies.set(copies.get() + 1));
        Self {
            agent: self.agent.clone(),
            versions: self.versions.clone(),
        }
    }
}

impl Profile {
    /// The number of the latest version.
    pub fn latest(&self) -> u32 {
        self.versions.last().map_or(0, |version| version.number)
    }
}
