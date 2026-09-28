//! The service accounts as they are kept: a leaf store of their own, one
//! leaf for each account created and each account retired, written before
//! the act is answered. A leaf is stored whole or not at all.
//!
//! An append that fails leaves its outcome unknown. It is settled by opening
//! the leaf store again and reading what it holds, before anything else is
//! read or written, so memory never runs ahead of or behind the leaves.
//!
//! A service account is named by the operation id it was created with, and a
//! retirement is kept under an operation id of its own. The same act sent
//! again in the same words answers the account as it stands and writes
//! nothing; the same operation in other words is refused.
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

use crate::config::Config;
use crate::error::ServerError;
use crate::routes::Say;
use crate::service_accounts_state::{Account, Created, DOMAIN, Held, Line, Retired};

/// How the leaf store is opened again after an append whose outcome is not known.
pub type Reopen<S> = Box<dyn Fn() -> StoreResult<S> + Send>;

/// The origin the service accounts' leaf store is created with.
pub const ORIGIN: &str = "lys/identity/service-accounts";

/// The service accounts, read from their leaf store and appended to it.
pub struct ServiceAccountStore<S: LeafStore = FileLeafStore> {
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
    ServerError::ServiceAccountsUnavailable {
        reason: what.to_string(),
    }
}

impl ServiceAccountStore<FileLeafStore> {
    /// The service accounts in the directory `config` names, their snapshots
    /// signed by `key`, saying through `say` how the log started; none when
    /// it names no directory.
    pub fn configured(
        config: &Config,
        key: Arc<Ed25519Identity>,
        say: &Say,
    ) -> Result<Option<Self>, ServerError> {
        let Some(dir) = config.service_accounts_dir.as_deref() else {
            return Ok(None);
        };
        let store = Self::open(dir, key)?;
        say(&format!(
            "service accounts log {}, holding {} accounts",
            store.start(),
            store.accounts().len()
        ));
        Ok(Some(store))
    }

    /// The service accounts kept in the directory `dir`, which is created
    /// when it does not exist, their snapshots signed by `key`.
    pub fn open(dir: &Path, key: Arc<Ed25519Identity>) -> Result<Self, ServerError> {
        if !dir.exists() {
            FileLeafStore::create(dir, ORIGIN).map_err(unavailable)?;
        }
        let dir = dir.to_owned();
        Self::over(Box::new(move || FileLeafStore::open(&dir)), key)
    }
}

impl<S: LeafStore> ServiceAccountStore<S> {
    /// The service accounts kept in the leaf store `reopen` opens, their
    /// snapshots signed by `key`.
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

    /// Append one line as one leaf. A failed append is settled by reading
    /// back: the line is kept only if the leaf store holds exactly it.
    fn append(&mut self, line: Line) -> Result<(), ServerError> {
        let bytes = serde_json::to_vec(&line).map_err(unavailable)?;
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
        self.settle()?;
        match self.log.leaf_bytes(index).map_err(unavailable)? {
            Some(held) if held == bytes => Ok(()),
            Some(_) => Err(unavailable(format!(
                "leaf {index} was written by another writer: {failure}"
            ))),
            None => Err(unavailable(failure)),
        }
    }

    /// Every service account, in the order created.
    pub fn accounts(&self) -> &[Account] {
        &self.held.accounts
    }

    /// The service account named `id`.
    pub fn account(&self, id: &str) -> Option<&Account> {
        self.held.account(id)
    }

    /// Whether `operation` already names a creation or a retirement.
    pub fn names(&self, operation: &str) -> bool {
        self.held.operation(operation).is_some()
    }

    /// Keep `created` and answer the account as it then stands. Created
    /// again in the same words it is kept once; the same operation in other
    /// words is refused.
    pub fn create(&mut self, created: Created) -> Result<Account, ServerError> {
        self.settle()?;
        match self.held.operation(&created.id) {
            Some(Line::Created(kept)) if same_creation(&kept, &created) => {
                self.standing(&created.id)
            }
            Some(_) => Err(ServerError::ServiceAccountReused {
                operation: created.id,
            }),
            None => {
                let id = created.id.clone();
                self.append(Line::Created(created))?;
                self.standing(&id)
            }
        }
    }

    /// Keep `retired` and answer the account as it then stands. Retired
    /// again in the same words it is kept once; the same operation in other
    /// words is refused, and so is an account already retired under another
    /// operation.
    pub fn retire(&mut self, retired: Retired) -> Result<Account, ServerError> {
        self.settle()?;
        match self.held.operation(&retired.operation) {
            Some(Line::Retired(kept)) if same_retirement(&kept, &retired) => {
                return self.standing(&retired.account);
            }
            Some(_) => {
                return Err(ServerError::ServiceAccountReused {
                    operation: retired.operation,
                });
            }
            None => {}
        }
        match self.held.account(&retired.account) {
            None => Err(ServerError::ServiceAccountUnknown),
            Some(account) if account.is_retired() => Err(ServerError::ServiceAccountRetired {
                account: retired.account,
            }),
            Some(_) => {
                let id = retired.account.clone();
                self.append(Line::Retired(retired))?;
                self.standing(&id)
            }
        }
    }

    fn standing(&self, id: &str) -> Result<Account, ServerError> {
        self.held
            .account(id)
            .cloned()
            .ok_or(ServerError::ServiceAccountUnknown)
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

/// Whether two creations are the same act, whenever each was received.
fn same_creation(kept: &Created, created: &Created) -> bool {
    let timeless = Created {
        at: kept.at,
        ..created.clone()
    };
    *kept == timeless
}

/// Whether two retirements are the same act, whenever each was received.
fn same_retirement(kept: &Retired, retired: &Retired) -> bool {
    let timeless = Retired {
        at: kept.at,
        ..retired.clone()
    };
    *kept == timeless
}
