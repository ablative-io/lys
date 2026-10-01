//! The provisioning profiles as they are kept: one file holding, for each
//! agent, every version of what it is set up with, replaced whole and
//! atomically at each change. A start reads that one file and nothing else,
//! whatever was changed before.
//!
//! A change is written before it is answered. When a write fails, what the
//! file holds is read again before anything else is answered, so memory
//! never runs ahead of or behind the file.
//!
//! A version is named by the operation id it was set with and is set over
//! the version its setter saw. Set again in the same words it is kept once;
//! set over a version that is no longer the latest it is refused, so a late
//! change never lands on a profile its setter did not see.

use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use lys_home::harness::launch_fields::{Channel, DeclaredHarness, InstructionsMode, Literal};
use serde::{Deserialize, Serialize};

use crate::error::ServerError;
use crate::launch_permissions::Permissions;

#[path = "provisioning_builds.rs"]
mod builds;

#[path = "provisioning_edits.rs"]
mod edits;

#[path = "provisioning_index.rs"]
mod index;

#[cfg(test)]
std::thread_local! {
    static HISTORY_READS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static PROFILE_COPIES: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static SKILL_COPIES: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static HISTORY_VISITS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
fn index_work() -> (usize, usize, usize) {
    (
        PROFILE_COPIES.with(std::cell::Cell::get),
        SKILL_COPIES.with(std::cell::Cell::get),
        HISTORY_VISITS.with(std::cell::Cell::get),
    )
}

#[cfg(test)]
/// Count source access so projection tests detect a retained-history rescan.
pub(crate) fn history_reads() -> usize {
    HISTORY_READS.with(std::cell::Cell::get)
}

/// One MCP server an agent is given: reached at an address, or started as
/// a command. A version kept before commands has an address and no channel,
/// which reads as off.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct McpServer {
    /// Its name.
    pub name: String,
    /// Where it is reached; empty for a command.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub url: String,
    /// The command it is started with; none for an address.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<McpCommand>,
    /// Whether its messages wake an idle seat: `off` or `wake`.
    #[serde(default, skip_serializing_if = "Channel::is_off")]
    #[schema(value_type = String)]
    pub channel: Channel,
}

/// The command an MCP server is started with.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct McpCommand {
    /// The program.
    pub program: String,
    /// Its arguments, in order.
    #[serde(default)]
    pub args: Vec<String>,
    /// The directory it is started in, when one is named.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    /// Its environment by name: a string, integer or boolean setting, or
    /// `{"handle": "<secret>"}` for a secret the agent holds a handle on.
    #[serde(default)]
    #[schema(value_type = Object)]
    pub env: BTreeMap<String, Setting>,
}

/// One environment setting of a command server.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Setting {
    /// A secret, as the name of the secret whose handle the launch sets.
    Handle {
        /// The secret.
        handle: String,
    },
    /// A setting that is not a secret.
    Literal(Literal),
}

/// A skill Lys keeps: its text under its name, by the hash of its bytes.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SkillText {
    /// Its name, one visible path component.
    pub name: String,
    /// Its text, as the harness reads it from SKILL.md.
    pub text: String,
    /// The length of the text in bytes.
    pub len: u64,
    /// The SHA-256 of the text's bytes, as lowercase hex.
    pub sha256: String,
}

impl Clone for SkillText {
    fn clone(&self) -> Self {
        #[cfg(test)]
        SKILL_COPIES.with(|copies| copies.set(copies.get() + 1));
        Self {
            name: self.name.clone(),
            text: self.text.clone(),
            len: self.len,
            sha256: self.sha256.clone(),
        }
    }
}

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
}

/// How an agent's sessions are driven through a runner.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SessionSettings {
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

#[derive(Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Kept {
    profiles: Vec<Profile>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    skills: Vec<SkillText>,
}

/// The provisioning profiles, read from their file and written to it.
pub struct ProvisioningStore {
    path: PathBuf,
    kept: Kept,
    indexes: Arc<index::Indexes>,
    uncertain: bool,
}

fn unavailable(what: impl std::fmt::Display) -> ServerError {
    ServerError::ProvisioningUnavailable {
        reason: what.to_string(),
    }
}

fn read(path: &Path) -> Result<Kept, ServerError> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Kept::default());
        }
        Err(error) => {
            return Err(unavailable(format!("reading {}: {error}", path.display())));
        }
    };
    serde_json::from_slice(&bytes)
        .map_err(|error| unavailable(format!("{} does not read: {error}", path.display())))
}

fn replace(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let beside = path.with_extension("writing");
    let mut file = fs::File::create(&beside)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    drop(file);
    fs::rename(&beside, path)?;
    match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => fs::File::open(parent)?.sync_all(),
        _ => Ok(()),
    }
}

impl ProvisioningStore {
    /// The profiles kept in the file `path`, none when it does not exist.
    pub fn open(path: &Path) -> Result<Self, ServerError> {
        let kept = read(path)?;
        let indexes = Arc::new(index::Indexes::rebuild(&kept));
        Ok(Self {
            path: path.to_owned(),
            kept,
            indexes,
            uncertain: false,
        })
    }

    /// Resolve a write whose outcome is not known, by reading the file again.
    pub fn settle(&mut self) -> Result<(), ServerError> {
        if self.uncertain {
            let kept = read(&self.path)?;
            let indexes = Arc::new(index::Indexes::rebuild(&kept));
            self.kept = kept;
            self.indexes = indexes;
            self.uncertain = false;
        }
        Ok(())
    }

    fn publish(&mut self, bytes: &[u8]) -> Result<(), ServerError> {
        if let Err(failure) = replace(&self.path, bytes) {
            self.uncertain = true;
            self.settle()?;
            return Err(unavailable(format!(
                "writing {}: {failure}",
                self.path.display()
            )));
        }
        Ok(())
    }

    fn commit(&mut self, edit: edits::Edit) -> Result<(), ServerError> {
        let bytes = edits::encode(&self.kept, &edit)?;
        self.publish(&bytes)?;
        let changed = edits::apply(&mut self.kept, edit)
            .and_then(|change| Arc::make_mut(&mut self.indexes).update(&self.kept, change));
        if let Err(error) = changed {
            self.uncertain = true;
            self.settle()?;
            return Err(error);
        }
        Ok(())
    }

    #[cfg(test)]
    fn write(&mut self, next: Kept) -> Result<(), ServerError> {
        let bytes = serde_json::to_vec_pretty(&next).map_err(unavailable)?;
        self.publish(&bytes)?;
        self.indexes = Arc::new(index::Indexes::rebuild(&next));
        self.kept = next;
        Ok(())
    }

    /// Every profile, in the order first set.
    pub fn profiles(&self) -> &[Profile] {
        #[cfg(test)]
        HISTORY_READS.with(|reads| reads.set(reads.get() + 1));
        &self.kept.profiles
    }

    /// Distinct reviewed builds for `contract`, in their catalogue order.
    pub fn reviewed_builds(
        &self,
        contract: &str,
    ) -> impl Iterator<Item = &crate::harness_catalogue::BuildView> {
        self.indexes
            .reviewed_builds
            .get(contract)
            .into_iter()
            .flat_map(std::collections::BTreeSet::iter)
    }

    /// The profile of `agent`, none while nothing was set for it.
    pub fn profile(&self, agent: &str) -> Option<&Profile> {
        self.kept.profiles.get(*self.indexes.profiles.get(agent)?)
    }

    /// The numbered version of the named agent's profile.
    pub fn version(&self, agent: &str, number: u32) -> Option<&Version> {
        self.profile(agent)?
            .versions
            .iter()
            .inspect(|_| {
                #[cfg(test)]
                HISTORY_VISITS.with(|visits| visits.set(visits.get() + 1));
            })
            .find(|version| version.number == number)
    }

    /// The agent and the version `operation` names, if it names one.
    pub(crate) fn named(&self, operation: &str) -> Option<(&str, &Version)> {
        let &(profile, version) = self.indexes.operations.get(operation)?;
        let profile = self.kept.profiles.get(profile)?;
        Some((profile.agent.as_str(), profile.versions.get(version)?))
    }

    /// The first reviewed declaration in profile and version order.
    pub(crate) fn declared_server(&self, name: &str) -> Option<&McpServer> {
        let &(profile, version, server) = self.indexes.servers.get(name)?;
        self.kept
            .profiles
            .get(profile)?
            .versions
            .get(version)?
            .settings
            .mcp_servers
            .get(server)
    }

    /// The last reviewed version in the agent's recorded version order.
    pub(crate) fn latest_reviewed(&self, agent: &str) -> Option<&Version> {
        let profile = *self.indexes.profiles.get(agent)?;
        let version = *self.indexes.latest_reviewed.get(&profile)?;
        self.kept.profiles.get(profile)?.versions.get(version)
    }

    /// Keep `settings` as the version of `agent` after `from`, under
    /// `operation`, and answer its number. `from` is the latest version the
    /// setter saw, 0 for none.
    pub fn set(&mut self, agent: &str, from: u32, version: Version) -> Result<u32, ServerError> {
        self.settle()?;
        if let Some((kept_for, kept)) = self.named(&version.operation) {
            let same = kept_for == agent
                && kept.settings == version.settings
                && kept.number == from.saturating_add(1);
            return if same {
                Ok(kept.number)
            } else {
                Err(ServerError::ProvisioningReused {
                    operation: version.operation,
                })
            };
        }
        let latest = self.profile(agent).map_or(0, Profile::latest);
        if from != latest {
            return Err(ServerError::ProvisioningChanged { latest });
        }
        let number = latest.saturating_add(1);
        let version = Version { number, ..version };
        let profile = self
            .indexes
            .profiles
            .get(agent)
            .map_or(self.kept.profiles.len(), |position| *position);
        self.commit(edits::Edit::Set {
            profile,
            agent: agent.to_owned(),
            version: Box::new(version),
        })?;
        Ok(number)
    }

    /// Keep `review` of version `number` of `agent`'s profile. Sent again in
    /// the same words it is kept once; the operation naming any other act is
    /// refused, and a version reviewed already answers its review unchanged.
    pub fn review(&mut self, agent: &str, number: u32, review: Review) -> Result<(), ServerError> {
        self.settle()?;
        let reused = self
            .indexes
            .review_reused(&self.kept, agent, number, &review)?;
        if reused {
            return Err(ServerError::ProvisioningReused {
                operation: review.operation,
            });
        }
        let (profile, version) = self
            .indexes
            .location(agent, number)
            .ok_or(ServerError::ProfileVersionUnknown { version: number })?;
        let kept = self
            .kept
            .profiles
            .get(profile)
            .and_then(|profile| profile.versions.get(version))
            .ok_or_else(|| unavailable("review index is outside the retained profile"))?;
        if kept.reviewed.is_some() {
            return Ok(());
        }
        self.commit(edits::Edit::Review {
            profile,
            version,
            review,
        })
    }

    /// Keep `skill`; kept already with the same text, it is kept once.
    pub fn keep_skill(&mut self, skill: SkillText) -> Result<(), ServerError> {
        self.settle()?;
        if self.skill(&skill.name, &skill.sha256).is_some() {
            return Ok(());
        }
        self.commit(edits::Edit::Skill(skill))
    }

    /// Every kept skill text, in the order kept.
    pub fn skills(&self) -> &[SkillText] {
        &self.kept.skills
    }

    /// The text of `name` whose hash is `sha256`.
    pub fn skill(&self, name: &str, sha256: &str) -> Option<&SkillText> {
        let position = *self.indexes.skills.get(name)?.get(sha256)?;
        self.kept.skills.get(position)
    }

    /// The skill pins the version set under `operation` was recorded with,
    /// so a request sent again is compared as it was first recorded.
    pub fn pins_for(&self, operation: &str) -> Option<Vec<SkillPin>> {
        self.named(operation)
            .map(|(_, version)| version.settings.skill_pins.clone())
    }

    /// Each of `names` pinned to its latest kept text, refusing a name Lys
    /// keeps no text for.
    pub fn pins(&self, names: &[String]) -> Result<Vec<SkillPin>, ServerError> {
        names
            .iter()
            .map(|name| {
                self.indexes
                    .latest_skills
                    .get(name)
                    .and_then(|position| self.kept.skills.get(*position))
                    .map(|kept| SkillPin {
                        name: name.clone(),
                        len: kept.len,
                        sha256: kept.sha256.clone(),
                    })
                    .ok_or_else(|| ServerError::SkillUnknown { name: name.clone() })
            })
            .collect()
    }
}

#[cfg(test)]
#[path = "provisioning_compat_tests.rs"]
mod compatibility_tests;

#[cfg(test)]
#[path = "provisioning_builds_tests.rs"]
mod build_tests;

#[cfg(test)]
#[path = "provisioning_index_tests.rs"]
mod index_tests;
