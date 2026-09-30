//! The emergency stops as they are kept: a leaf store of their own, one leaf
//! for each stop, written once every part of the stop is done and before it
//! is answered. A leaf is stored whole or not at all.
//!
//! An append that fails leaves its outcome unknown. It is settled by opening
//! the leaf store again and reading what it holds, before anything else is
//! read or written, so memory never runs ahead of or behind the leaves.
//!
//! A stop is named by the operation id it was sent under. The same stop sent
//! again in the same words answers what was kept and writes nothing; the
//! same operation in other words is refused.
//!
//! What the leaves fold to is sealed in the log's signed snapshot every
//! [`SNAPSHOT_EVERY`] leaves and at once after a rebuild, so a start reads the
//! snapshot and only the leaves after it. A snapshot refused, or a state that
//! does not read back, sends the start to every leaf, by name, never silently.

use std::path::Path;
use std::sync::Arc;

use lys_core::Ed25519Identity;
use lys_identity::SNAPSHOT_EVERY;
use lys_log_store::{
    FileLeafStore, FrontierLog, LeafStore, SnapshotRefusal, Start, StoreResult, open_with_snapshot,
};

use crate::config::Config;
use crate::error::ServerError;
use crate::routes::Say;
use crate::stops_state::{DOMAIN, Held, Stop};

/// How the leaf store is opened again after an append whose outcome is not known.
pub type Reopen<S> = Box<dyn Fn() -> StoreResult<S> + Send>;

/// The origin the stops' leaf store is created with.
pub const ORIGIN: &str = "lys/identity/stops";

/// The stops, read from their leaf store and appended to it.
pub struct StopStore<S: LeafStore = FileLeafStore> {
    reopen: Reopen<S>,
    key: Arc<Ed25519Identity>,
    log: FrontierLog<S>,
    held: Held,
    start: Start,
    since_snapshot: u64,
    snapshot_failure: Option<String>,
    uncertain: bool,
}

/// A log opened and folded: the log, what it folds to, and how it started.
type Opened<S> = (FrontierLog<S>, Held, Start);

fn unavailable(what: impl std::fmt::Display) -> ServerError {
    ServerError::StopsUnavailable {
        reason: what.to_string(),
    }
}

impl StopStore<FileLeafStore> {
    /// The stops in the directory `config` names, their snapshots signed by
    /// `key`, saying through `say` how the log started; none when it names no
    /// directory.
    pub fn configured(
        config: &Config,
        key: Arc<Ed25519Identity>,
        say: &Say,
    ) -> Result<Option<Self>, ServerError> {
        let Some(dir) = config.stops_dir.as_deref() else {
            return Ok(None);
        };
        let store = Self::open(dir, key)?;
        say(&format!(
            "stops log {}, holding {} stops",
            store.start(),
            store.held.stops.len()
        ));
        Ok(Some(store))
    }

    /// The stops kept in the directory `dir`, which is created when it does
    /// not exist, their snapshots signed by `key`.
    pub fn open(dir: &Path, key: Arc<Ed25519Identity>) -> Result<Self, ServerError> {
        if !dir.exists() {
            FileLeafStore::create(dir, ORIGIN).map_err(unavailable)?;
        }
        let dir = dir.to_owned();
        Self::over(Box::new(move || FileLeafStore::open(&dir)), key)
    }
}

impl<S: LeafStore> StopStore<S> {
    /// The stops kept in the leaf store `reopen` opens, their snapshots
    /// signed by `key`.
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

    /// How the log was started: from its snapshot, or from every leaf and
    /// the refusal that sent it there.
    pub fn start(&self) -> &Start {
        &self.start
    }

    /// Why the last snapshot could not be written, while no later one was.
    pub fn snapshot_failure(&self) -> Option<&str> {
        self.snapshot_failure.as_deref()
    }

    /// Owe a snapshot for the leaves the start read, and write one at once
    /// when the start rebuilt.
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

    /// Resolve an append whose outcome is not known, by opening the leaf
    /// store again and reading what it holds. Until that succeeds nothing is
    /// answered from memory and nothing is appended.
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

    /// Append one stop as one leaf. A failed append is settled by reading
    /// back: the stop is kept only if the leaf store holds exactly it.
    fn append(&mut self, stop: Stop) -> Result<(), ServerError> {
        let bytes = serde_json::to_vec(&stop).map_err(unavailable)?;
        let index = self.log.len();
        let Err(failure) = self.log.append(&bytes) else {
            if let Err(reason) = self.held.hold(stop) {
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

    /// The stop kept under `operation`.
    pub fn recorded(&self, operation: &str) -> Option<&Stop> {
        self.held.operation(operation)
    }

    /// Every stop kept on `agent`, in the order kept.
    pub fn of_agent(&self, agent: &str) -> Vec<Stop> {
        self.held.of_agent(agent).cloned().collect()
    }

    /// Keep `stop` as asked, before its first part. Answers the stop as kept
    /// when the same words were already done; nothing when they are asked
    /// and not yet done, or newly asked; the same operation in other words
    /// is refused.
    pub fn ask(&mut self, stop: Stop) -> Result<Option<Stop>, ServerError> {
        self.settle()?;
        if let Some(kept) = self.held.operation(&stop.operation) {
            if !kept.same_words(&stop) {
                return Err(ServerError::StopReused {
                    operation: stop.operation,
                });
            }
            return Ok(kept.done.then(|| kept.clone()));
        }
        self.append(Stop {
            done: false,
            ..stop
        })?;
        Ok(None)
    }

    /// Keep `stop` whole, once every part of it is done. Sent again in the
    /// same words it is kept once and answers what was kept; the same
    /// operation in other words is refused.
    pub fn keep(&mut self, stop: Stop) -> Result<Stop, ServerError> {
        self.settle()?;
        if let Some(kept) = self.held.operation(&stop.operation) {
            if !kept.same_words(&stop) {
                return Err(ServerError::StopReused {
                    operation: stop.operation,
                });
            }
            if kept.done {
                return Ok(kept.clone());
            }
        }
        let stop = Stop { done: true, ..stop };
        self.append(stop.clone())?;
        Ok(stop)
    }
}

/// Open the log from its snapshot, or from every leaf when the snapshot or
/// its state is refused, and fold what the start hands back.
fn opened<S: LeafStore>(
    reopen: &Reopen<S>,
    key: &Ed25519Identity,
) -> Result<Opened<S>, ServerError> {
    let store = reopen().map_err(unavailable)?;
    let started =
        open_with_snapshot(store, DOMAIN, &key.public_key_bytes()).map_err(unavailable)?;
    let mut held = match started.state.as_deref().map(Held::decode) {
        None => Held::default(),
        Some(Ok(held)) => held,
        Some(Err(reason)) => return rebuilt(reopen, reason),
    };
    held.fold(&started.tail).map_err(unavailable)?;
    Ok((started.log, held, started.start))
}

/// Open the log from every leaf, because the snapshot's state was refused
/// for `reason`.
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
