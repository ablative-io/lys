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

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::ServerError;

/// One MCP server an agent is given.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct McpServer {
    /// Its name.
    pub name: String,
    /// Where it is reached.
    pub url: String,
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
    /// Why this version was set; may be empty.
    pub note: String,
    /// How its sessions are driven through a runner, when the profile says.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<SessionSettings>,
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    /// The agent.
    pub agent: String,
    /// Its versions, in order from 1.
    pub versions: Vec<Version>,
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
}

/// The provisioning profiles, read from their file and written to it.
pub struct ProvisioningStore {
    path: PathBuf,
    kept: Kept,
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
        Ok(Self {
            path: path.to_owned(),
            kept: read(path)?,
            uncertain: false,
        })
    }

    /// Resolve a write whose outcome is not known, by reading the file again.
    pub fn settle(&mut self) -> Result<(), ServerError> {
        if self.uncertain {
            self.kept = read(&self.path)?;
            self.uncertain = false;
        }
        Ok(())
    }

    fn write(&mut self, next: Kept) -> Result<(), ServerError> {
        let bytes = serde_json::to_vec_pretty(&next).map_err(unavailable)?;
        if let Err(failure) = replace(&self.path, &bytes) {
            self.uncertain = true;
            self.settle()?;
            return Err(unavailable(format!(
                "writing {}: {failure}",
                self.path.display()
            )));
        }
        self.kept = next;
        Ok(())
    }

    /// Every profile, in the order first set.
    pub fn profiles(&self) -> &[Profile] {
        &self.kept.profiles
    }

    /// The profile of `agent`, none while nothing was set for it.
    pub fn profile(&self, agent: &str) -> Option<&Profile> {
        self.kept
            .profiles
            .iter()
            .find(|profile| profile.agent == agent)
    }

    /// The agent and the version `operation` names, if it names one.
    fn named(&self, operation: &str) -> Option<(&str, &Version)> {
        self.kept.profiles.iter().find_map(|profile| {
            profile
                .versions
                .iter()
                .find(|version| version.operation == operation)
                .map(|version| (profile.agent.as_str(), version))
        })
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
        let mut profiles = self.kept.profiles.clone();
        match profiles.iter_mut().find(|profile| profile.agent == agent) {
            Some(profile) => profile.versions.push(version),
            None => profiles.push(Profile {
                agent: agent.to_owned(),
                versions: vec![version],
            }),
        }
        self.write(Kept { profiles })?;
        Ok(number)
    }

    /// Keep `review` of version `number` of `agent`'s profile. Sent again in
    /// the same words it is kept once; the operation naming any other act is
    /// refused, and a version reviewed already answers its review unchanged.
    pub fn review(&mut self, agent: &str, number: u32, review: Review) -> Result<(), ServerError> {
        self.settle()?;
        let reused = self.kept.profiles.iter().any(|profile| {
            profile.versions.iter().any(|version| {
                let reviewed_here = profile.agent == agent && version.number == number;
                version.operation == review.operation
                    || version.reviewed.as_ref().is_some_and(|kept| {
                        kept.operation == review.operation
                            && !(reviewed_here && kept.by == review.by)
                    })
            })
        });
        if reused {
            return Err(ServerError::ProvisioningReused {
                operation: review.operation,
            });
        }
        let mut profiles = self.kept.profiles.clone();
        let version = profiles
            .iter_mut()
            .find(|profile| profile.agent == agent)
            .and_then(|profile| {
                profile
                    .versions
                    .iter_mut()
                    .find(|version| version.number == number)
            })
            .ok_or(ServerError::ProfileVersionUnknown { version: number })?;
        if version.reviewed.is_some() {
            return Ok(());
        }
        version.reviewed = Some(review);
        self.write(Kept { profiles })
    }
}
