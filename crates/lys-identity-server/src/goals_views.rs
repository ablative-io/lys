//! Goal responses include current activity and a nullable deadline without rewriting stored sets.

use serde::Serialize;

use crate::goals_state::{Fired, Goal, Item, Marked, Timer};
use crate::goals_types::{EvidenceKind, Holder, Kind, Remind, Standing};

/// The current aim, including fields omitted from old stored entries.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[schema(as = GoalSet)]
pub struct GoalView {
    /// The operation id that set it.
    pub id: String,
    /// Its agent or team.
    pub holder: Holder,
    /// What kind of item it is.
    pub kind: Kind,
    /// Its current words.
    pub words: String,
    /// Its deadline in seconds since the Unix epoch, or null.
    pub deadline: Option<u64>,
    /// Whether reminders are active.
    pub active: bool,
    /// What proves a deliverable was handed over.
    pub evidence: Option<EvidenceKind>,
    /// The action whose grant holders may judge it.
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

impl From<Goal> for GoalView {
    fn from(goal: Goal) -> Self {
        Self {
            id: goal.id,
            holder: goal.holder,
            kind: goal.kind,
            words: goal.words,
            deadline: goal.deadline,
            active: goal.active,
            evidence: goal.evidence,
            judged_by: goal.judged_by,
            reminders: goal.reminders,
            responsible: goal.responsible,
            set_by: goal.set_by,
            at: goal.at,
        }
    }
}

/// An item with its current aim and unchanged judgement and reminder history.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[schema(as = GoalItem)]
pub struct ItemView {
    /// Its current aim.
    pub goal: GoalView,
    /// Where its judgement stands.
    pub standing: Standing,
    /// The judgement that closed it.
    pub marked: Option<Marked>,
    /// Its reminder timers.
    pub timers: Vec<Timer>,
    /// Its reminder history.
    pub fired: Vec<Fired>,
}

impl From<Item> for ItemView {
    fn from(item: Item) -> Self {
        let active = item.active();
        let words = item
            .changes
            .into_iter()
            .rev()
            .find_map(|changed| match changed.change {
                crate::goals_types::Change::Words { words } => Some(words),
                crate::goals_types::Change::Active { .. } => None,
            });
        let mut goal = GoalView::from(item.goal);
        goal.active = active;
        if let Some(words) = words {
            goal.words = words;
        }
        Self {
            goal,
            standing: item.standing,
            marked: item.marked,
            timers: item.timers,
            fired: item.fired,
        }
    }
}

/// The items held on an agent or team.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct GoalsView {
    /// Every item, in the order set.
    pub goals: Vec<ItemView>,
}
