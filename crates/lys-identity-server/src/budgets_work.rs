//! Accounting regressions count work on their own test thread.

use std::cell::Cell;

mod crossings;
mod members;
mod tests;

#[derive(Clone, Copy)]
pub(crate) enum Work {
    Usage,
    Running,
    StateCopy,
    IndexCopy,
    Crossing,
    CrossingLookup,
    Team,
}

thread_local! {
    static COUNTS: Cell<[usize; 7]> = const { Cell::new([0; 7]) };
}

pub(crate) fn visit(work: Work) {
    COUNTS.with(|held| {
        let mut counts = held.get();
        counts[work as usize] += 1;
        held.set(counts);
    });
}

fn reset() {
    COUNTS.set([0; 7]);
}

fn count(work: Work) -> usize {
    COUNTS.get()[work as usize]
}
