//! A confirmed import applied (AGENTS-003 R5): each destination written to
//! its own owner under the operation's stable step id and the revision the
//! plan bound, then one completed-import manifest that selects them.
//!
//! Before a step is written its owner is read back: a write this step
//! already made, whose reply was lost, is found and kept as reconciled,
//! never written again under a new id. A record whose revision moved since
//! the plan bound it, such as a person's edit while the import was
//! interrupted, stops the import by name and is left as the person left it.
//! An owner that cannot be read or written leaves the import in progress,
//! and it resumes, without a second confirmation, at the next start.
//!
//! The confirmation refused every running seat of the agent, so no session
//! renders, enforces or schedules from these records while they are
//! written, and a launch selects them only through the completed manifest
//! ([`crate::seat_import_store::SeatImports::require_selectable`]).
//! Schedules are kept paused with their original start, written with their
//! pause under one lock so no timer sees them live; a shared schedule a
//! second seat imports gains that seat's binding and is never written
//! twice. Rule transfers are kept in the receipt and written nowhere. Every
//! write is held behind the shared upgrade-intent fence.

use std::collections::BTreeMap;
use std::sync::Mutex;

use axum::http::StatusCode;

use crate::budgets_limits::Limits;
use crate::budgets_store::BudgetStore;
use crate::error::ServerError;
use crate::error_seat_import::SeatImportError;
use crate::provisioning_store::{ProvisioningStore, Version};
use crate::routes::AppState;
use crate::schedules_store::SchedulesKept;
use crate::seat_import_changes::{
    Change, Readback, member, ours, parsed, schedule, schedule_revision, scheduled, stored,
};
use crate::seat_import_plan::{DestinationEntry, record};
use crate::seat_import_state::{Completed, Halted, Line, Operation, Outcome, Step};
use crate::seat_import_store::SeatImports;
use crate::session::now;
use crate::variables_store::{Patch, VariablesKept};
use crate::words_store::{Save, WordsKept};

/// The destination owners, as this install keeps them.
pub struct Owners<'a> {
    /// The provisioning profiles.
    pub provisioning: Option<&'a Mutex<ProvisioningStore>>,
    /// The budgets.
    pub budgets: Option<&'a Mutex<BudgetStore>>,
    /// The words.
    pub words: Option<&'a WordsKept>,
    /// The variables.
    pub variables: Option<&'a VariablesKept>,
    /// The schedules.
    pub schedules: Option<&'a SchedulesKept>,
}

fn owned<'a, T>(owner: Option<&'a T>, kind: &str) -> Result<&'a T, ServerError> {
    owner.ok_or_else(|| {
        SeatImportError::DestinationUnconfigured {
            kind: kind.to_owned(),
        }
        .into()
    })
}

fn locked<'a, T>(
    owner: Option<&'a Mutex<T>>,
    kind: &str,
) -> Result<std::sync::MutexGuard<'a, T>, ServerError> {
    owned(owner, kind)?.lock().map_err(|error| {
        SeatImportError::Unavailable {
            reason: format!("the {kind} lock is poisoned: {error}"),
        }
        .into()
    })
}

impl<'a> Owners<'a> {
    /// The owners `state` keeps.
    pub fn of(state: &'a AppState) -> Self {
        Self {
            provisioning: state.provisioning.as_ref(),
            budgets: state.budgets.as_ref(),
            words: state.words.as_ref(),
            variables: state.variables.as_ref(),
            schedules: state.schedules.as_ref(),
        }
    }

    /// The revision `destination`'s owner holds now, for `agent`'s seat.
    pub fn revision(
        &self,
        destination: &DestinationEntry,
        agent: &str,
    ) -> Result<u64, ServerError> {
        match parsed(destination)? {
            Change::Profile(_) => {
                let mut store = locked(self.provisioning, "provisioning profiles")?;
                store.settle()?;
                Ok(store
                    .profile(agent)
                    .map_or(0, |profile| u64::from(profile.latest())))
            }
            Change::Template(change) => owned(self.words, "words")?
                .with(|log, _| Ok(log.held().template_revision(&change.name))),
            Change::Slot(change) => owned(self.words, "words")?
                .with(|log, _| Ok(log.held().revision(&change.layer, change.slot))),
            Change::Variable(change) => owned(self.variables, "variables")?
                .with(|log, _| Ok(log.held().map(&change.scope).revision)),
            Change::Limits(change) => {
                let mut store = locked(self.budgets, "budgets")?;
                store.settle()?;
                Ok(store
                    .held()
                    .limit_set(&change.holder)
                    .map_or(0, |limits| limits.version))
            }
            Change::Schedule(change) => owned(self.schedules, "schedules")?
                .with(|log| Ok(schedule_revision(log.held().item(&change.schedule.id)))),
            Change::Transfer => Ok(0),
        }
    }

    /// What `change`'s owner holds of `step`'s write over `expected`.
    fn readback(
        &self,
        step: &str,
        change: &Change,
        agent: &str,
        expected: u64,
    ) -> Result<Readback, ServerError> {
        let next = expected + 1;
        match change {
            Change::Profile(change) => {
                let mut store = locked(self.provisioning, "provisioning profiles")?;
                store.settle()?;
                Ok(match store.named(step) {
                    Some((held, version)) => ours(
                        held == agent && version.settings == change.settings,
                        u64::from(version.number),
                    ),
                    None => Readback::Absent,
                })
            }
            Change::Template(change) => owned(self.words, "words")?.with(|log, _| {
                let held = log.held().templates.get(&change.name).is_some_and(|held| {
                    held.revision == next && held.text == change.text.trim()
                });
                Ok(ours(held, next))
            }),
            Change::Slot(change) => owned(self.words, "words")?.with(|log, _| {
                let held = log
                    .held()
                    .layers
                    .get(&change.layer.key())
                    .and_then(|slots| slots.get(&change.slot))
                    .is_some_and(|held| {
                        held.revision == next && held.setting == stored(&change.setting)
                    });
                Ok(ours(held, next))
            }),
            Change::Variable(change) => owned(self.variables, "variables")?.with(|log, _| {
                let map = log.held().map(&change.scope);
                let held = map.values.get(&change.name).is_some_and(|held| {
                    held.revision == next
                        && held.value == change.value
                        && held.author == change.author
                        && held.expires_at == change.expires_at
                });
                Ok(ours(held, next))
            }),
            Change::Limits(change) => {
                let mut store = locked(self.budgets, "budgets")?;
                store.settle()?;
                let held = store.held().limit_set(&change.holder).is_some_and(|held| {
                    held.version == next
                        && held.limits == change.limits
                        && held.warn_at == change.warn_at
                });
                Ok(ours(held, next))
            }
            Change::Schedule(change) => owned(self.schedules, "schedules")?.with(|log| {
                let item = log.held().item(&change.schedule.id);
                Ok(scheduled(item, change, agent, expected))
            }),
            Change::Transfer => Ok(Readback::Ours(0)),
        }
    }

    /// Write `step`'s change over `expected` for `agent` as `by`, answering
    /// the revision it left; `partial` names a schedule already set and not
    /// yet paused.
    fn write(
        &self,
        step: &str,
        change: Change,
        (agent, by): (&str, &str),
        expected: u64,
        partial: bool,
    ) -> Result<u64, ServerError> {
        match change {
            Change::Profile(change) => {
                let from = u32::try_from(expected).map_err(|error| {
                    SeatImportError::DestinationMalformed {
                        record: format!("{} {agent}", record::PROFILE),
                        reason: error.to_string(),
                    }
                })?;
                let version = Version {
                    number: 0,
                    operation: step.to_owned(),
                    settings: change.settings,
                    set_by: by.to_owned(),
                    set_at: now(),
                    reviewed: None,
                };
                let mut store = locked(self.provisioning, "provisioning profiles")?;
                Ok(u64::from(store.set(agent, from, version)?))
            }
            Change::Template(change) => crate::words_store::template(
                owned(self.words, "words")?,
                &change.name,
                &change.text,
                expected,
                by,
            ),
            Change::Slot(change) => crate::words_store::set(
                owned(self.words, "words")?,
                Save {
                    layer: change.layer,
                    slot: change.slot,
                    setting: change.setting,
                    revision: expected,
                    by: by.to_owned(),
                },
            ),
            Change::Variable(change) => {
                let patch = Patch {
                    scope: change.scope,
                    revision: expected,
                    author: change.author,
                    values: BTreeMap::from([(change.name, change.value)]),
                    expires_at: change.expires_at,
                };
                let variables = owned(self.variables, "variables")?;
                Ok(crate::variables_store::patch(variables, patch)?.revision)
            }
            Change::Limits(change) => {
                let limits = Limits {
                    holder: change.holder,
                    limits: change.limits,
                    warn_at: change.warn_at,
                    version: 0,
                    by: by.to_owned(),
                    at: now(),
                };
                let mut store = locked(self.budgets, "budgets")?;
                Ok(store.set_limits(limits, expected)?.version)
            }
            Change::Schedule(change) => {
                let schedules = owned(self.schedules, "schedules")?;
                schedule(schedules, step, change.schedule, (agent, by), partial)
            }
            Change::Transfer => Ok(0),
        }
    }
}

/// Whether the shared upgrade fence holds writes: true while the upgrade
/// is reversible; an error when its intent cannot be read.
pub type Fence<'a> = &'a dyn Fn() -> std::io::Result<bool>;

fn fenced(fence: Fence<'_>) -> Result<(), ServerError> {
    match fence() {
        Ok(false) => Ok(()),
        Ok(true) => Err(SeatImportError::UpgradePending.into()),
        Err(error) => Err(SeatImportError::UpgradeIntentUnreadable {
            reason: error.to_string(),
        }
        .into()),
    }
}

/// A point an apply passes, at which a fixture may end the process.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Point<'a> {
    /// A step's write is durable and its journal line not yet kept: its
    /// reply may be lost.
    Written(&'a str),
    /// A step's journal line is kept.
    Kept(&'a str),
    /// Every step is kept and the manifest is not yet.
    Completing,
}

/// What an apply tells at each [`Point`]; an error ends the apply there.
pub trait Signal {
    /// `point` was reached.
    fn reached(&self, point: Point<'_>) -> Result<(), ServerError>;
}

/// The service's own apply, which nothing ends early.
pub struct Unsignalled;

impl Signal for Unsignalled {
    fn reached(&self, _: Point<'_>) -> Result<(), ServerError> {
        Ok(())
    }
}

/// An import applied: the operation as kept, and how many owner writes
/// this apply made.
#[derive(Debug, Clone, PartialEq)]
pub struct Applied {
    /// The operation.
    pub operation: Operation,
    /// The owner writes this apply made.
    pub writes: u64,
}

/// The stable id of the operation's `index`th step.
pub fn step_id(operation: &str, index: usize) -> String {
    format!("{operation}/{index:04}")
}

fn stopped(halted: &Halted) -> ServerError {
    SeatImportError::Stopped {
        operation: halted.operation.clone(),
        record: halted.record.clone(),
        reason: format!("{}: {}", halted.refusal, halted.reason),
    }
    .into()
}

/// Stop `operation` at `step` of `record` for `refused`, kept as a line.
fn halt(
    imports: &SeatImports,
    (operation, step, record): (&str, &str, String),
    refused: &ServerError,
) -> ServerError {
    let halted = Halted {
        operation: operation.to_owned(),
        step: step.to_owned(),
        record,
        refusal: refused.name(),
        reason: refused.to_string(),
        at: now(),
    };
    match imports.record(Line::Halted(halted.clone())) {
        Ok(()) => stopped(&halted),
        Err(unkept) => unkept,
    }
}

fn kept_operation(imports: &SeatImports, operation: &str) -> Result<Operation, ServerError> {
    imports.operation(operation)?.ok_or_else(|| {
        SeatImportError::Unavailable {
            reason: format!("no import is reserved under `{operation}`"),
        }
        .into()
    })
}

/// Apply the reserved import `operation` to `owners`, from its first step
/// not yet kept; a completed import answers as kept and writes nothing.
pub fn apply(
    imports: &SeatImports,
    owners: &Owners<'_>,
    operation: &str,
    fence: Fence<'_>,
    signal: &dyn Signal,
) -> Result<Applied, ServerError> {
    let held = kept_operation(imports, operation)?;
    if held.completed.is_some() {
        return Ok(Applied {
            operation: held,
            writes: 0,
        });
    }
    if let Some(halted) = &held.halted {
        return Err(stopped(halted));
    }
    let plan = &held.reserved.plan;
    let who = (plan.agent.as_str(), held.reserved.confirmation.person.as_str());
    let mut writes = 0;
    for (index, destination) in plan.destinations.iter().enumerate() {
        let step = step_id(operation, index);
        if held.step(&step).is_some() {
            continue;
        }
        fenced(fence)?;
        let record = member(destination);
        let expected = destination.expected_revision.ok_or_else(|| {
            SeatImportError::DestinationMalformed {
                record: record.clone(),
                reason: "the plan bound no revision".to_owned(),
            }
        })?;
        let change = parsed(destination)?;
        let keep = |revision: u64, outcome: Outcome| {
            imports.record(Line::Step(Step {
                operation: operation.to_owned(),
                step: step.clone(),
                record_kind: destination.record_kind.clone(),
                record_id: destination.record_id.clone(),
                expected_revision: expected,
                revision,
                outcome,
                at: now(),
            }))
        };
        let partial = match owners.readback(&step, &change, who.0, expected)? {
            Readback::Ours(revision) => {
                let outcome = if change == Change::Transfer {
                    Outcome::ReceiptOnly
                } else {
                    Outcome::Reconciled
                };
                keep(revision, outcome)?;
                signal.reached(Point::Kept(&step))?;
                continue;
            }
            Readback::Partial => true,
            Readback::Absent => {
                let now_held = owners.revision(destination, who.0)?;
                if now_held != expected {
                    let moved: ServerError = SeatImportError::DestinationMoved {
                        record: record.clone(),
                        expected,
                        held: now_held,
                    }
                    .into();
                    return Err(halt(imports, (operation, &step, record), &moved));
                }
                false
            }
        };
        let revision = match owners.write(&step, change, who, expected, partial) {
            Ok(revision) => revision,
            Err(unsure) if unsure.status() == StatusCode::SERVICE_UNAVAILABLE => {
                return Err(unsure);
            }
            Err(refused) => return Err(halt(imports, (operation, &step, record), &refused)),
        };
        writes += 1;
        signal.reached(Point::Written(&step))?;
        keep(revision, Outcome::Written)?;
        signal.reached(Point::Kept(&step))?;
    }
    fenced(fence)?;
    signal.reached(Point::Completing)?;
    let selected = (0..plan.destinations.len())
        .map(|index| step_id(operation, index))
        .collect();
    imports.record(Line::Completed(Completed {
        operation: operation.to_owned(),
        seat: held.reserved.seat.clone(),
        selected,
        at: now(),
    }))?;
    Ok(Applied {
        operation: kept_operation(imports, operation)?,
        writes,
    })
}

/// Resume every import a stop left in progress, without a second
/// confirmation, saying how each came out by name.
pub fn resume_at_start(state: &AppState) {
    let open = match state.seat_imports.open_operations() {
        Ok(open) => open,
        Err(refused) => {
            (state.say)(&format!(
                "seat imports were not resumed: {}: {refused}",
                refused.name()
            ));
            return;
        }
    };
    let fence = || crate::operator::upgrade_pending(state);
    for operation in open {
        let owners = Owners::of(state);
        match apply(&state.seat_imports, &owners, &operation, &fence, &Unsignalled) {
            Ok(applied) => (state.say)(&format!(
                "seat import {operation} resumed and completed with {} writes",
                applied.writes
            )),
            Err(refused) => (state.say)(&format!(
                "seat import {operation} was not resumed: {}: {refused}",
                refused.name()
            )),
        }
    }
}
