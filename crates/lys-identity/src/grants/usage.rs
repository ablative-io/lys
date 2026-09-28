//! What the grants can say about a grant's use.
//!
//! A use is reported by the enforcement point, [`Grants::check`], as a use
//! event in the grant log. The book folds those events into the latest
//! recorded use and a count of recorded uses, so both survive a reopen.
//!
//! A permitted exercise whose use event could not be recorded has no report
//! in the log. It is named here as unreported, never folded into the count:
//! a count of zero with an unreported exercise is a missing report, not no
//! use. Nothing in the log carries it, so it is known only to the grants that
//! saw the record fail, and a reopen answers from the log alone.

use std::collections::BTreeMap;

use lys_log_store::LeafStore;

use super::admission::Route;
use super::authority::Grants;
use super::error::GrantError;
use super::permission::RelationshipStore;
use super::projection::{GrantRecord, LastUse};
use super::types::GrantId;

/// Permitted exercises of one grant whose use events could not be recorded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unreported {
    /// How many.
    pub count: u64,
    /// When the latest was permitted, in seconds since the Unix epoch.
    pub at: u64,
    /// How the latest arrived.
    pub route: Route,
    /// Why its use event was not recorded.
    pub reason: String,
}

/// A grant's use: what the log records, and what it is missing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Usage {
    /// The latest recorded use.
    pub last: LastUse,
    /// How many uses the log records.
    pub recorded: u64,
    /// Permitted exercises the log has no report of, if any.
    pub unreported: Option<Unreported>,
}

/// Note that an exercise of `grant` through `route` at `at` was permitted and
/// its use event refused with `error`.
pub(super) fn note(
    unreported: &mut BTreeMap<GrantId, Unreported>,
    grant: GrantId,
    route: Route,
    at: u64,
    error: &GrantError,
) {
    let entry = unreported.entry(grant).or_insert_with(|| Unreported {
        count: 0,
        at,
        route,
        reason: String::new(),
    });
    entry.count = entry.count.saturating_add(1);
    entry.at = at;
    entry.route = route;
    entry.reason = error.to_string();
}

impl Usage {
    /// The use of the grant `record` holds, with the exercises of it the log
    /// has no report of.
    pub fn of(record: &GrantRecord, unreported: Option<&Unreported>) -> Self {
        Self {
            last: record.last_use(),
            recorded: record.uses(),
            unreported: unreported.cloned(),
        }
    }
}

impl<S: LeafStore, R: RelationshipStore> Grants<S, R> {
    /// The permitted exercises of `grant` whose use events could not be
    /// recorded, if any.
    pub fn unreported(&self, grant: GrantId) -> Option<&Unreported> {
        self.unreported.get(&grant)
    }

    /// The use of `grant`, or None if the book does not hold it.
    pub fn usage(&self, grant: GrantId) -> Option<Usage> {
        let record = self.book.record(grant)?;
        Some(Usage::of(record, self.unreported(grant)))
    }
}
