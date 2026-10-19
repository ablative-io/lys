//! Where this configuration keeps each named store, and what is found there,
//! read without creating, writing or migrating anything.

use std::path::{Path, PathBuf};

use lys_core::Ed25519Identity;
use lys_identity::stores_receipt::{self as receipt, ComparedBy, Found, LogIdentity, Named};
use lys_log_store::{FileLeafStore, LeafStore, StoreError};

use crate::config::Config;

/// A named store and where this configuration keeps it.
#[derive(Debug, Clone)]
pub struct Place {
    /// Its name, class and comparison.
    pub named: &'static Named,
    /// Where it is kept.
    pub path: PathBuf,
}

fn at(name: &str, path: PathBuf) -> Option<Place> {
    receipt::store(name).map(|named| Place { named, path })
}

/// Every named store this configuration keeps, where it keeps it: the
/// service's own store-opening table. A store the configuration leaves out
/// has no place.
#[must_use]
pub fn places(config: &Config) -> Vec<Place> {
    let beside = |name: &str| config.log_dir.with_file_name(name);
    let mut found = vec![
        at("directory-log", config.log_dir.clone()),
        at("organisation", beside("organisation")),
        at("grant-log", config.grant_log_dir.clone()),
        at("service-key", config.event_key_file.clone()),
        at("apps", config.apps_dir()),
        at("cord", beside("cord")),
        at("runner-acts", beside("runner-acts")),
        at("seats", beside("seats")),
        at("seat-imports", beside("seat-imports")),
        at("launch-records", beside("launch-records")),
        config
            .log_dir
            .parent()
            .and_then(|folder| at("mcp-connected-apps", folder.join("connected-apps.json"))),
    ];
    if config.spicedb.is_some() {
        found.push(at("permission-projection", config.grant_log_dir.clone()));
    }
    if let Some(dir) = &config.certificates_dir {
        found.push(at("certificates", dir.clone()));
        found.push(at("agent-keys", dir.with_file_name("agent-keys")));
    }
    if let Some(file) = &config.network_file {
        found.push(at("network", file.clone()));
        found.push(at("network-joins", crate::network_join::beside(file)));
    }
    if let Some(dir) = &config.requests_dir {
        found.push(at("requests", dir.clone()));
        found.push(at("mcp-requests", dir.with_file_name("mcp-requests")));
    }
    for (name, path) in [
        ("roles", &config.roles_file),
        ("provisioning", &config.provisioning_file),
        ("homes", &config.homes_dir),
        ("runtime", &config.runtime_dir),
        ("service-accounts", &config.service_accounts_dir),
        ("reviews", &config.reviews_dir),
        ("teams", &config.teams_dir),
        ("stops", &config.stops_dir),
        ("budgets", &config.budgets_dir),
        ("policies", &config.policies_dir),
        ("goals", &config.goals_dir),
        ("words", &config.words_dir),
        ("variables", &config.variables_dir),
        ("schedules", &config.schedules_dir),
    ] {
        if let Some(path) = path {
            found.push(at(name, path.clone()));
        }
    }
    if let Some(run) = config.runner_socket.as_deref().and_then(Path::parent) {
        found.push(at("runner-join", run.join("runner-join.json")));
        found.push(at("runner-server-key", run.join("runner-server.pub")));
        found.push(at(
            "runner-machine-key",
            config.event_key_file.with_file_name("runner-machine.key"),
        ));
        found.push(at(
            "runner-owners",
            beside("runner").join(lys_runner::seat_owner::protocol::OWNERS_DIR),
        ));
    }
    found.into_iter().flatten().collect()
}

/// What was found where a store is kept.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Observed {
    /// Nothing is there.
    Absent,
    /// The store, at this head.
    Found(Found),
    /// Something is there that cannot be read now, and why: it is compared
    /// once it is opened, never taken for absent.
    Unread(String),
}

fn found(head: u64, digest: String) -> Observed {
    Observed::Found(Found {
        head,
        digest,
        identity: None,
    })
}

/// An append-only log's origin digest and head, read from its last segment.
fn log_head(path: &Path) -> Observed {
    if !path.exists() {
        return Observed::Absent;
    }
    match FileLeafStore::open_read_only(path) {
        Ok(store) => found(store.extent(), receipt::digest(store.origin().as_bytes())),
        Err(StoreError::NotInitialized { .. }) => Observed::Absent,
        Err(error) => Observed::Unread(error.to_string()),
    }
}

/// A one-file store's format digest; its records are counted by the store
/// once it is opened.
fn journal(path: &Path) -> Observed {
    match std::fs::read(path) {
        Ok(bytes) => {
            let first = bytes
                .split(|byte| *byte == b'\n')
                .next()
                .unwrap_or_default();
            let header = if first.starts_with(b"{") {
                &[][..]
            } else {
                first
            };
            found(0, receipt::digest(header))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Observed::Absent,
        Err(error) => Observed::Unread(error.to_string()),
    }
}

/// A key's public digest.
fn key(path: &Path) -> Observed {
    if !path.exists() {
        return Observed::Absent;
    }
    match Ed25519Identity::load(path) {
        Ok(key) => found(0, receipt::digest(&key.public_key_bytes())),
        Err(error) => Observed::Unread(error.to_string()),
    }
}

/// What is found at `place`, read without changing anything. The grant
/// log carries the identity recorded beside it.
#[must_use]
pub fn observe(place: &Place) -> Observed {
    let observed = match place.named.compared_by {
        ComparedBy::LogHead => log_head(&place.path),
        ComparedBy::JournalHead => journal(&place.path),
        ComparedBy::Digest => key(&place.path),
        ComparedBy::Presence if place.path.exists() => found(0, String::new()),
        ComparedBy::Presence => Observed::Absent,
        ComparedBy::ProjectedRevision => {
            Observed::Unread("compared once the grants are opened".to_owned())
        }
    };
    match (place.named.name, observed) {
        ("grant-log", Observed::Found(mut read)) => {
            match crate::grant_changes::recorded(&place.path) {
                Ok(name) => {
                    read.identity = name.map(|name| LogIdentity {
                        identity: name.identity,
                        epoch: name.epoch,
                    });
                    Observed::Found(read)
                }
                Err(error) => Observed::Unread(error.to_string()),
            }
        }
        (_, observed) => observed,
    }
}
