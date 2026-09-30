#![cfg(test)]
//! The teams as they are kept: a leaf store of their own, one leaf for each
//! team created, each member added or removed and each team retired, written
//! before the act is answered. A leaf is stored whole or not at all.
//!
//! An append that fails leaves its outcome unknown. It is settled by opening
//! the leaf store again and reading what it holds, before anything else is
//! read or written, so memory never runs ahead of or behind the leaves.
//!
//! A team is named by the operation id it was created with, and every later
//! change is kept under an operation id of its own. The same act sent again
//! in the same words answers the team as it stands and writes nothing; the
//! same operation in other words is refused.
//!
//! What the leaves fold to is sealed in the log's signed snapshot every
//! [`SNAPSHOT_EVERY`] leaves and at once after a rebuild, so a start reads the
//! snapshot and only the leaves after it. A snapshot refused, or a state that
//! does not read back, sends the start to every leaf, by name, never silently.

/// The frozen writer revision used by the compatibility proof.
pub const SOURCE_COMMIT: &str = "088351303e6b6d04ac3200ddb5bf86e243d91ddb";

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
use crate::teams_state::{DOMAIN, Held, Line, Refused, Team};

/// How the leaf store is opened again after an append whose outcome is not known.
pub type Reopen<S> = Box<dyn Fn() -> StoreResult<S> + Send>;

/// The origin the teams' leaf store is created with.
pub const ORIGIN: &str = "lys/identity/teams";

/// The teams, read from their leaf store and appended to it.
pub struct TeamStore<S: LeafStore = FileLeafStore> {
    reopen: Reopen<S>,
    key: Arc<Ed25519Identity>,
    log: FrontierLog<S>,
    held: Held,
    start: Start,
    since_snapshot: u64,
    snapshot_failure: Option<String>,
    uncertain: bool,
    snapshots_enabled: bool,
    pending: Vec<Line>,
    overlay: Option<Held>,
}

/// A log opened and folded: the log, what it folds to, and how it started.
type Opened<S> = (FrontierLog<S>, Held, Start);

fn unavailable(what: impl std::fmt::Display) -> ServerError {
    ServerError::TeamsUnavailable {
        reason: what.to_string(),
    }
}

impl TeamStore<FileLeafStore> {
    /// The teams in the directory `config` names, their snapshots signed by
    /// `key`, saying through `say` how the log started; none when it names no
    /// directory.
    pub fn configured(
        config: &Config,
        key: Arc<Ed25519Identity>,
        say: &Say,
    ) -> Result<Option<Self>, ServerError> {
        let Some(dir) = config.teams_dir.as_deref() else {
            return Ok(None);
        };
        if !dir.exists() {
            FileLeafStore::create(dir, ORIGIN).map_err(unavailable)?;
        }
        let dir = dir.to_owned();
        let store = Self::opening(Box::new(move || FileLeafStore::open(&dir)), key, false)?;
        say(&format!(
            "teams log {}, holding {} teams",
            store.start(),
            store.teams().len()
        ));
        Ok(Some(store))
    }

    /// The teams kept in the directory `dir`, which is created when it does
    /// not exist, their snapshots signed by `key`.
    pub fn open(dir: &Path, key: Arc<Ed25519Identity>) -> Result<Self, ServerError> {
        if !dir.exists() {
            FileLeafStore::create(dir, ORIGIN).map_err(unavailable)?;
        }
        let dir = dir.to_owned();
        Self::over(Box::new(move || FileLeafStore::open(&dir)), key)
    }
}

impl<S: LeafStore> TeamStore<S> {
    /// The teams kept in the leaf store `reopen` opens, their snapshots
    /// signed by `key`.
    pub fn over(reopen: Reopen<S>, key: Arc<Ed25519Identity>) -> Result<Self, ServerError> {
        Self::opening(reopen, key, true)
    }

    fn opening(
        reopen: Reopen<S>,
        key: Arc<Ed25519Identity>,
        snapshots_enabled: bool,
    ) -> Result<Self, ServerError> {
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
            snapshots_enabled,
            pending: Vec::new(),
            overlay: None,
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
        if !self.snapshots_enabled {
            return;
        }
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
            self.refresh_overlay()?;
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
            self.refresh_overlay()?;
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

    /// Every team, in the order created.
    pub fn teams(&self) -> &[Team] {
        &self.overlay.as_ref().unwrap_or(&self.held).teams
    }

    /// The team named `id`.
    pub fn team(&self, id: &str) -> Option<&Team> {
        self.overlay.as_ref().unwrap_or(&self.held).team(id)
    }

    /// The line first kept under `operation`.
    pub fn recorded(&self, operation: &str) -> Option<Line> {
        self.held.operation(operation)
    }

    /// Keep `line` and answer its team as it then stands. Sent again in the
    /// same words it is kept once; the same operation in other words is
    /// refused, and so is a line the team as it stands does not take.
    pub fn keep(&mut self, line: Line) -> Result<Team, ServerError> {
        self.settle()?;
        if matches!(line, Line::Held(_) | Line::Checked(_)) {
            return Err(unavailable("migration lines use the migration writer"));
        }
        if let Some(kept) = self.held.operation(line.operation()) {
            if !kept.same_act(&line) {
                return Err(ServerError::TeamReused {
                    operation: line.operation().to_owned(),
                });
            }
            return self.standing(line.team());
        }
        self.held.allows(&line).map_err(|refused| match refused {
            Refused::Unknown => ServerError::TeamUnknown,
            Refused::Retired => ServerError::TeamRetired {
                team: line.team().to_owned(),
            },
            Refused::Held => ServerError::TeamMemberHeld,
            Refused::Absent => ServerError::TeamMemberAbsent,
            Refused::NotHeld => unavailable(format!("team `{}` member is not held", line.team())),
            Refused::Checked => unavailable("legacy memberships were already checked"),
        })?;
        let team = line.team().to_owned();
        self.append(line)?;
        self.standing(&team)
    }

    /// Whether the durable log already completed its membership check.
    pub fn migration_checked(&self) -> bool {
        self.held.checked.is_some()
    }

    /// Enforce a validated migration in memory without changing any leaf or snapshot.
    pub fn stage_migration(&mut self, lines: Vec<Line>) -> Result<(), ServerError> {
        self.pending = lines;
        self.refresh_overlay()
    }

    fn pending_line(&self, line: &Line) -> bool {
        if self.held.operation(line.operation()).is_some() {
            return false;
        }
        match line {
            Line::Held(hold) => self.held.team(&hold.team).is_some_and(|team| {
                team.retired.is_none()
                    && team.members.contains(&hold.member)
                    && !team.held.iter().any(|held| held.member == hold.member)
            }),
            Line::Checked(_) => self.held.checked.is_none(),
            _ => false,
        }
    }

    fn refresh_overlay(&mut self) -> Result<(), ServerError> {
        let mut held = self.held.clone();
        for line in &self.pending {
            if self.pending_line(line) {
                held.hold(line.clone()).map_err(unavailable)?;
            }
        }
        self.overlay = if self.pending.is_empty() {
            None
        } else {
            Some(held)
        };
        Ok(())
    }

    /// Complete staged migration after the reversible upgrade window closes.
    /// Each leaf is idempotent; a crash prefix is completed by the next start.
    pub fn finish_migration(&mut self) -> Result<(), ServerError> {
        self.settle()?;
        if self.pending.is_empty() && self.snapshots_enabled {
            return Ok(());
        }
        for line in self.pending.clone() {
            if self.pending_line(&line) {
                self.append(line)?;
            }
        }
        self.pending.clear();
        self.overlay = None;
        self.snapshots_enabled = true;
        self.write_snapshot();
        Ok(())
    }

    fn standing(&self, id: &str) -> Result<Team, ServerError> {
        self.team(id).cloned().ok_or(ServerError::TeamUnknown)
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
