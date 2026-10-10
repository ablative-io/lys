//! The seat import journal as it is kept (AGENTS-003 R5): a leaf store of
//! its own beside the directory log, folded and snapshotted through the
//! agents log engine, so an append whose outcome is not known is settled by
//! reading the leaf store back before anything else is answered.
//!
//! The journal is always open, as the seats are: an install that keeps
//! seats keeps their imports. Beside it, in memory only, is each seat's
//! latest dry run, which a confirmation names; a restart forgets it, and the
//! person runs a new dry run.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

use lys_core::Ed25519Identity;

use crate::agents_log::{Kept, RecordLog};
use crate::error::ServerError;
use crate::error_seat_import::SeatImportError;
use crate::seat_import_plan::{Manifest, Plan};
use crate::seat_import_state::{Journal, Line, Operation, Reserved};

/// The origin the import journal's leaf store is created with.
pub const ORIGIN: &str = "lys/identity/seat-imports";

fn unavailable(reason: String) -> ServerError {
    SeatImportError::Unavailable { reason }.into()
}

/// A dry run kept for its confirmation: the manifest it read and the plan.
#[derive(Debug, Clone, PartialEq)]
pub struct Preview {
    /// The manifest the dry run read.
    pub manifest: Manifest,
    /// The plan it showed.
    pub plan: Plan,
}

/// How a confirmation's reservation stands.
#[derive(Debug, Clone, PartialEq)]
pub enum Reservation {
    /// Newly reserved under this operation.
    Fresh(String),
    /// The same import, reserved before and not yet complete.
    Resumed(String),
    /// The same import, complete: its receipt is answered and nothing is written.
    Completed(Box<Operation>),
}

/// The seats' imports.
pub struct SeatImports {
    /// The journal, one caller at a time.
    pub journal: Kept<Journal>,
    previews: Mutex<BTreeMap<String, Preview>>,
}

impl SeatImports {
    /// The journal kept in the directory `dir`, created when it does not
    /// exist, its snapshots signed by `key`.
    pub fn open(dir: &Path, key: Arc<Ed25519Identity>) -> Result<Self, ServerError> {
        Ok(Self::over(RecordLog::open(dir, ORIGIN, key, &unavailable)?))
    }

    /// The journal `log`.
    pub fn over(log: RecordLog<Journal>) -> Self {
        Self {
            journal: Kept::new(log, unavailable),
            previews: Mutex::new(BTreeMap::new()),
        }
    }

    /// How the journal started and what it holds, said once at start.
    pub fn started(&self) -> Result<String, ServerError> {
        self.journal.with(|log, _| {
            Ok(format!(
                "seat-imports log {}, holding {} imports",
                log.start(),
                log.held().operations.len()
            ))
        })
    }

    /// How many leaves the journal holds: a rerun that writes nothing leaves it.
    pub fn leaves(&self) -> Result<u64, ServerError> {
        self.journal.with(|log, _| Ok(log.len()))
    }

    fn previews(
        &self,
    ) -> Result<std::sync::MutexGuard<'_, BTreeMap<String, Preview>>, ServerError> {
        self.previews.lock().map_err(|error| {
            unavailable(format!(
                "the seat import previews lock is poisoned: {error}"
            ))
        })
    }

    /// Keep `preview` as `seat`'s latest dry run.
    pub fn remember(&self, seat: &str, preview: Preview) -> Result<(), ServerError> {
        self.previews()?.insert(seat.to_owned(), preview);
        Ok(())
    }

    /// `seat`'s latest dry run.
    pub fn preview(&self, seat: &str) -> Result<Option<Preview>, ServerError> {
        Ok(self.previews()?.get(seat).cloned())
    }

    /// The import `operation`.
    pub fn operation(&self, operation: &str) -> Result<Option<Operation>, ServerError> {
        self.journal
            .with(|log, _| Ok(log.held().operation(operation).cloned()))
    }

    /// The completed import reserved under `key`, when there is one.
    pub fn completed(&self, key: &str) -> Result<Option<Operation>, ServerError> {
        self.journal.with(|log, _| {
            Ok(log
                .held()
                .keyed(key)
                .filter(|operation| operation.completed.is_some())
                .cloned())
        })
    }

    /// Every import of `seat`, oldest first.
    pub fn of_seat(&self, seat: &str) -> Result<Vec<Operation>, ServerError> {
        self.journal.with(|log, _| {
            Ok(log
                .held()
                .operations
                .values()
                .filter(|operation| operation.reserved.seat == seat)
                .cloned()
                .collect())
        })
    }

    /// `seat`'s selected import: its latest completed one.
    pub fn selected(&self, seat: &str) -> Result<Option<Operation>, ServerError> {
        self.journal
            .with(|log, _| Ok(log.held().selected(seat).cloned()))
    }

    /// Every import neither completed nor stopped.
    pub fn open_operations(&self) -> Result<Vec<String>, ServerError> {
        self.journal.with(|log, _| Ok(log.held().open()))
    }

    /// Refuse `import_incomplete` while `seat`'s latest import is in
    /// progress or stopped: no incomplete import is selected by a launch.
    pub fn require_selectable(&self, seat: &str) -> Result<(), ServerError> {
        self.journal.with(|log, _| match log.held().latest(seat) {
            Some(latest) if latest.completed.is_none() => Err(SeatImportError::Incomplete {
                seat: seat.to_owned(),
                operation: latest.reserved.operation.clone(),
                state: latest.state().to_owned(),
            }
            .into()),
            Some(_) | None => Ok(()),
        })
    }

    /// Reserve `reserved` once, before any destination is written. The same
    /// key completed answers its operation and writes nothing; the same key
    /// in progress resumes it; the operation id naming another key is
    /// refused `import_operation_reused`; another import of the seat in
    /// progress is refused `import_in_progress`.
    pub fn reserve(&self, reserved: Reserved) -> Result<Reservation, ServerError> {
        self.journal.with(|log, unavailable| {
            let held = log.held();
            if let Some(kept) = held.operation(&reserved.operation) {
                if kept.reserved.key != reserved.key {
                    return Err(SeatImportError::OperationReused {
                        operation: reserved.operation.clone(),
                    }
                    .into());
                }
                return Ok(answered(kept));
            }
            if let Some(kept) = held.keyed(&reserved.key) {
                return Ok(answered(kept));
            }
            if let Some(open) = held
                .latest(&reserved.seat)
                .filter(|latest| latest.state() == "in_progress")
            {
                return Err(SeatImportError::InProgress {
                    seat: reserved.seat.clone(),
                    operation: open.reserved.operation.clone(),
                }
                .into());
            }
            let operation = reserved.operation.clone();
            log.append(Line::Reserved(Box::new(reserved)), unavailable)?;
            Ok(Reservation::Fresh(operation))
        })
    }

    /// Keep one line of an import already reserved.
    pub fn record(&self, line: Line) -> Result<(), ServerError> {
        self.journal
            .with(|log, unavailable| log.append(line, unavailable))
    }
}

/// How a reservation of an import already kept is answered.
fn answered(kept: &Operation) -> Reservation {
    if kept.completed.is_some() {
        Reservation::Completed(Box::new(kept.clone()))
    } else {
        Reservation::Resumed(kept.reserved.operation.clone())
    }
}
