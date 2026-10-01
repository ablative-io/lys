//! Accounting regressions count work on their own test thread.

use std::cell::Cell;

mod tests;

#[derive(Clone, Copy)]
pub(crate) enum Work {
    Usage,
    Running,
    StateCopy,
}

thread_local! {
    static COUNTS: Cell<[usize; 3]> = const { Cell::new([0; 3]) };
}

pub(crate) fn visit(work: Work) {
    COUNTS.with(|held| {
        let mut counts = held.get();
        counts[work as usize] += 1;
        held.set(counts);
    });
}

fn reset() {
    COUNTS.set([0; 3]);
}

fn count(work: Work) -> usize {
    COUNTS.get()[work as usize]
}
