//! Each agent's tool-boundary policy as it is kept: a leaf store of its
//! own, one leaf for each version set, written before it is answered. A
//! version is immutable once kept; a change is the next version, and it
//! applies from the agent's next launch.
//!
//! A policy has one canonical encoding: a domain line, then its JSON with
//! its members in their declared order and its rules in the order given,
//! since the judge reads them in that order. Its digest is the SHA-256 of
//! that encoding, computed from the policy itself and never taken from a
//! caller.
//!
//! An append that fails is settled by opening the leaf store again and
//! reading what it holds, before anything else is read or written. What the
//! leaves fold to is sealed in the log's signed snapshot every
//! [`SNAPSHOT_EVERY`] leaves, so a start reads the snapshot and only the
//! leaves after it; a snapshot refused sends the start to every leaf, by
//! name.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use lys_core::Ed25519Identity;
use lys_identity::SNAPSHOT_EVERY;
use lys_log_store::{
    FileLeafStore, FrontierLog, LeafStore, SnapshotRefusal, Start, StoreResult, start,
};
use lys_runner::judge::Policy;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::config::Config;
use crate::error::ServerError;
use crate::routes::{Say, hex};

/// The origin the policies' leaf store is created with.
pub const ORIGIN: &str = "lys/identity/agent-policies";

/// The snapshot domain the policies' folded state is sealed under.
pub const DOMAIN: &str = "lys/identity/agent-policies-state/v1";

/// The domain line a policy's canonical encoding begins with.
pub const ENCODING: &str = "lys-agent-policy/v1";

const FORMAT: &str = "lys-agent-policies-state/v1";

/// `policy`'s canonical encoding.
pub fn canonical(policy: &Policy) -> Result<Vec<u8>, ServerError> {
    let mut bytes = ENCODING.as_bytes().to_vec();
    bytes.push(b'\n');
    serde_json::to_writer(&mut bytes, policy).map_err(unavailable)?;
    Ok(bytes)
}

/// `policy`'s digest: the lowercase hex SHA-256 of its canonical encoding.
pub fn digest(policy: &Policy) -> Result<String, ServerError> {
    Ok(hex(&Sha256::digest(canonical(policy)?)))
}

/// Every version of every agent's policy, as the log folds them.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Held {
    /// Each agent's versions, oldest first.
    pub policies: BTreeMap<String, Vec<Policy>>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Sealed {
    format: String,
    held: Held,
}

impl Held {
    /// `agent`'s latest version.
    pub fn latest(&self, agent: &str) -> Option<&Policy> {
        self.policies
            .get(agent)
            .and_then(|versions| versions.last())
    }

    /// Fold one kept version; one that does not follow the version held is
    /// refused, since every version was checked before it was kept.
    fn hold(&mut self, policy: Policy) -> Result<(), String> {
        let held = self.latest(&policy.agent).map_or(0, |held| held.version);
        if policy.version != held + 1 {
            return Err(format!(
                "policy version {} of {} does not follow version {held}",
                policy.version, policy.agent
            ));
        }
        self.policies
            .entry(policy.agent.clone())
            .or_default()
            .push(policy);
        Ok(())
    }

    fn fold(&mut self, tail: &lys_log_store::Tail) -> Result<(), String> {
        for (index, bytes) in (tail.from..).zip(&tail.leaves) {
            let policy = serde_json::from_slice(bytes)
                .map_err(|error| format!("leaf {index} is not a policy: {error}"))?;
            self.hold(policy)
                .map_err(|reason| format!("leaf {index}: {reason}"))?;
        }
        Ok(())
    }

    fn encode(&self) -> Result<Vec<u8>, String> {
        serde_json::to_vec(&Sealed {
            format: FORMAT.to_owned(),
            held: self.clone(),
        })
        .map_err(|error| format!("policies state: {error}"))
    }

    fn decode(bytes: &[u8]) -> Result<Self, String> {
        let sealed: Sealed =
            serde_json::from_slice(bytes).map_err(|error| format!("policies state: {error}"))?;
        if sealed.format != FORMAT {
            return Err(format!(
                "policies state is in format {}, not {FORMAT}",
                sealed.format
            ));
        }
        Ok(sealed.held)
    }
}

/// How the leaf store is opened again after an append whose outcome is not known.
pub type Reopen<S> = Box<dyn Fn() -> StoreResult<S> + Send>;

/// The policies, read from their leaf store and appended to it.
pub struct PolicyStore<S: LeafStore = FileLeafStore> {
    reopen: Reopen<S>,
    key: Arc<Ed25519Identity>,
    log: FrontierLog<S>,
    held: Held,
    start: Start,
    since_snapshot: u64,
    snapshot_failure: Option<String>,
    uncertain: bool,
}

type Opened<S> = (FrontierLog<S>, Held, Start);

fn unavailable(what: impl std::fmt::Display) -> ServerError {
    ServerError::PolicyUnavailable {
        reason: what.to_string(),
    }
}

impl PolicyStore<FileLeafStore> {
    /// The policies in the directory `config` names; none when it names none.
    pub fn configured(
        config: &Config,
        key: Arc<Ed25519Identity>,
        say: &Say,
    ) -> Result<Option<Self>, ServerError> {
        let Some(dir) = config.policies_dir.as_deref() else {
            return Ok(None);
        };
        let store = Self::open(dir, key)?;
        say(&format!(
            "agent policies log {}, holding policies for {} agents",
            store.start(),
            store.held.policies.len()
        ));
        Ok(Some(store))
    }

    /// The policies kept in `dir`, created when it does not exist.
    pub fn open(dir: &Path, key: Arc<Ed25519Identity>) -> Result<Self, ServerError> {
        if !dir.exists() {
            FileLeafStore::create(dir, ORIGIN).map_err(unavailable)?;
        }
        let dir = dir.to_owned();
        Self::over(Box::new(move || FileLeafStore::open(&dir)), key)
    }
}

impl<S: LeafStore> PolicyStore<S> {
    /// The policies kept in the leaf store `reopen` opens.
    pub fn over(reopen: Reopen<S>, key: Arc<Ed25519Identity>) -> Result<Self, ServerError> {
        let (log, held, start) = opened(&reopen, &key)?;
        let mut store = Self {
            reopen,
            key,
            log,
            held,
            start: start.clone(),
            since_snapshot: 0,
            snapshot_failure: None,
            uncertain: false,
        };
        store.after_start(&start);
        Ok(store)
    }

    /// How the log was started.
    pub fn start(&self) -> &Start {
        &self.start
    }

    /// Why the last snapshot could not be written, while no later one was.
    pub fn snapshot_failure(&self) -> Option<&str> {
        self.snapshot_failure.as_deref()
    }

    /// The policies as held.
    pub fn held(&self) -> &Held {
        &self.held
    }

    fn after_start(&mut self, start: &Start) {
        match start {
            Start::Resumed { replayed, .. } => self.since_snapshot = *replayed,
            Start::Rebuilt { .. } => self.write_snapshot(),
        }
        if self.since_snapshot >= SNAPSHOT_EVERY.get() {
            self.write_snapshot();
        }
    }

    fn write_snapshot(&mut self) {
        let written = self.held.encode().and_then(|state| {
            self.log
                .write_snapshot(DOMAIN, &state, &self.key)
                .map_err(|error| error.to_string())
        });
        match written {
            Ok(_) => {
                self.since_snapshot = 0;
                self.snapshot_failure = None;
            }
            Err(reason) => self.snapshot_failure = Some(reason),
        }
    }

    /// Resolve an append whose outcome is not known by reading the leaf
    /// store again; until then nothing is answered or appended.
    pub fn settle(&mut self) -> Result<(), ServerError> {
        if self.uncertain {
            let (log, held, start) = opened(&self.reopen, &self.key)?;
            self.log = log;
            self.held = held;
            self.start = start.clone();
            self.uncertain = false;
            self.after_start(&start);
        }
        Ok(())
    }

    /// Keep `policy` as the version after `expected`, the version the caller
    /// read (0 for none); refused `PolicyVersionConflict` when another change
    /// came between, and by name when its shape is wrong.
    pub fn set(&mut self, policy: Policy, expected: u64) -> Result<Policy, ServerError> {
        self.settle()?;
        let held = self
            .held
            .latest(&policy.agent)
            .map_or(0, |held| held.version);
        if held != expected {
            return Err(ServerError::PolicyVersionConflict { held, expected });
        }
        let policy = Policy {
            version: expected + 1,
            ..policy
        }
        .checked()
        .map_err(|refused| ServerError::PolicyRefused {
            refusal: refused.refusal,
            words: refused.words,
        })?;
        self.append(&policy)?;
        Ok(policy)
    }

    fn append(&mut self, policy: &Policy) -> Result<(), ServerError> {
        let bytes = serde_json::to_vec(policy).map_err(unavailable)?;
        let index = self.log.len();
        let Err(failure) = self.log.append(&bytes) else {
            if let Err(reason) = self.held.hold(policy.clone()) {
                self.uncertain = true;
                return Err(unavailable(reason));
            }
            self.since_snapshot += 1;
            if self.since_snapshot >= SNAPSHOT_EVERY.get() {
                self.write_snapshot();
            }
            return Ok(());
        };
        self.uncertain = true;
        self.settle()?;
        match self.log.leaf_bytes(index).map_err(unavailable)? {
            Some(held) if held == bytes => Ok(()),
            Some(_) => Err(unavailable(format!(
                "leaf {index} was written by another writer: {failure}"
            ))),
            None => Err(unavailable(failure)),
        }
    }
}

fn opened<S: LeafStore>(
    reopen: &Reopen<S>,
    key: &Ed25519Identity,
) -> Result<Opened<S>, ServerError> {
    let store = reopen().map_err(unavailable)?;
    let started = start(store, DOMAIN, &key.public_key_bytes()).map_err(unavailable)?;
    let mut held = match started.state.as_deref().map(Held::decode) {
        None => Held::default(),
        Some(Ok(held)) => held,
        Some(Err(reason)) => return rebuilt(reopen, reason),
    };
    held.fold(&started.tail).map_err(unavailable)?;
    Ok((started.log, held, started.start))
}

fn rebuilt<S: LeafStore>(reopen: &Reopen<S>, reason: String) -> Result<Opened<S>, ServerError> {
    let (log, tail) = FrontierLog::open(reopen().map_err(unavailable)?).map_err(unavailable)?;
    let mut held = Held::default();
    held.fold(&tail).map_err(unavailable)?;
    let replayed = log.len();
    let start = Start::Rebuilt {
        refusal: SnapshotRefusal::StateUnreadable { reason },
        replayed,
    };
    Ok((log, held, start))
}
