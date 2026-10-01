//! Lookup regressions count record visits on their own test thread.

use std::cell::Cell;

mod tests;

#[derive(Clone, Copy)]
pub(crate) enum Work {
    Stop,
    Review,
    Account,
    Team,
    Refusal,
    TeamOperation,
    RuntimeOperation,
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

pub(crate) fn reset() {
    COUNTS.set([0; 7]);
}

pub(crate) fn count(work: Work) -> usize {
    COUNTS.get()[work as usize]
}
