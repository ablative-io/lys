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

use axum::http::StatusCode;
use lys_runner::operations::{OperationOutcome, OperationState};
use serde::{Deserialize, Serialize};

/// The snapshot domain the goals' folded state is sealed under.
pub const DOMAIN: &str = "lys/identity/goals-state/v1";

const FORMAT: &str = "lys-goals-state/v1";

/// Everything the goals refuse, each by name.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum GoalError {
    /// The goals are not configured, or their log could not be read or written.
    #[error("goals_unavailable: {reason}")]
    Unavailable {
        /// Why.
        reason: String,
    },
    /// No goal by that id is visible to the caller.
    #[error("goal_unknown: no goal by that id is visible to the caller")]
    Unknown,
    /// The operation id already names a goal act in other words.
    #[error(
        "goal_reused: operation `{operation}` already names a goal act in other words: send this act under a new operation id"
    )]
    Reused {
        /// The operation id.
        operation: String,
    },
    /// The goal is no longer open and takes no other mark.
    #[error("goal_closed: goal `{goal}` is already {standing}")]
    Closed {
        /// The goal.
        goal: String,
        /// Its standing.
        standing: &'static str,
    },
    /// The agent a goal judges asked to mark that goal.
    #[error(
        "not_your_judgement: agent `{agent}` is judged by this goal and may not mark it; its responsible person or a holder of the relation the goal names does"
    )]
    NotYourJudgement {
        /// The agent.
        agent: String,
    },
    /// A deliverable names no evidence, or is marked met without a claim of it.
    #[error("evidence_missing: {why}")]
    EvidenceMissing {
        /// What is missing.
        why: &'static str,
    },
}

impl GoalError {
    /// How the refusal is answered over HTTP.
    pub(crate) fn status(&self) -> StatusCode {
        match self {
            Self::Unavailable { .. } => StatusCode::SERVICE_UNAVAILABLE,
            Self::Unknown => StatusCode::NOT_FOUND,
            Self::Reused { .. } | Self::Closed { .. } => StatusCode::CONFLICT,
            Self::NotYourJudgement { .. } => StatusCode::FORBIDDEN,
            Self::EvidenceMissing { .. } => StatusCode::BAD_REQUEST,
        }
    }
}

/// What an item is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[schema(as = GoalKind)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    /// An outcome.
    Goal,
    /// A standard the holder's work is held to.
    Expectation,
    /// A named thing handed over, with the evidence that proves it.
    Deliverable,
}

/// What proves a deliverable was handed over.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[schema(as = GoalEvidence)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceKind {
    /// A landed commit.
    Commit,
    /// A document.
    Document,
    /// A passing check.
    Check,
}

/// Where an item stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[schema(as = GoalStanding)]
#[serde(rename_all = "snake_case")]
pub enum Standing {
    /// Not yet judged.
    Open,
    /// Judged met.
    Met,
    /// Judged missed.
    Missed,
    /// Dropped by its responsible person or judge.
    Dropped,
}

impl Standing {
    /// Its name.
    pub fn name(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Met => "met",
            Self::Missed => "missed",
            Self::Dropped => "dropped",
        }
    }
}

/// What holds an item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[schema(as = GoalHolderKind)]
#[serde(rename_all = "snake_case")]
pub enum HolderKind {
    /// An agent.
    Agent,
    /// A team.
    Team,
}

/// The agent or team an item is held on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[schema(as = GoalHolder)]
#[serde(deny_unknown_fields)]
pub struct Holder {
    /// An agent or a team.
    pub kind: HolderKind,
    /// Its id.
    pub id: String,
}

/// An event a reminder waits on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[schema(as = GoalEvent)]
#[serde(rename_all = "snake_case")]
pub enum Event {
    /// A session of the holder compacted its context.
    Compaction,
}

/// When a reminder falls due.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[schema(as = GoalReminder)]
#[serde(tag = "when", rename_all = "snake_case", deny_unknown_fields)]
pub enum Remind {
    /// Once, this many seconds before the deadline, and never before the
    /// item was set.
    Before {
        /// Seconds before the deadline.
        seconds: u64,
    },
    /// Every this many seconds from when the item was set, until the deadline.
    Every {
        /// Seconds between reminders.
        seconds: u64,
    },
    /// Each time the event is kept for the item.
    On {
        /// The event.
        event: Event,
    },
}

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
    /// Its deadline, in seconds since the Unix epoch.
    pub deadline: u64,
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
    /// The refusal a deliverable naming no evidence is answered with.
    pub fn check(&self) -> Result<(), GoalError> {
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
        Remind::Before { seconds } => Some(goal.deadline.saturating_sub(*seconds).max(goal.at)),
        Remind::Every { seconds } => {
            Some(goal.at.saturating_add(*seconds)).filter(|due| *due <= goal.deadline)
        }
        Remind::On { .. } => None,
    }
}

fn after(goal: &Goal, remind: &Remind, fired: u64) -> Option<u64> {
    match remind {
        Remind::Every { seconds } if *seconds > 0 => {
            let steps = fired.saturating_sub(goal.at) / seconds + 1;
            let next = goal.at.saturating_add(steps.saturating_mul(*seconds));
            Some(next).filter(|due| *due <= goal.deadline)
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
        self.items.iter().find(|item| item.goal.id == id)
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
            .find(|marked| marked.operation == operation)
    }

    /// Whether `operation` names a firing or an event already kept.
    pub fn kept(&self, operation: &str) -> bool {
        self.events.iter().any(|event| event == operation)
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
            .filter(|item| item.standing == Standing::Open)
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
            .filter(|item| item.standing == Standing::Open)
            .flat_map(|item| item.timers.iter().filter_map(|timer| timer.next_due))
            .min()
    }

    /// Fold one leaf. A leaf that contradicts what came before is refused,
    /// since every kept leaf was checked against it.
    pub fn hold(&mut self, line: Line) -> Result<(), String> {
        match line {
            Line::Set(goal) => {
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
            .find(|sent| sent.operation == operation)
    }

    /// Every delivery still asked of a runner, with the text it types.
    pub fn unsettled(&self) -> Vec<(Sent, String)> {
        self.items
            .iter()
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
