//! What the connected-apps door keeps across restarts: the apps that
//! registered themselves, the agent each person's approval made for an app,
//! and the digests of the tokens issued to them. A token is kept only as its
//! SHA-256, so the file never holds a credential. The file is replaced whole
//! through a synced temporary file, so a crash leaves the old or the new
//! contents and never a torn one.

use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::ServerError;

/// An app that registered itself to connect to Lys's MCP door.
#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct App {
    pub(crate) name: String,
    pub(crate) redirect_uris: Vec<String>,
    pub(crate) registered_at: u64,
}

/// Which kind of token a digest is of.
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum Kind {
    Access,
    Refresh,
}

/// One issued token, by the digest of its value.
#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct Issued {
    pub(crate) agent: String,
    pub(crate) client_id: String,
    pub(crate) kind: Kind,
    pub(crate) expires_at: u64,
}

#[derive(Default, Serialize, Deserialize)]
struct Held {
    apps: BTreeMap<String, App>,
    /// The agent made for an app by a person's approval, by
    /// "client id, space, person id".
    connections: BTreeMap<String, String>,
    tokens: BTreeMap<String, Issued>,
}

/// How many apps may register, how fast, and how long one waits for a
/// person's approval before it is let go.
#[derive(Clone, Copy)]
pub(crate) struct Limits {
    pub(crate) most: usize,
    pub(crate) per_minute: usize,
    pub(crate) unapproved_seconds: u64,
}

/// The kept state and the file it is kept in.
pub(crate) struct Store {
    path: PathBuf,
    held: Held,
}

fn unavailable(reason: &str) -> ServerError {
    ServerError::ConfigInvalid {
        reason: format!("the connected-apps store is unavailable: {reason}"),
    }
}

impl Store {
    /// Open the store at `path`, empty when no file is there yet.
    pub(crate) fn open(path: &Path) -> Result<Self, ServerError> {
        let held = match std::fs::read(path) {
            Ok(bytes) => serde_json::from_slice(&bytes).map_err(|error| {
                unavailable(&format!("{} does not read: {error}", path.display()))
            })?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Held::default(),
            Err(error) => return Err(unavailable(&format!("{}: {error}", path.display()))),
        };
        Ok(Self {
            path: path.to_owned(),
            held,
        })
    }

    fn save(&self) -> Result<(), ServerError> {
        let bytes = serde_json::to_vec(&self.held)
            .map_err(|error| unavailable(&format!("the store does not render: {error}")))?;
        let temporary = self.path.with_extension("json.next");
        let mut file = std::fs::File::create(&temporary)
            .map_err(|error| unavailable(&format!("{}: {error}", temporary.display())))?;
        file.write_all(&bytes)
            .and_then(|()| file.sync_all())
            .map_err(|error| unavailable(&format!("{}: {error}", temporary.display())))?;
        std::fs::rename(&temporary, &self.path)
            .map_err(|error| unavailable(&format!("{}: {error}", self.path.display())))?;
        if let Some(parent) = self.path.parent() {
            std::fs::File::open(parent)
                .and_then(|directory| directory.sync_all())
                .map_err(|error| unavailable(&format!("{}: {error}", parent.display())))?;
        }
        Ok(())
    }

    /// The app registered under `client_id`.
    pub(crate) fn app(&self, client_id: &str) -> Option<&App> {
        self.held.apps.get(client_id)
    }

    /// Keep a newly registered app. An app no person has approved within
    /// `limits.unapproved_seconds` of registering is let go first, so an
    /// abandoned registration never holds a place; a registration beyond
    /// `limits.per_minute` in the last minute, or beyond `limits.most` apps
    /// held, is refused by name.
    pub(crate) fn register(
        &mut self,
        client_id: String,
        app: App,
        limits: Limits,
    ) -> Result<(), ServerError> {
        let at = app.registered_at;
        let connections = &self.held.connections;
        self.held.apps.retain(|id, kept| {
            kept.registered_at.saturating_add(limits.unapproved_seconds) > at
                || connections
                    .keys()
                    .any(|key| key.split_once(' ').is_some_and(|(client, _)| client == id))
        });
        let recent = self
            .held
            .apps
            .values()
            .filter(|kept| kept.registered_at.saturating_add(60) > at)
            .count();
        if recent >= limits.per_minute {
            return Err(ServerError::RegistrationThrottled);
        }
        if self.held.apps.len() >= limits.most {
            return Err(ServerError::RequestMalformed {
                reason: "this install holds as many connected apps as it keeps".to_owned(),
            });
        }
        self.held.apps.insert(client_id, app);
        self.save()
    }

    /// The agent a person's earlier approval made for an app.
    pub(crate) fn connection(&self, client_id: &str, person: &str) -> Option<&str> {
        self.held
            .connections
            .get(&format!("{client_id} {person}"))
            .map(String::as_str)
    }

    /// Keep the agent a person's approval made for an app.
    pub(crate) fn connect(
        &mut self,
        client_id: &str,
        person: &str,
        agent: String,
    ) -> Result<(), ServerError> {
        self.held
            .connections
            .insert(format!("{client_id} {person}"), agent);
        self.save()
    }

    /// The live token whose digest is `digest`, of the kind asked for.
    pub(crate) fn token(&self, digest: &str, kind: Kind, now: u64) -> Option<&Issued> {
        self.held
            .tokens
            .get(digest)
            .filter(|issued| issued.kind == kind && issued.expires_at > now)
    }

    /// Keep newly issued tokens, ending `spent` (a refresh token used to
    /// issue them) and every token past its instant, in one save.
    pub(crate) fn issue(
        &mut self,
        issued: Vec<(String, Issued)>,
        spent: Option<&str>,
        now: u64,
    ) -> Result<(), ServerError> {
        self.held.tokens.retain(|_, held| held.expires_at > now);
        if let Some(spent) = spent {
            self.held.tokens.remove(spent);
        }
        self.held.tokens.extend(issued);
        self.save()
    }
}

#[cfg(test)]
#[path = "mcp_oauth_store_tests.rs"]
mod tests;
