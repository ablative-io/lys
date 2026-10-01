//! Derived locations preserve the fold's first item and nested record order.

use std::collections::{HashMap, HashSet};

use super::{Fired, Item};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(super) struct Index {
    pub(super) items: HashMap<String, usize>,
    pub(super) marked: HashMap<String, usize>,
    pub(super) changed: HashMap<String, (usize, usize)>,
    pub(super) kept: HashSet<String>,
    pub(super) sent: HashMap<String, (usize, usize, usize)>,
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
    pub(super) fn of(items: &[Item], events: &[String]) -> Self {
        let mut index = Self::default();
        index.kept.extend(events.iter().cloned());
        for (position, item) in items.iter().enumerate() {
            index.items.entry(item.goal.id.clone()).or_insert(position);
            if let Some(marked) = &item.marked {
                first(&mut index.marked, &marked.operation, position);
            }
            for (change, changed) in item.changes.iter().enumerate() {
                index.change(position, change, &changed.operation);
            }
            for (firing, fired) in item.fired.iter().enumerate() {
                index.fire(position, firing, fired);
            }
        }
        index
    }

    pub(super) fn mark(&mut self, position: usize, operation: &str) {
        first(&mut self.marked, operation, position);
    }

    pub(super) fn change(&mut self, position: usize, change: usize, operation: &str) {
        first(&mut self.changed, operation, (position, change));
        self.kept.insert(operation.to_owned());
    }

    pub(super) fn fire(&mut self, position: usize, firing: usize, fired: &Fired) {
        self.kept.insert(fired.operation.clone());
        for (delivery, sent) in fired.sent.iter().enumerate() {
            first(
                &mut self.sent,
                &sent.operation,
                (position, firing, delivery),
            );
        }
    }
}
