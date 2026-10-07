//! Derived locations preserve the fold's first item and nested record order.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use super::{Change, Changed, Fired, HolderKind, Item, Remind, Sent, Standing};

pub(super) type Location = (usize, usize, usize);

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(super) struct Index {
    pub(super) items: HashMap<String, usize>,
    pub(super) compaction_agents: BTreeMap<String, BTreeSet<usize>>,
    pub(super) compaction_teams: BTreeMap<String, BTreeSet<usize>>,
    pub(super) marked: HashMap<String, usize>,
    pub(super) changed: HashMap<String, (usize, usize)>,
    pub(super) kept: HashSet<String>,
    pub(super) sent: HashMap<String, (usize, usize, usize)>,
    pub(super) firings: HashMap<String, (usize, usize)>,
    pub(super) prior_resends: HashMap<String, (usize, usize)>,
    pub(super) pending: BTreeMap<Location, String>,
    pub(super) pending_by_session: HashMap<String, BTreeSet<Location>>,
    pub(super) pending_by_goal: HashMap<usize, BTreeSet<Location>>,
    pub(super) current_words: HashMap<usize, usize>,
    pub(super) current_active: HashMap<usize, bool>,
}

fn first<T: Copy + Ord>(map: &mut HashMap<String, T>, operation: &str, location: T) {
    map.entry(operation.to_owned())
        .and_modify(|previous| {
            if location < *previous {
                *previous = location;
            }
        })
        .or_insert(location);
}

impl Index {
    pub(super) fn of(
        items: &[Item],
        events: &[String],
        resends: &BTreeMap<String, String>,
    ) -> Self {
        let mut index = Self::default();
        index.kept.extend(events.iter().cloned());
        for (position, item) in items.iter().enumerate() {
            index.items.entry(item.goal.id.clone()).or_insert(position);
            if let Some(marked) = &item.marked {
                first(&mut index.marked, &marked.operation, position);
            }
            for (change, changed) in item.changes.iter().enumerate() {
                index.change(position, change, changed);
            }
            for (firing, fired) in item.fired.iter().enumerate() {
                index.fire(position, firing, fired);
                if let Some(prior) = resends.get(&fired.operation) {
                    index
                        .prior_resends
                        .entry(prior.clone())
                        .or_insert((position, firing));
                }
            }
            index.compaction(position, item);
        }
        index
    }

    pub(super) fn compaction(&mut self, position: usize, item: &Item) {
        let held = match item.goal.holder.kind {
            HolderKind::Agent => &mut self.compaction_agents,
            HolderKind::Team => &mut self.compaction_teams,
        };
        if let Some(positions) = held.get_mut(&item.goal.holder.id) {
            positions.remove(&position);
            if positions.is_empty() {
                held.remove(&item.goal.holder.id);
            }
        }
        if item.standing == Standing::Open
            && self
                .current_active
                .get(&position)
                .copied()
                .unwrap_or(item.goal.active)
            && item.goal.reminders.iter().any(|remind| {
                matches!(
                    remind,
                    Remind::On {
                        event: super::Event::Compaction
                    }
                )
            })
        {
            held.entry(item.goal.holder.id.clone())
                .or_default()
                .insert(position);
        }
    }

    pub(super) fn mark(&mut self, position: usize, operation: &str) {
        first(&mut self.marked, operation, position);
    }

    pub(super) fn change(&mut self, position: usize, change: usize, changed: &Changed) {
        first(&mut self.changed, &changed.operation, (position, change));
        self.kept.insert(changed.operation.clone());
        match &changed.change {
            Change::Words { .. } => {
                self.current_words.insert(position, change);
            }
            Change::Active { active } => {
                self.current_active.insert(position, *active);
            }
        }
    }

    pub(super) fn fire(&mut self, position: usize, firing: usize, fired: &Fired) {
        first(&mut self.firings, &fired.operation, (position, firing));
        self.kept.insert(fired.operation.clone());
        for (delivery, sent) in fired.sent.iter().enumerate() {
            let location = (position, firing, delivery);
            let previous = self.sent.get(&sent.operation).copied();
            first(&mut self.sent, &sent.operation, location);
            if self.sent.get(&sent.operation) == Some(&location) {
                if let Some(previous) = previous.filter(|previous| *previous != location) {
                    self.finish(previous);
                }
                self.answer(location, sent);
            }
        }
    }

    pub(super) fn answer(&mut self, location: Location, sent: &Sent) {
        if sent.state.unsettled() {
            self.pending.insert(location, sent.session.clone());
            self.pending_by_session
                .entry(sent.session.clone())
                .or_default()
                .insert(location);
            self.pending_by_goal
                .entry(location.0)
                .or_default()
                .insert(location);
        } else {
            self.finish(location);
        }
    }

    fn finish(&mut self, location: Location) {
        let Some(session) = self.pending.remove(&location) else {
            return;
        };
        if let Some(held) = self.pending_by_session.get_mut(&session) {
            held.remove(&location);
            if held.is_empty() {
                self.pending_by_session.remove(&session);
            }
        }
        if let Some(held) = self.pending_by_goal.get_mut(&location.0) {
            held.remove(&location);
            if held.is_empty() {
                self.pending_by_goal.remove(&location.0);
            }
        }
    }
}
