//! What the seat import journal folds to (AGENTS-003 R5), sealed in its
//! log's signed snapshot through the agents log engine.
//!
//! An import is one operation: reserved once, before any destination is
//! written, under the key of its seat, its source revision vector and its
//! plan revision; then one line for each destination step, under the step's
//! stable id, naming the revision it expected and the revision it left;
//! then either one completed line, which is the durable manifest that
//! selects the imported configuration, or one stopped line naming the
//! conflict. A completed or stopped operation takes no further line.
//!
//! The journal is a new record kind with its own versioned format; it has
//! no legacy format, and a snapshot in any other format is refused.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::agents_log::Folded;
use crate::seat_import_plan::Plan;

/// Who confirmed an import, and what admitted them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[schema(as = SeatImportConfirmation)]
#[serde(deny_unknown_fields)]
pub struct Confirmation {
    /// The person who confirmed it.
    pub person: String,
    /// What admitted them: `grant`, `administrator` or `responsible`.
    pub by: String,
    /// The `seat.import` grant exercised, when one was.
    pub grant: Option<String>,
    /// The grant log's use event recording that exercise.
    pub use_event: Option<u64>,
}

/// An import reserved, before any destination is written.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reserved {
    /// The operation id it was confirmed under.
    pub operation: String,
    /// The seat.
    pub seat: String,
    /// The key of its seat, source revision vector and plan revision.
    pub key: String,
    /// The plan confirmed, whole.
    pub plan: Plan,
    /// Who confirmed it.
    pub confirmation: Confirmation,
    /// When, in seconds since the Unix epoch.
    pub at: u64,
}

/// How a step's write stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[schema(as = SeatImportStepOutcome)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    /// Written by this import.
    Written,
    /// Found already written under this step when read back.
    Reconciled,
    /// Kept in the receipt only; no owner is written.
    ReceiptOnly,
}

/// One destination step, kept.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[schema(as = SeatImportStep)]
#[serde(deny_unknown_fields)]
pub struct Step {
    /// The operation.
    pub operation: String,
    /// The step's stable id.
    pub step: String,
    /// The destination kind.
    pub record_kind: String,
    /// The destination record.
    pub record_id: String,
    /// The revision the plan bound.
    pub expected_revision: u64,
    /// The revision the owner holds after the step.
    pub revision: u64,
    /// How the write stands.
    pub outcome: Outcome,
    /// When, in seconds since the Unix epoch.
    pub at: u64,
}

/// An import stopped on a named conflict.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[schema(as = SeatImportHalted)]
#[serde(deny_unknown_fields)]
pub struct Halted {
    /// The operation.
    pub operation: String,
    /// The step it stopped at.
    pub step: String,
    /// The record it stopped at.
    pub record: String,
    /// The refusal's name.
    pub refusal: String,
    /// Its words.
    pub reason: String,
    /// When, in seconds since the Unix epoch.
    pub at: u64,
}

/// The completed-import manifest: the one line that selects an import.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[schema(as = SeatImportCompleted)]
#[serde(deny_unknown_fields)]
pub struct Completed {
    /// The operation.
    pub operation: String,
    /// The seat.
    pub seat: String,
    /// Every step it selects, by its stable id, in order.
    pub selected: Vec<String>,
    /// When, in seconds since the Unix epoch.
    pub at: u64,
}

/// One line of the import journal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Line {
    /// An import reserved.
    Reserved(Box<Reserved>),
    /// A step kept.
    Step(Step),
    /// An import stopped.
    Halted(Halted),
    /// An import completed.
    Completed(Completed),
}

/// One import as its lines fold it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Operation {
    /// What was reserved.
    pub reserved: Reserved,
    /// Every step kept, in order.
    pub steps: Vec<Step>,
    /// Why it stopped, once it has.
    pub halted: Option<Halted>,
    /// Its manifest, once complete.
    pub completed: Option<Completed>,
    /// Raised by every line that changes it, from 1.
    pub revision: u64,
}

impl Operation {
    /// `completed`, `stopped` or `in_progress`.
    pub fn state(&self) -> &'static str {
        if self.completed.is_some() {
            "completed"
        } else if self.halted.is_some() {
            "stopped"
        } else {
            "in_progress"
        }
    }

    /// The step kept under `step`.
    pub fn step(&self, step: &str) -> Option<&Step> {
        self.steps.iter().find(|kept| kept.step == step)
    }

    fn open(&self) -> bool {
        self.completed.is_none() && self.halted.is_none()
    }
}

/// The journal as its lines fold it.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Journal {
    /// Every import, by operation id.
    pub operations: BTreeMap<String, Operation>,
    /// The operation each reserved key names, while it is not stopped.
    pub keys: BTreeMap<String, String>,
    /// Each seat's latest operation.
    pub latest: BTreeMap<String, String>,
    /// Each seat's latest completed operation: its selected import.
    pub selected: BTreeMap<String, String>,
}

impl Journal {
    /// The import `operation`.
    pub fn operation(&self, operation: &str) -> Option<&Operation> {
        self.operations.get(operation)
    }

    /// The import reserved under `key`, while it is not stopped.
    pub fn keyed(&self, key: &str) -> Option<&Operation> {
        self.keys
            .get(key)
            .and_then(|operation| self.operation(operation))
    }

    /// The seat's latest import.
    pub fn latest(&self, seat: &str) -> Option<&Operation> {
        self.latest
            .get(seat)
            .and_then(|operation| self.operation(operation))
    }

    /// The seat's selected import: its latest completed one.
    pub fn selected(&self, seat: &str) -> Option<&Operation> {
        self.selected
            .get(seat)
            .and_then(|operation| self.operation(operation))
    }

    /// Every import neither completed nor stopped, by operation id.
    pub fn open(&self) -> Vec<String> {
        self.latest
            .values()
            .filter(|operation| self.operation(operation).is_some_and(Operation::open))
            .cloned()
            .collect()
    }

    fn changed(&mut self, operation: &str) -> Result<&mut Operation, String> {
        let held = self
            .operations
            .get_mut(operation)
            .ok_or_else(|| format!("no import is reserved under `{operation}`"))?;
        if !held.open() {
            return Err(format!(
                "import `{operation}` is {} and takes no further line",
                held.state()
            ));
        }
        Ok(held)
    }
}

impl Folded for Journal {
    type Line = Line;
    const DOMAIN: &'static str = "lys/identity/seat-imports-state/v1";
    const FORMAT: &'static str = "lys-seat-imports-state/v1";
    const KIND: &'static str = "seat import";

    fn hold(&mut self, line: Line) -> Result<(), String> {
        match line {
            Line::Reserved(reserved) => {
                let operation = reserved.operation.clone();
                if self.operations.contains_key(&operation) {
                    return Err(format!("operation `{operation}` is already reserved"));
                }
                if let Some(held) = self.keys.get(&reserved.key) {
                    return Err(format!("key `{}` is reserved by `{held}`", reserved.key));
                }
                if let Some(open) = self.latest(&reserved.seat).filter(|held| held.open()) {
                    return Err(format!(
                        "seat `{}` is being imported by `{}`",
                        reserved.seat, open.reserved.operation
                    ));
                }
                self.keys.insert(reserved.key.clone(), operation.clone());
                self.latest.insert(reserved.seat.clone(), operation.clone());
                self.operations.insert(
                    operation,
                    Operation {
                        reserved: *reserved,
                        steps: Vec::new(),
                        halted: None,
                        completed: None,
                        revision: 1,
                    },
                );
            }
            Line::Step(step) => {
                let held = self.changed(&step.operation)?;
                if held.step(&step.step).is_some() {
                    return Err(format!("step `{}` is already kept", step.step));
                }
                held.steps.push(step);
                held.revision += 1;
            }
            Line::Halted(halted) => {
                let key = {
                    let held = self.changed(&halted.operation)?;
                    held.halted = Some(halted);
                    held.revision += 1;
                    held.reserved.key.clone()
                };
                self.keys.remove(&key);
            }
            Line::Completed(completed) => {
                let held = self.changed(&completed.operation)?;
                let destinations = held.reserved.plan.destinations.len();
                if held.steps.len() != destinations {
                    return Err(format!(
                        "import `{}` kept {} of {destinations} steps and cannot complete",
                        completed.operation,
                        held.steps.len()
                    ));
                }
                if completed.seat != held.reserved.seat {
                    return Err(format!(
                        "import `{}` is of seat `{}`, not `{}`",
                        completed.operation, held.reserved.seat, completed.seat
                    ));
                }
                let (seat, operation) = (completed.seat.clone(), completed.operation.clone());
                held.completed = Some(completed);
                held.revision += 1;
                self.selected.insert(seat, operation);
            }
        }
        Ok(())
    }
}
