#![cfg(test)]
//! Work counters belong to each test thread and never enter a served build.

use std::cell::Cell;

#[path = "budgets_work_accounting.rs"]
mod accounting;
#[path = "budgets_work_members.rs"]
mod members;
#[path = "budgets_work_state.rs"]
mod state;

#[derive(Clone, Copy)]
pub(crate) enum Work {
    Usage,
    Running,
    Team,
    Parent,
    Target,
    Settings,
    CrossingLookup,
    Pending,
    StateCopy,
}

thread_local! {
    static COUNTS: Cell<[usize; 9]> = const { Cell::new([0; 9]) };
}

pub(crate) fn visit(work: Work) {
    COUNTS.with(|held| {
        let mut counts = held.get();
        counts[work as usize] += 1;
        held.set(counts);
    });
}

fn reset() {
    COUNTS.set([0; 9]);
}

fn count(work: Work) -> usize {
    COUNTS.get()[work as usize]
}
