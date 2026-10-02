//! The signed-in sessions as they are kept on disk, so a restart of the
//! service leaves everyone signed in.
//!
//! The file never holds a cookie secret: each session is keyed by the
//! SHA-256 of its secret, the same key the live sessions are looked up by.
//! The actor is written in a stored form of its own, named by `FORMAT`,
//! field by field: a change to `Actor` does not change what is on disk
//! until this form changes and its format with it. A file in another format
//! is refused by name rather than read as this one.
//!
//! The file is replaced whole: written beside itself with owner-only
//! permissions, synced, renamed over the old one, and the directory synced.
//! A session whose end has passed is dropped as the file is read.

use std::collections::HashMap;
use std::hash::BuildHasher;
use std::io::Write;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

use lys_identity::{Actor, AgentId, AuthMethod, LoginBinding, Provenance, ServiceAccountId};
use serde::{Deserialize, Serialize};

use crate::error::ServerError;
use crate::session::SessionEntry;

/// The stored form's name and version.
pub const FORMAT: &str = "lys-directory-sessions/v1";

/// How the actor was authenticated, as stored.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
enum StoredMethod {
    Oidc,
    AgentSignature,
    ServiceAccountBearer,
    AgentPass,
}

/// An actor, as stored.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredActor {
    issuer: String,
    subject: String,
    method: StoredMethod,
    agent: Option<String>,
    authenticated_at: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum RunMethod {
    AgentPass,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredRunActor {
    issuer: String,
    subject: String,
    method: RunMethod,
    agent: String,
    launch: String,
    session: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
enum SessionActor {
    Authenticated(StoredActor),
    AgentPass(StoredRunActor),
}

/// One session, as stored, under the SHA-256 of its cookie secret.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredSession {
    key: String,
    id: String,
    actor: SessionActor,
    started_at: u64,
    ends_at: u64,
}

/// The whole file.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredSessions {
    format: String,
    sessions: Vec<StoredSession>,
}

fn unavailable(path: &Path, what: &str) -> ServerError {
    ServerError::SessionsUnavailable {
        reason: format!("{}: {what}", path.display()),
    }
}

impl SessionActor {
    fn of(actor: &Actor) -> Result<Self, ServerError> {
        if let Provenance::AgentPass {
            agent,
            launch,
            session,
        } = actor.provenance()
        {
            return Ok(Self::AgentPass(StoredRunActor {
                issuer: actor.binding().issuer().to_owned(),
                subject: actor.binding().subject().to_owned(),
                method: RunMethod::AgentPass,
                agent: agent.to_string(),
                launch: launch.clone(),
                session: session.clone(),
            }));
        }
        StoredActor::of(actor).map(Self::Authenticated)
    }

    fn actor(&self, path: &Path) -> Result<Actor, ServerError> {
        match self {
            Self::Authenticated(actor) => actor.actor(path),
            Self::AgentPass(actor) => {
                let agent = actor.agent.parse::<AgentId>().map_err(|error| {
                    unavailable(path, &format!("a stored agent does not read: {error}"))
                })?;
                Ok(Actor::new(
                    LoginBinding::new(&actor.issuer, &actor.subject).map_err(|error| {
                        unavailable(path, &format!("a stored login does not read: {error}"))
                    })?,
                    Provenance::by_pass(agent, &actor.launch, &actor.session).map_err(|error| {
                        unavailable(
                            path,
                            &format!("stored pass provenance does not read: {error}"),
                        )
                    })?,
                ))
            }
        }
    }
}

impl StoredActor {
    fn of(actor: &Actor) -> Result<Self, ServerError> {
        let provenance = actor.provenance();
        let (method, agent) = match provenance.method() {
            AuthMethod::Operator => {
                return Err(ServerError::OperatorRefused {
                    reason: "an operator credential cannot create a personal session",
                });
            }
            AuthMethod::Oidc => (StoredMethod::Oidc, None),
            AuthMethod::AgentSignature(agent) => {
                (StoredMethod::AgentSignature, Some(agent.to_string()))
            }
            AuthMethod::AgentPass(agent) => (StoredMethod::AgentPass, Some(agent.to_string())),
            AuthMethod::ServiceAccountBearer(account) => (
                StoredMethod::ServiceAccountBearer,
                Some(account.to_string()),
            ),
        };
        Ok(Self {
            issuer: actor.binding().issuer().to_owned(),
            subject: actor.binding().subject().to_owned(),
            method,
            agent,
            authenticated_at: provenance.authenticated_at().ok_or_else(|| {
                ServerError::AgentPassRefused {
                    reason: "a run-pass actor cannot use timed session encoding".to_owned(),
                }
            })?,
        })
    }

    fn actor(&self, path: &Path) -> Result<Actor, ServerError> {
        let binding = LoginBinding::new(&self.issuer, &self.subject).map_err(|error| {
            unavailable(path, &format!("a stored login does not read: {error}"))
        })?;
        let method = match (&self.method, &self.agent) {
            (StoredMethod::Oidc, None) => AuthMethod::Oidc,
            (StoredMethod::AgentSignature, Some(agent)) => {
                AuthMethod::AgentSignature(agent.parse::<AgentId>().map_err(|error| {
                    unavailable(path, &format!("a stored agent does not read: {error}"))
                })?)
            }
            (StoredMethod::AgentPass, Some(agent)) => {
                AuthMethod::AgentPass(agent.parse::<AgentId>().map_err(|error| {
                    unavailable(path, &format!("a stored agent does not read: {error}"))
                })?)
            }
            (StoredMethod::ServiceAccountBearer, Some(account)) => {
                AuthMethod::ServiceAccountBearer(account.parse::<ServiceAccountId>().map_err(
                    |error| {
                        unavailable(
                            path,
                            &format!("a stored service account does not read: {error}"),
                        )
                    },
                )?)
            }
            _ => {
                return Err(unavailable(
                    path,
                    "a stored method and its agent do not agree",
                ));
            }
        };
        Ok(Actor::new(
            binding,
            Provenance::new(method, self.authenticated_at),
        ))
    }
}

/// The sessions kept at `path` that end after `now`, by key. No file is no
/// sessions: an install that kept none before starts with none.
pub fn load(path: &Path, now: u64) -> Result<HashMap<String, SessionEntry>, ServerError> {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(HashMap::new()),
        Err(error) => return Err(unavailable(path, &format!("could not be read: {error}"))),
    };
    let stored: StoredSessions = serde_json::from_slice(&bytes)
        .map_err(|error| unavailable(path, &format!("does not read: {error}")))?;
    if stored.format != FORMAT {
        return Err(unavailable(
            path,
            &format!("is in format {}, not {FORMAT}", stored.format),
        ));
    }
    let mut live = HashMap::new();
    for session in stored.sessions {
        if session.ends_at <= now {
            continue;
        }
        let entry = SessionEntry {
            id: session.id,
            actor: session.actor.actor(path)?,
            started_at: session.started_at,
            ends_at: session.ends_at,
        };
        live.insert(session.key, entry);
    }
    Ok(live)
}

/// Replace the file at `path` with `live`, owner-only.
pub fn save<S: BuildHasher>(
    path: &Path,
    live: &HashMap<String, SessionEntry, S>,
) -> Result<(), ServerError> {
    let mut sessions: Vec<StoredSession> = live
        .iter()
        .map(|(key, entry)| {
            Ok(StoredSession {
                key: key.clone(),
                id: entry.id.clone(),
                actor: SessionActor::of(&entry.actor)?,
                started_at: entry.started_at,
                ends_at: entry.ends_at,
            })
        })
        .collect::<Result<_, ServerError>>()?;
    sessions.sort_by(|a, b| a.key.cmp(&b.key));
    let stored = StoredSessions {
        format: FORMAT.to_owned(),
        sessions,
    };
    let bytes = serde_json::to_vec(&stored)
        .map_err(|error| unavailable(path, &format!("could not be written: {error}")))?;
    let temporary = beside(path);
    let written = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(&temporary)
        .and_then(|mut file| {
            file.set_permissions(std::fs::Permissions::from_mode(0o600))?;
            file.write_all(&bytes)?;
            file.sync_all()
        })
        .and_then(|()| std::fs::rename(&temporary, path));
    written.map_err(|error| unavailable(path, &format!("could not be written: {error}")))?;
    if let Some(parent) = path.parent() {
        std::fs::File::open(parent)
            .and_then(|directory| directory.sync_all())
            .map_err(|error| {
                unavailable(path, &format!("its directory could not be synced: {error}"))
            })?;
    }
    Ok(())
}

/// The temporary file a save writes before renaming it over `path`.
fn beside(path: &Path) -> PathBuf {
    let mut name = path.file_name().map(ToOwned::to_owned).unwrap_or_default();
    name.push(".writing");
    path.with_file_name(name)
}

#[cfg(test)]
#[path = "session_provenance_tests.rs"]
mod tests;
