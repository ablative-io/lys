#![cfg(test)]
//! A per-thread count of the leaves this crate has hashed, so a test can say
//! how often a leaf was hashed rather than infer it from timing. Every leaf
//! hash goes through [`hash_leaf`](crate::frontier::hash_leaf), which counts
//! here; tests run on their own threads, so one test's count is its own.

use std::cell::Cell;

thread_local! {
    static LEAF_HASHES: Cell<u64> = const { Cell::new(0) };
}

/// Records that one leaf was hashed on this thread.
pub(crate) fn count_leaf_hash() {
    LEAF_HASHES.with(|count| count.set(count.get() + 1));
}

/// How many leaves this thread has hashed.
pub(crate) fn leaf_hashes() -> u64 {
    LEAF_HASHES.with(Cell::get)
}
