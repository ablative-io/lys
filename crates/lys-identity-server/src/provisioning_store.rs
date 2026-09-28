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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
}
