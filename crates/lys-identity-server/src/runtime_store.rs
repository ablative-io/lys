//! The runtime reports as they are kept: a leaf store of their own, one leaf
//! for each report, written before the report is answered. A leaf is stored
//! whole or not at all.
//!
//! An append that fails leaves its outcome unknown. It is settled by opening
//! the leaf store again and reading what it holds, before anything else is
//! read or written, so memory never runs ahead of or behind the leaves.
//!
//! A report is named by the operation id it was sent with, so sending it
//! again in the same words answers the session as it stands, and the same
//! operation in other words is refused. An agent's session begins with
//! `starting`; a found session begins with `running`, since its runtime first
//! sees it already running. A stopped session takes no further report.
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
    FileLeafStore, FrontierLog, LeafStore, SnapshotRefusal, Start, StoreResult, start,
};

use lys_identity::signer::load_service_key;

use crate::config::Config;
use crate::error::ServerError;
use crate::routes::Say;
use crate::runtime_state::{DOMAIN, Held, Report, Reported, Tracked};

/// How the leaf store is opened again after an append whose outcome is not known.
pub type Reopen<S> = Box<dyn Fn() -> StoreResult<S> + Send>;

/// The origin the reports' leaf store is created with.
pub const ORIGIN: &str = "lys/identity/runtime-reports";

/// The runtime reports, read from their leaf store and appended to it.
pub struct RuntimeStore<S: LeafStore = FileLeafStore> {
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
    ServerError::RuntimeUnavailable {
        reason: what.to_string(),
    }
}

impl RuntimeStore<FileLeafStore> {
    /// The reports in the directory `config` names, their snapshots signed
    /// by the service's event key, saying through `say` how the log started;
    /// none when it names no directory.
    pub fn configured(config: &Config, say: &Say) -> Result<Option<Self>, ServerError> {
        let Some(dir) = config.runtime_dir.as_deref() else {
            return Ok(None);
        };
        let store = Self::open(dir, Arc::new(load_service_key(&config.event_key_file)?))?;
        say(&format!(
            "runtime reports log {}, holding {} sessions",
            store.start(),
            store.sessions().len()
        ));
        Ok(Some(store))
    }

    /// The reports kept in the directory `dir`, which is created when it
    /// does not exist, their snapshots signed by `key`.
    pub fn open(dir: &Path, key: Arc<Ed25519Identity>) -> Result<Self, ServerError> {
        if !dir.exists() {
            FileLeafStore::create(dir, ORIGIN).map_err(unavailable)?;
        }
        let dir = dir.to_owned();
        Self::over(Box::new(move || FileLeafStore::open(&dir)), key)
    }
}

impl<S: LeafStore> RuntimeStore<S> {
    /// The reports kept in the leaf store `reopen` opens, their snapshots
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

    /// Append one report as one leaf. A failed append is settled by reading
    /// back: the report is kept only if the leaf store holds exactly it.
    fn append(&mut self, report: Report) -> Result<(), ServerError> {
        let bytes = serde_json::to_vec(&report).map_err(unavailable)?;
        let index = self.log.len();
        let Err(failure) = self.log.append(&bytes) else {
            if let Err(reason) = self.held.hold(report) {
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

    /// Every session, in the order first reported.
    pub fn sessions(&self) -> &[Tracked] {
        &self.held.sessions
    }

    /// The session named `session`.
    pub fn session(&self, session: &str) -> Option<&Tracked> {
        self.held.session(session)
    }

    /// Keep `report` and answer the session as it then stands. Sent again in
    /// the same words it is kept once; the same operation in other words is
    /// refused, and so is a report the session as it stands does not take.
    pub fn report(&mut self, report: Report) -> Result<Tracked, ServerError> {
        self.settle()?;
        if let Some(kept) = self.held.operation(&report.operation) {
            if !same_words(kept, &report) {
                return Err(ServerError::RuntimeReportReused {
                    operation: report.operation,
                });
            }
            return self.standing(&report.session);
        }
        match self.held.session(&report.session) {
            None => {
                let begins = match report.agent {
                    Some(_) => report.state == Reported::Starting,
                    None => report.state == Reported::Running,
                };
                if !begins {
                    return Err(ServerError::RuntimeSessionUnknown);
                }
            }
            Some(tracked) => {
                if tracked.agent != report.agent {
                    return Err(ServerError::RuntimeSessionUnknown);
                }
                if tracked.stopped() {
                    return Err(ServerError::RuntimeSessionStopped {
                        session: report.session,
                    });
                }
                if report.state == Reported::Starting {
                    return Err(ServerError::RuntimeSessionStarted {
                        session: report.session,
                    });
                }
                if tracked.machine != report.machine {
                    return Err(ServerError::RequestMalformed {
                        reason: format!(
                            "session `{}` runs on machine `{}`, not `{}`",
                            report.session, tracked.machine, report.machine
                        ),
                    });
                }
            }
        }
        let session = report.session.clone();
        self.append(report)?;
        self.standing(&session)
    }

    fn standing(&self, session: &str) -> Result<Tracked, ServerError> {
        self.held
            .session(session)
            .cloned()
            .ok_or(ServerError::RuntimeSessionUnknown)
    }
}

/// Open the log from its snapshot, or from every leaf when the snapshot or
/// its state is refused, and fold what the start hands back.
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

/// Whether two reports are the same report, whenever each was received.
fn same_words(kept: &Report, report: &Report) -> bool {
    let timeless = Report {
        at: kept.at,
        ..report.clone()
    };
    *kept == timeless
}
