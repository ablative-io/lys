//! [`open_with_snapshot`] opens a log from its owner's snapshot, or from the
//! whole log when the snapshot cannot be believed, and says which.
//!
//! A start reads the store's snapshot, checks it (see [`crate::snapshot`]),
//! refuses one claiming more leaves than the log has pinned, and resumes the
//! tree from its frontier through every leaf after it. The log must reach its
//! pinned root from there. Any refusal is named in the [`Start`] the owner is
//! handed, and the log is then opened from nothing, reading every leaf: a bad
//! snapshot costs one full start and is never used, and the owner is expected
//! to write a fresh one once it has folded the whole log.

use crate::error::StoreResult;
use crate::frontier::Frontier;
use crate::frontier_log::{FrontierLog, Reading, Tail};
use crate::snapshot::{SnapshotRefusal, unseal};
use crate::store::LeafStore;

/// How a start came by its state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Start {
    /// From the snapshot at `size`, replaying the `replayed` leaves after it.
    Resumed {
        /// The tree size of the snapshot.
        size: u64,
        /// How many leaves were read after it.
        replayed: u64,
    },
    /// From the whole log, reading `replayed` leaves, because the snapshot was
    /// refused.
    Rebuilt {
        /// Why the snapshot was not used.
        refusal: SnapshotRefusal,
        /// How many leaves were read.
        replayed: u64,
    },
}

impl Start {
    /// The refusal that sent the start to the whole log, if one did.
    pub fn refusal(&self) -> Option<&SnapshotRefusal> {
        match self {
            Self::Resumed { .. } => None,
            Self::Rebuilt { refusal, .. } => Some(refusal),
        }
    }
}

impl std::fmt::Display for Start {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Resumed { size, replayed } => write!(
                f,
                "resumed from the snapshot at tree size {size}, replaying {replayed} leaves after it"
            ),
            Self::Rebuilt { refusal, replayed } => write!(
                f,
                "rebuilt from the whole log, replaying {replayed} leaves, because the snapshot was refused: {refusal}"
            ),
        }
    }
}

/// What a start opened: the log, the snapshot's state when it was used, the
/// leaves the owner must still apply, and how the start went.
#[derive(Debug)]
pub struct Started<S: LeafStore> {
    /// The log, reconciled with its pin.
    pub log: FrontierLog<S>,
    /// The owner's state from the snapshot, or `None` when rebuilding.
    pub state: Option<Vec<u8>>,
    /// The leaves after the state: after the snapshot, or every leaf.
    pub tail: Tail,
    /// Which way the start went, and why.
    pub start: Start,
}

/// Opens the log over `store` from its snapshot of `domain`, signed by
/// `public_key`, or from the whole log when the snapshot is refused.
///
/// # Errors
///
/// Whatever the store returns while reading, and [`FrontierLog::open`]'s
/// refusals when even the whole log does not reconcile with its pin. A
/// snapshot that fails a check is never an error: it is refused by name in
/// [`Started::start`].
pub fn open_with_snapshot<S: LeafStore>(
    store: S,
    domain: &str,
    public_key: &[u8; 32],
) -> StoreResult<Started<S>> {
    let refusal = match store.snapshot()? {
        None => SnapshotRefusal::Missing,
        Some(sealed) => match unseal(&sealed, domain, store.origin(), public_key) {
            Err(refusal) => refusal,
            Ok(snapshot) => {
                let pinned = store.pinned().tree_size;
                let size = snapshot.frontier().size();
                if size > pinned {
                    SnapshotRefusal::BeyondLog { size, pinned }
                } else {
                    let (frontier, state) = snapshot.into_parts();
                    let reading = Reading::from(&store, frontier)?;
                    if reading
                        .reconcile(store.pinned(), store.batch_intent())
                        .is_ok()
                    {
                        let replayed = reading.replayed();
                        let (log, tail) = FrontierLog::from_reading(store, reading)?;
                        return Ok(Started {
                            log,
                            state: Some(state),
                            tail,
                            start: Start::Resumed { size, replayed },
                        });
                    }
                    SnapshotRefusal::WrongRoot { size }
                }
            }
        },
    };
    let reading = Reading::from(&store, Frontier::new())?;
    let replayed = reading.replayed();
    let (log, tail) = FrontierLog::from_reading(store, reading)?;
    Ok(Started {
        log,
        state: None,
        tail,
        start: Start::Rebuilt { refusal, replayed },
    })
}

#[cfg(test)]
#[path = "start_tests.rs"]
mod tests;
