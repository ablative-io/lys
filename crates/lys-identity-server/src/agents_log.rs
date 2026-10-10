//! One fold-and-snapshot leaf log for the agents cluster's records: the
//! words (AGENTS-001 R1), the variables (R2) and the schedules (R4) each
//! keep their own leaf store through this engine, so one engine opens from
//! the log's signed snapshot and replays only the leaves after it, appends
//! one line as one leaf, and settles an append whose outcome is not known
//! by reading the leaf store back before anything else is answered.
//!
//! The engine is the goals store's (goals_store.rs) with the fold made a
//! trait, so three record kinds do not carry three copies of it.

use std::path::Path;
use std::sync::{Arc, Mutex};

use lys_core::Ed25519Identity;
use lys_identity::SNAPSHOT_EVERY;
use lys_log_store::{
    FileLeafStore, FrontierLog, LeafStore, SnapshotRefusal, Start, StoreResult, open_with_snapshot,
};
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::error::ServerError;
use crate::session::now;

/// How the leaf store is opened again after an append whose outcome is not known.
pub type Reopen<S> = Box<dyn Fn() -> StoreResult<S> + Send>;

/// What a record log folds to, line by line.
pub trait Folded: Default + Serialize + DeserializeOwned + Send {
    /// One leaf of the log.
    type Line: Serialize + DeserializeOwned;
    /// The snapshot domain the folded state is sealed under.
    const DOMAIN: &'static str;
    /// The format name sealed beside the state, refused when it differs.
    const FORMAT: &'static str;
    /// The record kind, for refusals.
    const KIND: &'static str;
    /// Fold one line; a line the fold refuses names why.
    fn hold(&mut self, line: Self::Line) -> Result<(), String>;

    /// Fold every leaf of `tail`, in order.
    fn fold(&mut self, tail: &lys_log_store::Tail) -> Result<(), String> {
        for (index, bytes) in (tail.from..).zip(&tail.leaves) {
            let line = serde_json::from_slice(bytes)
                .map_err(|error| format!("leaf {index} is not a {} line: {error}", Self::KIND))?;
            self.hold(line)
                .map_err(|reason| format!("leaf {index}: {reason}"))?;
        }
        Ok(())
    }

    /// The state a snapshot seals.
    fn encode(&self) -> Result<Vec<u8>, String> {
        #[derive(Serialize)]
        struct Sealing<'a, T> {
            format: &'static str,
            held: &'a T,
        }
        serde_json::to_vec(&Sealing {
            format: Self::FORMAT,
            held: self,
        })
        .map_err(|error| format!("{} state: {error}", Self::KIND))
    }

    /// The state a snapshot sealed, refused by reason unless it reads whole
    /// in this format.
    fn decode(bytes: &[u8]) -> Result<Self, String> {
        let value: serde_json::Value = serde_json::from_slice(bytes)
            .map_err(|error| format!("{} state: {error}", Self::KIND))?;
        let format = value
            .get("format")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| format!("{} state has no format", Self::KIND))?;
        if format != Self::FORMAT {
            return Err(format!(
                "{} state is in format {format}, not {}",
                Self::KIND,
                Self::FORMAT
            ));
        }
        let held = value
            .get("held")
            .cloned()
            .ok_or_else(|| format!("{} state has no held member", Self::KIND))?;
        serde_json::from_value(held).map_err(|error| format!("{} state: {error}", Self::KIND))
    }
}

/// A record log: its leaf store, what it folds to, and how it started.
pub struct RecordLog<F: Folded, S: LeafStore = FileLeafStore> {
    reopen: Reopen<S>,
    key: Arc<Ed25519Identity>,
    log: FrontierLog<S>,
    held: F,
    start: Start,
    opened_at: u64,
    since_snapshot: u64,
    snapshot_failure: Option<String>,
    uncertain: bool,
}

type Opened<F, S> = (FrontierLog<S>, F, Start);

impl<F: Folded> RecordLog<F, FileLeafStore> {
    /// The records kept in the directory `dir`, created with `origin` when
    /// it does not exist, their snapshots signed by `key`.
    pub fn open(
        dir: &Path,
        origin: &str,
        key: Arc<Ed25519Identity>,
        unavailable: &dyn Fn(String) -> ServerError,
    ) -> Result<Self, ServerError> {
        if !dir.exists() {
            FileLeafStore::create(dir, origin).map_err(|error| unavailable(error.to_string()))?;
        }
        let dir = dir.to_owned();
        Self::over(
            Box::new(move || FileLeafStore::open(&dir)),
            key,
            unavailable,
        )
    }
}

impl<F: Folded, S: LeafStore> RecordLog<F, S> {
    /// The records kept in the leaf store `reopen` opens, their snapshots
    /// signed by `key`; `unavailable` names a refusal of the store itself.
    pub fn over(
        reopen: Reopen<S>,
        key: Arc<Ed25519Identity>,
        unavailable: &dyn Fn(String) -> ServerError,
    ) -> Result<Self, ServerError> {
        let (log, held, start) = opened(&reopen, &key, unavailable)?;
        let mut record = Self {
            reopen,
            key,
            log,
            held,
            start: start.clone(),
            opened_at: now(),
            since_snapshot: 0,
            snapshot_failure: None,
            uncertain: false,
        };
        record.after_start(&start);
        Ok(record)
    }

    /// How the log was started.
    pub fn start(&self) -> &Start {
        &self.start
    }

    /// When this log was opened, in seconds since the Unix epoch.
    pub fn opened_at(&self) -> u64 {
        self.opened_at
    }

    /// Why the last snapshot could not be written, while no later one was.
    pub fn snapshot_failure(&self) -> Option<&str> {
        self.snapshot_failure.as_deref()
    }

    /// What the log folds to.
    pub fn held(&self) -> &F {
        &self.held
    }

    /// How many leaves the log holds.
    pub fn len(&self) -> u64 {
        self.log.len()
    }

    /// Whether the log holds no leaf.
    pub fn is_empty(&self) -> bool {
        self.log.len() == 0
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
                .write_snapshot(F::DOMAIN, &state, &self.key)
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

    /// Resolve an append whose outcome is not known by opening the leaf
    /// store again and reading what it holds. Until that succeeds nothing
    /// is answered from memory and nothing is appended.
    pub fn settle(&mut self, unavailable: &dyn Fn(String) -> ServerError) -> Result<(), ServerError> {
        if self.uncertain {
            let (log, held, start) = opened(&self.reopen, &self.key, unavailable)?;
            self.log = log;
            self.held = held;
            self.start = start.clone();
            self.uncertain = false;
            self.after_start(&start);
        }
        Ok(())
    }

    /// Append one line as one leaf and fold it. A failed append is settled
    /// by reading back: the line is kept only if the leaf store holds
    /// exactly it.
    pub fn append(
        &mut self,
        line: F::Line,
        unavailable: &dyn Fn(String) -> ServerError,
    ) -> Result<(), ServerError> {
        self.settle(unavailable)?;
        let bytes = serde_json::to_vec(&line).map_err(|error| unavailable(error.to_string()))?;
        let index = self.log.len();
        let Err(failure) = self.log.append(&bytes) else {
            if let Err(reason) = self.held.hold(line) {
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
        self.settle(unavailable)?;
        match self
            .log
            .leaf_bytes(index)
            .map_err(|error| unavailable(error.to_string()))?
        {
            Some(held) if held == bytes => Ok(()),
            Some(_) => Err(unavailable(format!(
                "leaf {index} was written by another writer: {failure}"
            ))),
            None => Err(unavailable(failure.to_string())),
        }
    }
}

fn opened<F: Folded, S: LeafStore>(
    reopen: &Reopen<S>,
    key: &Ed25519Identity,
    unavailable: &dyn Fn(String) -> ServerError,
) -> Result<Opened<F, S>, ServerError> {
    let store = reopen().map_err(|error| unavailable(error.to_string()))?;
    let started = open_with_snapshot(store, F::DOMAIN, &key.public_key_bytes())
        .map_err(|error| unavailable(error.to_string()))?;
    let mut held = match started.state.as_deref().map(F::decode) {
        None => F::default(),
        Some(Ok(held)) => held,
        Some(Err(reason)) => return rebuilt(reopen, reason, unavailable),
    };
    held.fold(&started.tail).map_err(unavailable)?;
    Ok((started.log, held, started.start))
}

/// Open the log from every leaf, because the snapshot's state was refused
/// for `reason`.
fn rebuilt<F: Folded, S: LeafStore>(
    reopen: &Reopen<S>,
    reason: String,
    unavailable: &dyn Fn(String) -> ServerError,
) -> Result<Opened<F, S>, ServerError> {
    let store = reopen().map_err(|error| unavailable(error.to_string()))?;
    let (log, tail) = FrontierLog::open(store).map_err(|error| unavailable(error.to_string()))?;
    let mut held = F::default();
    held.fold(&tail).map_err(unavailable)?;
    let replayed = log.len();
    let start = Start::Rebuilt {
        refusal: SnapshotRefusal::StateUnreadable { reason },
        replayed,
    };
    Ok((log, held, start))
}

/// A record log behind one lock, with the refusal its store answers.
pub struct Kept<F: Folded> {
    /// The log, one caller at a time.
    pub log: Mutex<RecordLog<F>>,
    unavailable: fn(String) -> ServerError,
}

impl<F: Folded> Kept<F> {
    /// `log`, refusing by `unavailable` when it cannot be read or written.
    pub fn new(log: RecordLog<F>, unavailable: fn(String) -> ServerError) -> Self {
        Self {
            log: Mutex::new(log),
            unavailable,
        }
    }

    /// Run `act` on the settled log.
    pub fn with<T>(
        &self,
        act: impl FnOnce(&mut RecordLog<F>, &dyn Fn(String) -> ServerError) -> Result<T, ServerError>,
    ) -> Result<T, ServerError> {
        let unavailable = self.unavailable;
        let mut log = self
            .log
            .lock()
            .map_err(|error| unavailable(format!("the {} lock is poisoned: {error}", F::KIND)))?;
        log.settle(&unavailable)?;
        act(&mut log, &unavailable)
    }
}
