//! The seats as they are kept (AGENTS-002 R1): a leaf store of their own
//! beside the directory log, one leaf for each line, written before the line
//! is answered. A leaf is stored whole or not at all.
//!
//! The store is always open, as the master off switch and the runner acts
//! are: a seat is this install's own record, and an install that keeps
//! machines and runs agents keeps its seats too.
//!
//! An append that fails leaves its outcome unknown. It is settled by opening
//! the leaf store again and reading what it holds, before anything else is
//! read or written, so memory never runs ahead of or behind the leaves.
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

use crate::error::ServerError;
use crate::error_seat::SeatError;
use crate::seats_state::{DOMAIN, Held, Line, Seat};

/// How the leaf store is opened again after an append whose outcome is not known.
pub type Reopen<S> = Box<dyn Fn() -> StoreResult<S> + Send>;

/// The origin the seats' leaf store is created with.
pub const ORIGIN: &str = "lys/identity/seats";

/// The seats, read from their leaf store and appended to it.
pub struct SeatStore<S: LeafStore = FileLeafStore> {
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
    SeatError::Unavailable {
        reason: what.to_string(),
    }
    .into()
}

impl SeatStore<FileLeafStore> {
    /// The seats kept in the directory `dir`, which is created when it does
    /// not exist, their snapshots signed by `key`.
    pub fn open(dir: &Path, key: Arc<Ed25519Identity>) -> Result<Self, ServerError> {
        if !dir.exists() {
            FileLeafStore::create(dir, ORIGIN).map_err(unavailable)?;
        }
        let dir = dir.to_owned();
        Self::over(Box::new(move || FileLeafStore::open(&dir)), key)
    }
}

impl<S: LeafStore> SeatStore<S> {
    /// The seats kept in the leaf store `reopen` opens, their snapshots
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

    /// How many seats are kept.
    pub fn len(&self) -> usize {
        self.held.seats.len()
    }

    /// Whether no seat is kept.
    pub fn is_empty(&self) -> bool {
        self.held.seats.is_empty()
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

    /// Every seat, in the order added.
    pub fn seats(&self) -> &[Seat] {
        &self.held.seats
    }

    /// The seat named `name`.
    pub fn seat(&self, name: &str) -> Option<&Seat> {
        self.held.seat(name)
    }

    /// The seat named `name`, refused `seat_unknown` when none is.
    pub fn named(&self, name: &str) -> Result<Seat, ServerError> {
        self.held.seat(name).cloned().ok_or_else(|| {
            SeatError::Unknown {
                name: name.to_owned(),
            }
            .into()
        })
    }

    /// The line kept under `operation`.
    pub fn recorded(&self, operation: &str) -> Option<&Line> {
        self.held.operation(operation)
    }

    /// Keep `line` once. The same line sent again under its operation
    /// answers the line as first kept and writes nothing; the same
    /// operation in other words is refused `seat_operation_reused`, and a
    /// second seat by a name already held `seat_name_taken`.
    pub fn keep(&mut self, line: Line) -> Result<Line, ServerError> {
        self.settle()?;
        if let Some(kept) = self.held.operation(line.operation()) {
            if kept.same_words(&line) {
                return Ok(kept.clone());
            }
            return Err(SeatError::OperationReused {
                operation: line.operation().to_owned(),
            }
            .into());
        }
        if matches!(line, Line::Added(_)) && self.held.seat(line.name()).is_some() {
            return Err(SeatError::NameTaken {
                name: line.name().to_owned(),
            }
            .into());
        }
        if !matches!(line, Line::Added(_)) && self.held.seat(line.name()).is_none() {
            return Err(SeatError::Unknown {
                name: line.name().to_owned(),
            }
            .into());
        }
        self.append(line.clone())?;
        Ok(line)
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

#[cfg(test)]
mod tests {
    use std::error::Error;
    use std::sync::Arc;

    use lys_core::Ed25519Identity;

    use super::SeatStore;
    use crate::error::ServerError;
    use crate::error_agents::AgentsError;
    use crate::error_seat::SeatError;
    use crate::seats_state::{Added, Line, Started};

    fn added(operation: &str, name: &str) -> Line {
        Line::Added(Added {
            operation: operation.to_owned(),
            name: name.to_owned(),
            agent: "agent".to_owned(),
            harness: "claude".to_owned(),
            profile_version: 1,
            machine: "op-1".to_owned(),
            working_folder: None,
            account: None,
            by: "person".to_owned(),
            at: 1,
        })
    }

    #[test]
    fn a_seat_survives_a_restart_and_an_operation_answers_once() -> Result<(), Box<dyn Error>> {
        let dir = tempfile::tempdir()?;
        let key = Arc::new(Ed25519Identity::load_or_generate(&dir.path().join("key"))?);
        let path = dir.path().join("seats");
        let mut store = SeatStore::open(&path, Arc::clone(&key))?;
        store.keep(added("op-a", "waffles"))?;
        assert_eq!(
            store.keep(added("op-a", "waffles"))?,
            added("op-a", "waffles")
        );
        assert!(matches!(
            store.keep(added("op-a", "other")),
            Err(ServerError::Agents(AgentsError::Seat(SeatError::OperationReused { .. })))
        ));
        assert!(matches!(
            store.keep(added("op-b", "waffles")),
            Err(ServerError::Agents(AgentsError::Seat(SeatError::NameTaken { .. })))
        ));
        store.keep(Line::Started(Started {
            operation: "op-c".to_owned(),
            name: "waffles".to_owned(),
            session: "op-c".to_owned(),
            harness_session: None,
            by: "person".to_owned(),
            at: 2,
        }))?;
        drop(store);
        let reopened = SeatStore::open(&path, key)?;
        let seat = reopened.named("waffles")?;
        assert!(seat.running);
        assert_eq!(seat.revision, 2);
        assert_eq!(reopened.len(), 1);
        assert!(matches!(
            reopened.named("absent"),
            Err(ServerError::Agents(AgentsError::Seat(SeatError::Unknown { .. })))
        ));
        Ok(())
    }
}
