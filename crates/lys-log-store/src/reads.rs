//! The physical reads a file store makes, counted where it reads (ACCESS-006
//! R5, R6): every leaf read and every snapshot read through
//! [`LeafStore`](crate::LeafStore) on a [`FileLeafStore`](crate::FileLeafStore)
//! that goes to its files. A leaf asked beyond the extent is answered from
//! memory and is not a read. The reads made while a store opens (its layout,
//! offsets and tail) are not counted here.
//!
//! The count is process-wide and always kept: one relaxed atomic add for each
//! read, with no lock and no timer, so a caller measures a piece of work by
//! the difference across it. It counts every store in the process, so a
//! difference is that work's alone only while nothing else reads.

use std::sync::atomic::{AtomicU64, Ordering};

static PROCESS_READS: AtomicU64 = AtomicU64::new(0);

/// How many physical leaf and snapshot reads every file store in this
/// process has made.
pub fn process_read_count() -> u64 {
    PROCESS_READS.load(Ordering::Relaxed)
}

/// Count one physical read.
pub(crate) fn count_read() {
    PROCESS_READS.fetch_add(1, Ordering::Relaxed);
}
