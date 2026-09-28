//! What a session read from its file and how many syncs its own writes made,
//! and the block store's syncs beside them (HOME-020 R1).
//!
//! The counts prove cost bounds without a clock: a bound on the entries an
//! ingest reads, or the syncs an import makes, is a number a test reads.
//! Each count is held by the value that owns it, in an atomic, so a read
//! through `&self` counts and the owner stays `Send` and `Sync`; nothing is
//! counted process-wide. Counting changes no read, no write, no sync and no
//! order.

use std::sync::atomic::{AtomicU64, Ordering};

use crate::record::session::Session;

/// A session's counts at one moment.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct IoCounts {
    /// Entries deserialised from the session file.
    pub entries_read: u64,
    /// `sync_all` calls on the session file, its index and its head, and
    /// syncs of the sessions directory, made by the session's own writes.
    pub syncs: u64,
}

impl Session {
    /// The entries this session has deserialised from its file and the
    /// syncs its own writes have made: zero for a session just opened from a
    /// current index and head; what `create` made for a new one.
    #[must_use]
    pub fn io_counts(&self) -> IoCounts {
        self.io.counts()
    }
}

/// The live counts one session holds.
#[derive(Debug, Default)]
pub(crate) struct IoCounter {
    entries_read: AtomicU64,
    syncs: SyncCount,
}

impl IoCounter {
    /// Count `n` entries read.
    pub(crate) fn read(&self, n: usize) {
        let n = u64::try_from(n).unwrap_or(u64::MAX);
        self.entries_read.fetch_add(n, Ordering::Relaxed);
    }

    /// Count one sync.
    pub(crate) fn synced(&self) {
        self.syncs.add();
    }

    /// The counts as they stand.
    pub(crate) fn counts(&self) -> IoCounts {
        IoCounts {
            entries_read: self.entries_read.load(Ordering::Relaxed),
            syncs: self.syncs.get(),
        }
    }
}

/// A count of syncs. A clone starts at the count its source had, so the
/// clone and its source count their own syncs from then on.
#[derive(Debug, Default)]
pub(crate) struct SyncCount(AtomicU64);

impl SyncCount {
    /// Count one sync.
    pub(crate) fn add(&self) {
        self.0.fetch_add(1, Ordering::Relaxed);
    }

    /// The count as it stands.
    pub(crate) fn get(&self) -> u64 {
        self.0.load(Ordering::Relaxed)
    }
}

impl Clone for SyncCount {
    fn clone(&self) -> Self {
        Self(AtomicU64::new(self.get()))
    }
}
