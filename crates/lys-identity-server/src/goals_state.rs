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
use std::sync::Arc;

#[path = "goals_index.rs"]
mod index;

#[cfg(test)]
#[path = "goals_operation_tests.rs"]
mod operation_tests;

/// The snapshot domain the goals' folded state is sealed under.
pub const DOMAIN: &str = "lys/identity/goals-state/v1";

const FORMAT: &str = "lys-goals-state/v2";

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
    /// A responsible person authorises a separate occurrence after uncertain delivery.
    Resent(Resent),
    /// A runner's answer to a delivery.
    Answered(Answered),
    /// An event kept.
    Evented(Evented),
}

/// A distinct occurrence that leaves the regular reminder timer unchanged.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Resent {
    /// The new occurrence and its intended session deliveries.
    pub fired: Fired,
    /// The original session operation whose delivery remains uncertain.
    pub prior: String,
    /// The responsible person authorising possible prior delivery.
    pub by: String,
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
#[serde(deny_unknown_fields, from = "Records")]
pub struct Held {
    /// The items.
    pub items: Vec<Item>,
    /// The operation ids of the events kept.
    pub events: Vec<String>,
    /// Prior delivery identities retained by explicitly resent occurrences.
    pub resends: std::collections::BTreeMap<String, String>,
    #[serde(skip)]
    index: Arc<index::Index>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Records {
    items: Vec<Item>,
    events: Vec<String>,
    resends: std::collections::BTreeMap<String, String>,
}

impl From<Records> for Held {
    fn from(records: Records) -> Self {
        let index = Arc::new(index::Index::of(&records.items, &records.events));
        Self {
            items: records.items,
            events: records.events,
            resends: records.resends,
            index,
        }
    }
}

#[derive(Serialize)]
struct Sealing<'a> {
    format: &'static str,
    held: &'a Held,
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
        self.index
            .items
            .get(id)
            .and_then(|position| self.items.get(*position))
    }

    fn position(&self, id: &str) -> Result<usize, String> {
        self.index
            .items
            .get(id)
            .copied()
            .ok_or_else(|| format!("no goal `{id}` is held"))
    }

    fn item_mut(&mut self, id: &str) -> Result<&mut Item, String> {
        let position = self.position(id)?;
        self.items
            .get_mut(position)
            .ok_or_else(|| format!("no goal `{id}` is held"))
    }

    /// The judgement kept under `operation`.
    pub fn marked(&self, operation: &str) -> Option<&Marked> {
        self.items
            .get(*self.index.marked.get(operation)?)?
            .marked
            .as_ref()
    }

    /// The change kept under an operation id.
    pub fn changed(&self, operation: &str) -> Option<&Changed> {
        let (item, change) = self.index.changed.get(operation)?;
        self.items.get(*item)?.changes.get(*change)
    }

    /// Whether `operation` names a firing, event or aim change already kept.
    pub fn kept(&self, operation: &str) -> bool {
        self.index.kept.contains(operation)
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
                Arc::make_mut(&mut self.index)
                    .items
                    .insert(goal.id.clone(), self.items.len());
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
                let position = self.position(&marked.goal)?;
                let operation = marked.operation.clone();
                let item = self.item_mut(&marked.goal)?;
                if item.standing != Standing::Open || marked.standing == Standing::Open {
                    return Err(format!("goal `{}` cannot be marked so", marked.goal));
                }
                item.standing = marked.standing;
                item.marked = Some(marked);
                for timer in &mut item.timers {
                    timer.next_due = None;
                }
                Arc::make_mut(&mut self.index).mark(position, &operation);
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
                let change = item.changes.len();
                let position = self.position(&changed.goal)?;
                Arc::make_mut(&mut self.index).change(position, change, &changed);
                self.items[position].changes.push(changed);
            }
            Line::Fired(fired) => self.fire(fired)?,
            Line::Resent(resent) => {
                self.check_resent(&resent)?;
                let position = self.position(&resent.fired.goal)?;
                let firing = self.items[position].fired.len();
                Arc::make_mut(&mut self.index).fire(position, firing, &resent.fired);
                self.resends
                    .insert(resent.fired.operation.clone(), resent.prior);
                self.items[position].fired.push(resent.fired);
            }
            Line::Answered(answered) => self.answer(&answered)?,
            Line::Evented(evented) => {
                if self.kept(&evented.operation) {
                    return Err(format!("operation `{}` is already kept", evented.operation));
                }
                self.events.push(evented.operation.clone());
                Arc::make_mut(&mut self.index)
                    .kept
                    .insert(evented.operation.clone());
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
        let position = self.position(&fired.goal)?;
        let item = self.item_mut(&fired.goal)?;
        let goal = item.goal.clone();
        let timer = item
            .timers
            .get_mut(fired.reminder)
            .ok_or_else(|| format!("goal `{}` has no reminder {}", goal.id, fired.reminder))?;
        timer.next_due = after(&goal, &timer.remind, fired.fired);
        let firing = item.fired.len();
        Arc::make_mut(&mut self.index).fire(position, firing, &fired);
        self.items[position].fired.push(fired);
        Ok(())
    }

    /// Validate every resend input before a leaf or folded state is changed.
    pub fn check_resent(&self, resent: &Resent) -> Result<(), String> {
        let location = self
            .index
            .sent
            .get(&resent.prior)
            .ok_or("resent prior operation is unknown")?;
        let prior = self.pending_at(*location)?;
        if prior.sent.state != Delivery::Uncertain {
            return Err("resent prior delivery is not uncertain".to_owned());
        }
        if resent.by != prior.item.goal.responsible {
            return Err("resent caller is not the responsible person".to_owned());
        }
        if prior.item.standing != Standing::Open || !prior.active {
            return Err("resent goal is closed or inactive".to_owned());
        }
        if resent.fired.goal != prior.item.goal.id
            || resent.fired.reminder != prior.fired.reminder
            || resent.fired.due != prior.fired.due
            || resent.fired.fired < prior.sent.at
        {
            return Err(
                "resent occurrence does not preserve its prior goal and due instant".to_owned(),
            );
        }
        if self.kept(&resent.fired.operation)
            || self.index.firings.contains_key(&resent.fired.operation)
        {
            return Err("resent occurrence identity was already used".to_owned());
        }
        if resent.fired.sent.len() != 1 || resent.fired.refused.is_some() {
            return Err("resent occurrence names exactly one intended delivery".to_owned());
        }
        let sent = &resent.fired.sent[0];
        if sent.session.is_empty()
            || sent.operation.is_empty()
            || sent.state != Delivery::Pending
            || self.index.sent.contains_key(&sent.operation)
            || sent.operation == resent.prior
        {
            return Err("resent delivery has an invalid or reused identity".to_owned());
        }
        Ok(())
    }

    /// The firing under one stable identity, without walking goal history.
    pub fn firing(&self, operation: &str) -> Option<&Fired> {
        let (item, firing) = self.index.firings.get(operation)?;
        self.items.get(*item)?.fired.get(*firing)
    }

    fn answer(&mut self, answered: &Answered) -> Result<(), String> {
        let (item, firing, delivery) = self
            .index
            .sent
            .get(&answered.operation)
            .copied()
            .ok_or_else(|| format!("no delivery `{}` is held", answered.operation))?;
        let sent = self
            .items
            .get_mut(item)
            .and_then(|item| item.fired.get_mut(firing))
            .and_then(|fired| fired.sent.get_mut(delivery))
            .ok_or_else(|| format!("no delivery `{}` is held", answered.operation))?;
        sent.state = answered.state;
        sent.words.clone_from(&answered.words);
        sent.at = answered.at;
        Arc::make_mut(&mut self.index).answer((item, firing, delivery), sent);
        Ok(())
    }

    /// The delivery asked under `operation`.
    pub fn sent(&self, operation: &str) -> Option<&Sent> {
        let (item, firing, delivery) = self.index.sent.get(operation)?;
        self.items
            .get(*item)?
            .fired
            .get(*firing)?
            .sent
            .get(*delivery)
    }

    /// Every delivery still asked of a runner, with the text it types.
    pub fn unsettled(&self) -> Result<Vec<(Sent, String)>, String> {
        let mut unsettled = Vec::with_capacity(self.index.pending.len());
        for location in self.index.pending.keys() {
            let pending = self.pending_at(*location)?;
            if pending.active && pending.item.standing == Standing::Open {
                unsettled.push((
                    pending.sent.clone(),
                    crate::goals_store::text_with_words(
                        pending.item,
                        pending.words,
                        pending.fired.fired,
                    ),
                ));
            }
        }
        Ok(unsettled)
    }

    /// Pending occurrences in record order, without visiting settled history.
    pub fn pending(&self) -> Result<Vec<PendingReminder<'_>>, String> {
        self.index
            .pending
            .keys()
            .map(|location| self.pending_at(*location))
            .collect()
    }

    /// Pending deliveries for one session, borrowing their current aim and words.
    pub fn pending_for_session(&self, session: &str) -> Result<Vec<PendingReminder<'_>>, String> {
        self.index
            .pending_by_session
            .get(session)
            .into_iter()
            .flat_map(|locations| locations.iter())
            .map(|location| self.pending_at(*location))
            .collect()
    }

    /// Pending deliveries for one aim, including those awaiting a named refusal.
    pub fn pending_for_goal(&self, goal: &str) -> Result<Vec<PendingReminder<'_>>, String> {
        self.index
            .items
            .get(goal)
            .and_then(|position| self.index.pending_by_goal.get(position))
            .into_iter()
            .flat_map(|locations| locations.iter())
            .map(|location| self.pending_at(*location))
            .collect()
    }

    /// The current queued delivery under one stable operation identity.
    pub fn pending_operation(
        &self,
        operation: &str,
    ) -> Result<Option<PendingReminder<'_>>, String> {
        self.index
            .sent
            .get(operation)
            .filter(|location| self.index.pending.contains_key(location))
            .map(|location| self.pending_at(*location))
            .transpose()
    }

    fn pending_at(&self, location: index::Location) -> Result<PendingReminder<'_>, String> {
        let (position, firing, delivery) = location;
        let item = self
            .items
            .get(position)
            .ok_or_else(|| format!("pending reminder item {position} is absent"))?;
        let fired = item.fired.get(firing).ok_or_else(|| {
            format!(
                "pending reminder occurrence {firing} is absent from goal `{}`",
                item.goal.id
            )
        })?;
        let sent = fired.sent.get(delivery).ok_or_else(|| {
            format!(
                "pending reminder delivery {delivery} is absent from occurrence `{}`",
                fired.operation
            )
        })?;
        let words = match self.index.current_words.get(&position) {
            Some(change) => match &item
                .changes
                .get(*change)
                .ok_or_else(|| {
                    format!(
                        "current words revision {change} is absent from goal `{}`",
                        item.goal.id
                    )
                })?
                .change
            {
                Change::Words { words } => words.as_str(),
                Change::Active { .. } => {
                    return Err(format!(
                        "current words revision {change} of goal `{}` changes activity instead",
                        item.goal.id
                    ));
                }
            },
            None => item.goal.words.as_str(),
        };
        Ok(PendingReminder {
            item,
            fired,
            sent,
            prior: self.resends.get(&fired.operation).map(String::as_str),
            words,
            active: self
                .index
                .current_active
                .get(&position)
                .copied()
                .unwrap_or(item.goal.active),
            version: item
                .changes
                .last()
                .map_or(item.goal.id.as_str(), |change| change.operation.as_str()),
        })
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
        serde_json::to_vec(&Sealing {
            format: FORMAT,
            held: self,
        })
        .map_err(|error| format!("goals state: {error}"))
    }

    /// The state a snapshot sealed, refused by reason unless it reads whole
    /// in this format.
    pub fn decode(bytes: &[u8]) -> Result<Self, String> {
        let value: serde_json::Value =
            serde_json::from_slice(bytes).map_err(|error| format!("goals state: {error}"))?;
        let format = value
            .get("format")
            .and_then(serde_json::Value::as_str)
            .ok_or("goals state has no format")?;
        if format != FORMAT {
            return Err(format!("goals state is in format {format}, not {FORMAT}",));
        }
        let sealed: Sealed =
            serde_json::from_value(value).map_err(|error| format!("goals state: {error}"))?;
        Ok(sealed.held)
    }
}

/// One pending occurrence with borrowed current authority facts.
pub struct PendingReminder<'a> {
    /// The aim and its current standing.
    pub item: &'a Item,
    /// The stable occurrence and original due instant.
    pub fired: &'a Fired,
    /// This session's delivery receipt.
    pub sent: &'a Sent,
    /// A possible earlier delivery, for an explicitly authorised resend.
    pub prior: Option<&'a str>,
    /// The currently saved words, without copying edit history.
    pub words: &'a str,
    /// Whether the currently saved policy permits reminders.
    pub active: bool,
    /// The operation that names the current saved revision.
    pub version: &'a str,
}
