//! What the goals' log folds to, and how that fold is sealed in the log's
//! signed snapshot so a start reads only the leaves after it.
//!
//! A goal, an expectation or a deliverable is held on an agent or a team,
//! with its words, its deadline, its standing and its reminders. Each
//! reminder is a timer whose next due instant is folded from the leaves: the
//! goal as set, each firing, each event it waits on and the mark that closes
//! it. So a restart keeps every timer where it stood, and a reminder fired
//! is never fired again for the same due instant.
//!
//! A firing names, before any runner is asked, the operation id each
//! session's delivery is asked under, and each delivery is then kept as the
//! runner answered it: pending until an answer is kept, then accepted,
//! delivered, uncertain or refused. Uncertain is never shown delivered.

use lys_runner::operations::{OperationOutcome, OperationState};
use serde::{Deserialize, Serialize};

#[cfg(test)]
#[path = "goals_operation_tests.rs"]
mod operation_tests;

/// The snapshot domain the goals' folded state is sealed under.
pub const DOMAIN: &str = "lys/identity/goals-state/v1";

const FORMAT: &str = "lys-goals-state/v1";

pub use crate::goals_types::{
    Change, Changed, Event, EvidenceKind, GoalError, Holder, HolderKind, Kind, Remind, Standing,
};

/// An item as it was set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[schema(as = GoalSet)]
#[serde(deny_unknown_fields)]
pub struct Goal {
    /// The operation id it was set under, which names it.
    pub id: String,
    /// What holds it.
    pub holder: Holder,
    /// What it is.
    pub kind: Kind,
    /// Its words.
    pub words: String,
    /// Its optional deadline, in seconds since the Unix epoch.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deadline: Option<u64>,
    /// Whether reminders are active; old entries are active without adding stored bytes.
    #[serde(
        default = "crate::goals_types::active_default",
        skip_serializing_if = "std::clone::Clone::clone"
    )]
    pub active: bool,
    /// For a deliverable, what proves it was handed over.
    pub evidence: Option<EvidenceKind>,
    /// The action on the holder whose grant holders may judge it, besides
    /// its responsible person.
    pub judged_by: Option<String>,
    /// Its reminders.
    pub reminders: Vec<Remind>,
    /// The person responsible for it.
    pub responsible: String,
    /// The identity that set it.
    pub set_by: String,
    /// When it was set, in seconds since the Unix epoch.
    pub at: u64,
}

impl Goal {
    /// Refuse an item that cannot support its reminders or deliverable evidence.
    pub fn check(&self) -> Result<(), GoalError> {
        if self.deadline.is_none()
            && self
                .reminders
                .iter()
                .any(|remind| matches!(remind, Remind::Before { .. }))
        {
            return Err(GoalError::ReminderNeedsDeadline);
        }
        if self.kind == Kind::Deliverable && self.evidence.is_none() {
            return Err(GoalError::EvidenceMissing {
                why: "a deliverable names the evidence that proves it: a commit, a document or a check",
            });
        }
        Ok(())
    }
}

/// A judgement of an item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[schema(as = GoalMark)]
#[serde(deny_unknown_fields)]
pub struct Marked {
    /// The operation id it was made under.
    pub operation: String,
    /// The item judged.
    pub goal: String,
    /// Met, missed or dropped.
    pub standing: Standing,
    /// The identity that judged it.
    pub by: String,
    /// Why, in their words.
    pub words: String,
    /// For a met deliverable, the evidence as the judge claims it, in their words.
    pub evidence: Option<String>,
    /// When, in seconds since the Unix epoch.
    pub at: u64,
}

/// Where one session's delivery of a reminder stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[schema(as = ReminderDelivery)]
#[serde(rename_all = "snake_case")]
pub enum Delivery {
    /// Asked, with no answer kept yet: asked again under the same id.
    Pending,
    /// Accepted by the runner, waiting for the session's turn boundary.
    Accepted,
    /// Typed into the session.
    Delivered,
    /// Whether it reached the session cannot be known; never typed again.
    Uncertain,
    /// Not delivered, by name.
    Refused,
}

impl Delivery {
    /// Whether it is still asked of the runner.
    pub fn unsettled(self) -> bool {
        matches!(self, Self::Pending | Self::Accepted)
    }
}

/// One session's delivery of a reminder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[schema(as = ReminderSent)]
#[serde(deny_unknown_fields)]
pub struct Sent {
    /// The session.
    pub session: String,
    /// The runner operation id it is asked under.
    pub operation: String,
    /// Where it stands.
    pub state: Delivery,
    /// The runner's words, or why it stands as it does.
    pub words: String,
    /// When it came to stand there, in seconds since the Unix epoch.
    pub at: u64,
}

/// A reminder fired.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[schema(as = ReminderFired)]
#[serde(deny_unknown_fields)]
pub struct Fired {
    /// The operation id naming this firing.
    pub operation: String,
    /// The item.
    pub goal: String,
    /// The reminder, by its place in the item's reminders.
    pub reminder: usize,
    /// When it fell due, in seconds since the Unix epoch.
    pub due: u64,
    /// When it fired, in seconds since the Unix epoch.
    pub fired: u64,
    /// Whether it fell due while the service was stopped.
    pub late: bool,
    /// The text typed: the item's words and the time left.
    pub text: String,
    /// Each session it is delivered into.
    pub sent: Vec<Sent>,
    /// Why no session was asked, by name, when none was.
    pub refused: Option<String>,
}

/// A runner's answer to one delivery.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[schema(as = ReminderAnswered)]
#[serde(deny_unknown_fields)]
pub struct Answered {
    /// The runner operation id.
    pub operation: String,
    /// Where the delivery stands.
    pub state: Delivery,
    /// The runner's words.
    pub words: String,
    /// When it was kept, in seconds since the Unix epoch.
    pub at: u64,
}

impl Answered {
    /// The delivery as the runner's `outcome` says it stands, kept at `at`.
    pub fn from_runner(outcome: &OperationOutcome, at: u64) -> Self {
        let state = match outcome.state {
            OperationState::Accepted | OperationState::Delivering => Delivery::Accepted,
            OperationState::Delivered | OperationState::Confirmed => Delivery::Delivered,
            OperationState::Uncertain => Delivery::Uncertain,
            OperationState::Refused => Delivery::Refused,
        };
        Self {
            operation: outcome.operation.clone(),
            state,
            words: outcome.words.clone(),
            at,
        }
    }
}

/// An event kept for the items waiting on it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[schema(as = GoalEvented)]
#[serde(deny_unknown_fields)]
pub struct Evented {
    /// The operation id naming it.
    pub operation: String,
    /// The event.
    pub event: Event,
    /// The items it concerns.
    pub goals: Vec<String>,
    /// When it happened, in seconds since the Unix epoch.
    pub at: u64,
}

/// One leaf of the goals' log.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "line", rename_all = "snake_case")]
pub enum Line {
    /// An item set.
    Set(Goal),
    /// An item judged.
    Marked(Marked),
    /// The activity or words of an aim changed.
    Changed(Changed),
    /// A reminder fired.
    Fired(Fired),
    /// A runner's answer to a delivery.
    Answered(Answered),
    /// An event kept.
    Evented(Evented),
}

/// A reminder's timer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[schema(as = ReminderTimer)]
#[serde(deny_unknown_fields)]
pub struct Timer {
    /// When it falls due.
    pub remind: Remind,
    /// Its next due instant, in seconds since the Unix epoch; null when
    /// nothing is due: fired for the last time, waiting on an event, or its
    /// item closed.
    pub next_due: Option<u64>,
}

/// An item as its log folds it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[schema(as = GoalItem)]
#[serde(deny_unknown_fields)]
pub struct Item {
    /// The item as set.
    pub goal: Goal,
    /// Where it stands.
    pub standing: Standing,
    /// The judgement that closed it.
    pub marked: Option<Marked>,
    /// Its reminders' timers, in the order set.
    pub timers: Vec<Timer>,
    /// Every reminder fired, in the order fired.
    pub fired: Vec<Fired>,
    /// Changes kept after the original set; absent from old snapshots.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub changes: Vec<Changed>,
}

impl Item {
    /// Whether this aim currently sends reminders.
    pub fn active(&self) -> bool {
        self.changes
            .iter()
            .rev()
            .find_map(|changed| match changed.change {
                Change::Active { active } => Some(active),
                Change::Words { .. } => None,
            })
            .unwrap_or(self.goal.active)
    }

    /// The current words, retaining the original set for idempotent retries.
    pub fn words(&self) -> &str {
        self.changes
            .iter()
            .rev()
            .find_map(|changed| match &changed.change {
                Change::Words { words } => Some(words.as_str()),
                Change::Active { .. } => None,
            })
            .unwrap_or(&self.goal.words)
    }
}

/// A reminder due.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Due {
    /// The item.
    pub goal: String,
    /// The reminder, by its place.
    pub reminder: usize,
    /// When it fell due.
    pub due: u64,
}

fn first_due(goal: &Goal, remind: &Remind) -> Option<u64> {
    match remind {
        Remind::Before { seconds } => goal
            .deadline
            .map(|deadline| deadline.saturating_sub(*seconds).max(goal.at)),
        Remind::Every { seconds } => Some(goal.at.saturating_add(*seconds))
            .filter(|due| goal.deadline.is_none_or(|deadline| *due <= deadline)),
        Remind::On { .. } => None,
    }
}

fn after(goal: &Goal, remind: &Remind, fired: u64) -> Option<u64> {
    match remind {
        Remind::Every { seconds } if *seconds > 0 => {
            let steps = fired.saturating_sub(goal.at) / seconds + 1;
            let next = goal.at.saturating_add(steps.saturating_mul(*seconds));
            Some(next).filter(|due| goal.deadline.is_none_or(|deadline| *due <= deadline))
        }
        _ => None,
    }
}

/// The goals as their log folds them, in the order set.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Held {
    /// The items.
    pub items: Vec<Item>,
    /// The operation ids of the events kept.
    pub events: Vec<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Sealed {
    format: String,
    held: Held,
}

impl Held {
    /// The item `id`.
    pub fn item(&self, id: &str) -> Option<&Item> {
        self.items
            .iter()
            .inspect(|_| {
                #[cfg(test)]
                crate::folded_work::visit(crate::folded_work::Work::GoalOperation);
            })
            .find(|item| item.goal.id == id)
    }

    fn item_mut(&mut self, id: &str) -> Result<&mut Item, String> {
        self.items
            .iter_mut()
            .find(|item| item.goal.id == id)
            .ok_or_else(|| format!("no goal `{id}` is held"))
    }

    /// The judgement kept under `operation`.
    pub fn marked(&self, operation: &str) -> Option<&Marked> {
        self.items
            .iter()
            .filter_map(|item| item.marked.as_ref())
            .inspect(|_| {
                #[cfg(test)]
                crate::folded_work::visit(crate::folded_work::Work::GoalOperation);
            })
            .find(|marked| marked.operation == operation)
    }

    /// The change kept under an operation id.
    pub fn changed(&self, operation: &str) -> Option<&Changed> {
        self.items
            .iter()
            .flat_map(|item| &item.changes)
            .inspect(|_| {
                #[cfg(test)]
                crate::folded_work::visit(crate::folded_work::Work::GoalOperation);
            })
            .find(|changed| changed.operation == operation)
    }

    /// Whether `operation` names a firing, event or aim change already kept.
    pub fn kept(&self, operation: &str) -> bool {
        self.changed(operation).is_some()
            || self.events.iter().any(|event| event == operation)
            || self
                .items
                .iter()
                .flat_map(|item| &item.fired)
                .any(|fired| fired.operation == operation)
    }

    /// Every item held on `holder`, in the order set.
    pub fn of_holder<'a>(&'a self, holder: &'a Holder) -> impl Iterator<Item = &'a Item> + 'a {
        self.items
            .iter()
            .filter(move |item| &item.goal.holder == holder)
    }

    /// Every reminder of an open item due at `now`.
    pub fn due(&self, now: u64) -> Vec<Due> {
        let mut due = Vec::new();
        for item in self
            .items
            .iter()
            .filter(|item| item.standing == Standing::Open && item.active())
        {
            for (reminder, timer) in item.timers.iter().enumerate() {
                if let Some(at) = timer.next_due.filter(|at| *at <= now) {
                    due.push(Due {
                        goal: item.goal.id.clone(),
                        reminder,
                        due: at,
                    });
                }
            }
        }
        due
    }

    /// The earliest instant any timer falls due.
    pub fn next_due(&self) -> Option<u64> {
        self.items
            .iter()
            .filter(|item| item.standing == Standing::Open && item.active())
            .flat_map(|item| item.timers.iter().filter_map(|timer| timer.next_due))
            .min()
    }

    /// Fold one leaf. A leaf that contradicts what came before is refused,
    /// since every kept leaf was checked against it.
    pub fn hold(&mut self, line: Line) -> Result<(), String> {
        match line {
            Line::Set(goal) => {
                goal.check().map_err(|error| error.to_string())?;
                if self.item(&goal.id).is_some() {
                    return Err(format!("goal `{}` is already set", goal.id));
                }
                let timers = goal
                    .reminders
                    .iter()
                    .map(|remind| Timer {
                        remind: remind.clone(),
                        next_due: first_due(&goal, remind),
                    })
                    .collect();
                self.items.push(Item {
                    goal,
                    standing: Standing::Open,
                    marked: None,
                    timers,
                    fired: Vec::new(),
                    changes: Vec::new(),
                });
            }
            Line::Marked(marked) => {
                let item = self.item_mut(&marked.goal)?;
                if item.standing != Standing::Open || marked.standing == Standing::Open {
                    return Err(format!("goal `{}` cannot be marked so", marked.goal));
                }
                item.standing = marked.standing;
                item.marked = Some(marked);
                for timer in &mut item.timers {
                    timer.next_due = None;
                }
            }
            Line::Changed(changed) => {
                changed.change.check().map_err(|error| error.to_string())?;
                if self.changed(&changed.operation).is_some() {
                    return Err(format!(
                        "operation `{}` already names a change",
                        changed.operation
                    ));
                }
                let item = self.item_mut(&changed.goal)?;
                if item.standing != Standing::Open {
                    return Err(format!("goal `{}` is already closed", changed.goal));
                }
                item.changes.push(changed);
            }
            Line::Fired(fired) => self.fire(fired)?,
            Line::Answered(answered) => self.answer(&answered)?,
            Line::Evented(evented) => {
                if self.kept(&evented.operation) {
                    return Err(format!("operation `{}` is already kept", evented.operation));
                }
                self.events.push(evented.operation.clone());
                for goal in &evented.goals {
                    let item = self.item_mut(goal)?;
                    if item.standing != Standing::Open {
                        continue;
                    }
                    for timer in &mut item.timers {
                        if timer.remind
                            == (Remind::On {
                                event: evented.event,
                            })
                        {
                            timer.next_due = Some(timer.next_due.unwrap_or(evented.at));
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn fire(&mut self, fired: Fired) -> Result<(), String> {
        if self.kept(&fired.operation) {
            return Err(format!(
                "operation `{}` already names a firing",
                fired.operation
            ));
        }
        let item = self.item_mut(&fired.goal)?;
        let goal = item.goal.clone();
        let timer = item
            .timers
            .get_mut(fired.reminder)
            .ok_or_else(|| format!("goal `{}` has no reminder {}", goal.id, fired.reminder))?;
        timer.next_due = after(&goal, &timer.remind, fired.fired);
        item.fired.push(fired);
        Ok(())
    }

    fn answer(&mut self, answered: &Answered) -> Result<(), String> {
        let sent = self
            .items
            .iter_mut()
            .flat_map(|item| item.fired.iter_mut())
            .flat_map(|fired| fired.sent.iter_mut())
            .find(|sent| sent.operation == answered.operation)
            .ok_or_else(|| format!("no delivery `{}` is held", answered.operation))?;
        sent.state = answered.state;
        sent.words.clone_from(&answered.words);
        sent.at = answered.at;
        Ok(())
    }

    /// The delivery asked under `operation`.
    pub fn sent(&self, operation: &str) -> Option<&Sent> {
        self.items
            .iter()
            .flat_map(|item| &item.fired)
            .flat_map(|fired| &fired.sent)
            .inspect(|_| {
                #[cfg(test)]
                crate::folded_work::visit(crate::folded_work::Work::GoalOperation);
            })
            .find(|sent| sent.operation == operation)
    }

    /// Every delivery still asked of a runner, with the text it types.
    pub fn unsettled(&self) -> Vec<(Sent, String)> {
        self.items
            .iter()
            .filter(|item| item.active())
            .flat_map(|item| &item.fired)
            .flat_map(|fired| {
                fired
                    .sent
                    .iter()
                    .filter(|sent| sent.state.unsettled())
                    .map(|sent| (sent.clone(), fired.text.clone()))
            })
            .collect()
    }

    /// Fold every leaf of `tail`, in order.
    pub fn fold(&mut self, tail: &lys_log_store::Tail) -> Result<(), String> {
        for (index, bytes) in (tail.from..).zip(&tail.leaves) {
            let line = serde_json::from_slice(bytes)
                .map_err(|error| format!("leaf {index} is not a goal line: {error}"))?;
            self.hold(line)
                .map_err(|reason| format!("leaf {index}: {reason}"))?;
        }
        Ok(())
    }

    /// The state a snapshot seals.
    pub fn encode(&self) -> Result<Vec<u8>, String> {
        serde_json::to_vec(&Sealed {
            format: FORMAT.to_owned(),
            held: self.clone(),
        })
        .map_err(|error| format!("goals state: {error}"))
    }

    /// The state a snapshot sealed, refused by reason unless it reads whole
    /// in this format.
    pub fn decode(bytes: &[u8]) -> Result<Self, String> {
        let sealed: Sealed =
            serde_json::from_slice(bytes).map_err(|error| format!("goals state: {error}"))?;
        if sealed.format != FORMAT {
            return Err(format!(
                "goals state is in format {}, not {FORMAT}",
                sealed.format
            ));
        }
        Ok(sealed.held)
    }
}
