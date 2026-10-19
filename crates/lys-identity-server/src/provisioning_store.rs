//! The provisioning profiles as they are kept: one file holding, for each
//! agent, every version of what it is set up with. A snapshot and appended
//! changes share that file, and a start reads both before answering.
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

use lys_home::harness::launch_fields::{Channel, Literal};
use serde::{Deserialize, Serialize};

use crate::error::ServerError;
mod profile;
mod stall;
pub use profile::{Profile, Review, SessionSettings, Settings, SkillPin, Version};

#[path = "provisioning_builds.rs"]
mod builds;

#[path = "provisioning_edits.rs"]
mod edits;

#[path = "provisioning_index.rs"]
mod index;

#[path = "provisioning_journal.rs"]
mod journal;

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
    journal: bool,
    since_snapshot: u64,
    uncertain: bool,
    stall_upgrades: journal::StallUpgrades,
}

fn unavailable(what: impl std::fmt::Display) -> ServerError {
    ServerError::ProvisioningUnavailable {
        reason: what.to_string(),
    }
}

#[cfg(test)]
fn read(path: &Path) -> Result<Kept, ServerError> {
    journal::read(path).map(|(kept, _, _, _)| kept)
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
    ///
    /// # Errors
    /// Refuses an unreadable provisioning file, malformed snapshot or journal,
    /// or a retained edit that cannot be folded into its profile.
    pub fn open(path: &Path) -> Result<Self, ServerError> {
        let (kept, journal, since_snapshot, stall_upgrades) = journal::read(path)?;
        let indexes = Arc::new(index::Indexes::rebuild(&kept));
        Ok(Self {
            path: path.to_owned(),
            kept,
            indexes,
            journal,
            since_snapshot,
            uncertain: false,
            stall_upgrades,
        })
    }

    /// Resolve a write whose outcome is not known, by reading the file again.
    ///
    /// # Errors
    /// Refuses when the provisioning file cannot be read back and folded after
    /// an uncertain write.
    pub fn settle(&mut self) -> Result<(), ServerError> {
        if self.uncertain {
            let (kept, journal, since_snapshot, stall_upgrades) = journal::settle(&self.path)?;
            let indexes = Arc::new(index::Indexes::rebuild(&kept));
            self.kept = kept;
            self.indexes = indexes;
            self.journal = journal;
            self.since_snapshot = since_snapshot;
            self.uncertain = false;
            self.stall_upgrades.extend(stall_upgrades);
        }
        Ok(())
    }

    #[cfg(test)]
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
        if self.since_snapshot >= lys_identity::SNAPSHOT_EVERY.get() {
            self.checkpoint()?;
        }
        let appended = self.journal;
        let written = if self.journal {
            journal::append(&self.path, &edit)
        } else {
            let bytes = edits::encode(&self.kept, &edit)?;
            journal::snapshot(&self.path, &bytes)
        };
        if let Err(error) = written {
            self.uncertain = true;
            self.settle()?;
            return Err(error);
        }
        self.journal = true;
        self.since_snapshot = if appended { self.since_snapshot + 1 } else { 0 };
        let changed = edits::apply(&mut self.kept, edit)
            .and_then(|change| Arc::make_mut(&mut self.indexes).update(&self.kept, change));
        if let Err(error) = changed {
            self.uncertain = true;
            self.settle()?;
            return Err(error);
        }
        if self.since_snapshot >= lys_identity::SNAPSHOT_EVERY.get() {
            self.checkpoint()?;
        }
        Ok(())
    }

    #[cfg(test)]
    fn write(&mut self, next: Kept) -> Result<(), ServerError> {
        let bytes = serde_json::to_vec_pretty(&next).map_err(unavailable)?;
        self.publish(&bytes)?;
        self.indexes = Arc::new(index::Indexes::rebuild(&next));
        self.kept = next;
        self.journal = false;
        self.since_snapshot = 0;
        Ok(())
    }

    /// Replace the change log with a durable snapshot during maintenance.
    ///
    /// # Errors
    /// Refuses an unsettled write, an unencodable snapshot, or a snapshot that
    /// cannot be durably written and read back.
    pub fn checkpoint(&mut self) -> Result<(), ServerError> {
        self.settle()?;
        let bytes = serde_json::to_vec(&self.kept)
            .map_err(|error| unavailable(format!("encoding provisioning checkpoint: {error}")))?;
        if let Err(error) = journal::snapshot(&self.path, &bytes) {
            self.uncertain = true;
            self.settle()?;
            return Err(unavailable(format!(
                "writing provisioning checkpoint: {error}"
            )));
        }
        self.journal = true;
        self.since_snapshot = 0;
        Ok(())
    }

    /// Every profile, in the order first set.
    #[must_use]
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
    #[must_use]
    pub fn profile(&self, agent: &str) -> Option<&Profile> {
        self.kept.profiles.get(*self.indexes.profiles.get(agent)?)
    }

    /// The numbered version of the named agent's profile.
    #[must_use]
    pub fn version(&self, agent: &str, number: u32) -> Option<&Version> {
        let (profile, version) = self.indexes.location(agent, number)?;
        self.kept.profiles.get(profile)?.versions.get(version)
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

    /// Every version reviewed itself whose settings have `digest`, with its
    /// agent, in the order kept (AGENTS-017 R3).
    pub(crate) fn reviewed_with<'a>(
        &'a self,
        digest: &str,
    ) -> impl Iterator<Item = (&'a str, &'a Version)> + 'a {
        self.indexes
            .reviewed_digests
            .get(digest)
            .into_iter()
            .flatten()
            .filter_map(|&(profile, version)| {
                let profile = self.kept.profiles.get(profile)?;
                Some((profile.agent.as_str(), profile.versions.get(version)?))
            })
    }

    /// Keep `settings` as the version of `agent` after `from`, under
    /// `operation`, and answer its number. `from` is the latest version the
    /// setter saw, 0 for none.
    ///
    /// # Errors
    /// Refuses a stall interval that is zero or cannot fit milliseconds,
    /// a reused operation or a changed latest version. Propagates settlement
    /// and durable journal failures.
    pub fn set(&mut self, agent: &str, from: u32, version: Version) -> Result<u32, ServerError> {
        if version.settings.session.as_ref().is_some_and(|session| {
            session.stall_after == 0 || session.stall_after.checked_mul(1000).is_none()
        }) {
            return Err(unavailable(
                "session.stall_after must be positive and fit milliseconds",
            ));
        }
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
    ///
    /// # Errors
    /// Refuses a reused operation, an unknown profile version or an index
    /// outside the retained profile. Propagates settlement and durable journal
    /// failures.
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
    ///
    /// # Errors
    /// Refuses when an uncertain provisioning write cannot be settled or the
    /// skill cannot be durably kept in the journal.
    pub fn keep_skill(&mut self, skill: SkillText) -> Result<(), ServerError> {
        self.settle()?;
        if self.skill(&skill.name, &skill.sha256).is_some() {
            return Ok(());
        }
        self.commit(edits::Edit::Skill(skill))
    }

    /// Every kept skill text, in the order kept.
    #[must_use]
    pub fn skills(&self) -> &[SkillText] {
        &self.kept.skills
    }

    /// The text of `name` whose hash is `sha256`.
    #[must_use]
    pub fn skill(&self, name: &str, sha256: &str) -> Option<&SkillText> {
        let position = *self.indexes.skills.get(name)?.get(sha256)?;
        self.kept.skills.get(position)
    }

    /// The skill pins the version set under `operation` was recorded with,
    /// so a request sent again is compared as it was first recorded.
    #[must_use]
    pub fn pins_for(&self, operation: &str) -> Option<Vec<SkillPin>> {
        self.named(operation)
            .map(|(_, version)| version.settings.skill_pins.clone())
    }

    /// Each of `names` pinned to its latest kept text, refusing a name Lys
    /// keeps no text for.
    ///
    /// # Errors
    /// Refuses each skill name for which the provisioning store keeps no text.
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

#[cfg(test)]
#[path = "provisioning_journal_tests.rs"]
mod journal_tests;
