//! The grant book: every grant as its committed events say it is.
//!
//! The book is rebuilt from the grant events when the grants open, and
//! advanced by each event as it is committed. Both paths go through
//! [`GrantBook::apply`]. [`GrantBook::check`] answers whether an event would
//! apply without applying it, so a change is refused by name before it is
//! committed. A committed event the book refuses, such as one ending later
//! than its source, is kept as a named refusal and grants nothing.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use super::admission::Route;
use super::authority::ONE_TIME_SPENT;
use super::error::GrantError;
use super::events::{GrantChange, GrantEvent};
use super::lineage::{self, Lineage, check_hop};
use super::schema::owner_of;
use super::types::{Grant, GrantId, Resource, Source};
use crate::id::IdentityId;
use crate::operation::OperationId;

/// The rule a use event observed for anyone but the grant's holder breaks.
pub(crate) const USE_BY_HOLDER: &str = "a use is observed for the grant's own holder";

/// Where and why a grant was revoked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Revocation {
    /// The operation that revoked it.
    pub operation: OperationId,
    /// The index of the revoking event.
    pub index: u64,
    /// When the revoking event was recorded, in seconds since the epoch.
    pub at: u64,
    /// The reason given.
    pub reason: String,
}

/// When a grant was last seen exercised, as the use events in the grant log
/// record it. A grant with no use event is not seen: that says only that no
/// exercise was observed, never that it was never used.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LastUse {
    /// No exercise of the grant has been observed.
    NotSeen,
    /// The latest observed exercise.
    Seen {
        /// When it was exercised, in seconds since the Unix epoch.
        at: u64,
        /// How the exercise arrived.
        route: Route,
        /// The index of the use event.
        index: u64,
    },
}

/// One grant, as the book holds it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrantRecord {
    grant: Grant,
    index: u64,
    revoked: Option<Revocation>,
    last_use: LastUse,
    uses: u64,
}

impl GrantRecord {
    /// The grant.
    pub fn grant(&self) -> &Grant {
        &self.grant
    }

    /// The index of the event that issued it.
    pub fn index(&self) -> u64 {
        self.index
    }

    /// When it was last seen exercised.
    pub fn last_use(&self) -> LastUse {
        self.last_use
    }

    /// How many use events the grant log records for it. Zero is a count of
    /// recorded uses, not a claim that it was never exercised: an exercise
    /// whose use event could not be recorded is not in this count, and is
    /// named by [`Usage::unreported`](super::usage::Usage::unreported).
    pub fn uses(&self) -> u64 {
        self.uses
    }

    /// Its own revocation, if it was revoked directly.
    pub fn revoked(&self) -> Option<&Revocation> {
        self.revoked.as_ref()
    }
}

/// What the event at one index did to a grant, as the book records it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Changed {
    /// The grant was issued.
    Issued(GrantId),
    /// The grant was revoked.
    Revoked(GrantId),
}

/// Every grant, the operations that made them, and the committed events refused.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GrantBook {
    records: BTreeMap<GrantId, GrantRecord>,
    children: BTreeMap<GrantId, BTreeSet<GrantId>>,
    by_resource: BTreeMap<Resource, BTreeSet<GrantId>>,
    /// Each holder's grants, derived on apply and on decode, never stored.
    by_holder: BTreeMap<IdentityId, BTreeSet<GrantId>>,
    by_kind: BTreeMap<(String, String), BTreeSet<GrantId>>,
    operations: HashMap<OperationId, u64>,
    refused: BTreeMap<u64, (OperationId, GrantError)>,
}

#[cfg(any(test, feature = "test-support"))]
thread_local! { static RECORD_VISITS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) }; }

impl GrantBook {
    /// An empty book.
    pub fn new() -> Self {
        Self::default()
    }

    /// The grant `id`, if the book holds it.
    pub fn record(&self, id: GrantId) -> Option<&GrantRecord> {
        self.records.get(&id)
    }

    /// The grant `id`, if the book holds it.
    pub fn grant(&self, id: GrantId) -> Option<&Grant> {
        self.records.get(&id).map(GrantRecord::grant)
    }

    /// Every grant, in id order.
    pub fn records(&self) -> impl Iterator<Item = &GrantRecord> {
        let records = self.records.values();
        #[cfg(any(test, feature = "test-support"))]
        let records =
            records.inspect(|_| RECORD_VISITS.with(|visits| visits.set(visits.get() + 1)));
        records
    }

    /// The whole-book records visited by this thread, for cost regressions.
    #[cfg(any(test, feature = "test-support"))]
    pub fn record_visits() -> usize {
        RECORD_VISITS.with(std::cell::Cell::get)
    }

    /// Grants belonging to `app`, selected by kind then by grant id.
    pub fn in_app<'a>(&'a self, app: &'a str) -> impl Iterator<Item = &'a GrantRecord> {
        self.by_kind
            .range((app.to_owned(), String::new())..)
            .take_while(move |((owner, _), _)| owner == app)
            .flat_map(|(_, ids)| ids)
            .filter_map(|id| self.records.get(id))
    }

    /// Index one issued grant by holder and by kind, without walking earlier
    /// grants.
    pub(crate) fn index_kind(&mut self, grant: &Grant) {
        self.by_holder
            .entry(grant.holder())
            .or_default()
            .insert(grant.id());
        let kind = grant.resource().kind();
        self.by_kind
            .entry((owner_of(kind).to_owned(), kind.to_owned()))
            .or_default()
            .insert(grant.id());
    }

    /// Every grant `holder` holds, in id order, read from the holder index
    /// rather than the whole book (ACCESS-006 R3).
    pub fn held_by(&self, holder: IdentityId) -> impl Iterator<Item = &GrantRecord> {
        self.by_holder
            .get(&holder)
            .into_iter()
            .flatten()
            .filter_map(|id| self.records.get(id))
    }

    /// Every grant on `resource`, in id order.
    pub fn on_resource<'a>(&'a self, resource: &Resource) -> impl Iterator<Item = &'a GrantRecord> {
        self.by_resource
            .get(resource)
            .into_iter()
            .flatten()
            .filter_map(|id| self.records.get(id))
    }

    /// Every grant derived from `id`, directly or through others, in the order reached.
    pub fn descendants(&self, id: GrantId) -> Vec<GrantId> {
        let mut found = Vec::new();
        let mut next = vec![id];
        while let Some(current) = next.pop() {
            for child in self.children.get(&current).into_iter().flatten() {
                found.push(*child);
                next.push(*child);
            }
        }
        found
    }

    /// The checked ancestry of `id`.
    pub fn lineage(&self, id: GrantId) -> Result<Lineage, GrantError> {
        lineage::resolve(id, |grant| self.grant(grant))
    }

    /// The index of the event an operation made, whether it applied or was refused.
    pub fn operation(&self, operation: OperationId) -> Option<u64> {
        self.operations.get(&operation).copied()
    }

    /// The committed events the book refused, by index.
    pub fn refused(&self) -> &BTreeMap<u64, (OperationId, GrantError)> {
        &self.refused
    }

    /// Whether `event` would apply to the book as it stands, refused by name if not.
    pub fn check(&self, event: &GrantEvent) -> Result<(), GrantError> {
        if self.operations.contains_key(&event.operation()) {
            return Err(GrantError::OperationReused {
                operation: event.operation().to_string(),
            });
        }
        match event.change() {
            GrantChange::Issue(grant) => {
                if self.records.contains_key(&grant.id()) {
                    return Err(GrantError::GrantExists {
                        grant: grant.id().to_string(),
                    });
                }
                match grant.source() {
                    Source::Root => Ok(()),
                    Source::Grant(source) => {
                        let source =
                            self.grant(source)
                                .ok_or_else(|| GrantError::SourceUnknown {
                                    grant: source.to_string(),
                                })?;
                        check_hop(grant, source)
                    }
                }
            }
            GrantChange::Revoke { grant, .. } => {
                let record = self
                    .records
                    .get(grant)
                    .ok_or_else(|| GrantError::GrantUnknown {
                        grant: grant.to_string(),
                    })?;
                match record.revoked {
                    Some(_) => Err(GrantError::AlreadyRevoked {
                        grant: grant.to_string(),
                    }),
                    None => Ok(()),
                }
            }
            GrantChange::Use { grant, .. } => {
                let record = self
                    .records
                    .get(grant)
                    .ok_or_else(|| GrantError::GrantUnknown {
                        grant: grant.to_string(),
                    })?;
                if record.grant.holder() != event.caller() {
                    return Err(GrantError::EventMismatch {
                        reason: USE_BY_HOLDER,
                    });
                }
                if record.grant.is_once() && record.revoked.is_some() {
                    return Err(GrantError::AlreadyRevoked {
                        grant: grant.to_string(),
                    });
                }
                Ok(())
            }
        }
    }

    /// Advance the book by `event`, committed at `index`.
    pub fn apply(&mut self, event: &GrantEvent, index: u64) -> Result<(), GrantError> {
        self.check(event)?;
        self.operations.insert(event.operation(), index);
        match event.change() {
            GrantChange::Issue(grant) => {
                self.index_kind(grant);
                if let Source::Grant(source) = grant.source() {
                    self.children.entry(source).or_default().insert(grant.id());
                }
                self.by_resource
                    .entry(grant.resource().clone())
                    .or_default()
                    .insert(grant.id());
                self.records.insert(
                    grant.id(),
                    GrantRecord {
                        grant: grant.as_ref().clone(),
                        index,
                        revoked: None,
                        last_use: LastUse::NotSeen,
                        uses: 0,
                    },
                );
            }
            GrantChange::Revoke { grant, reason } => {
                if let Some(record) = self.records.get_mut(grant) {
                    record.revoked = Some(Revocation {
                        operation: event.operation(),
                        index,
                        at: event.recorded_at(),
                        reason: reason.clone(),
                    });
                }
            }
            GrantChange::Use { grant, route } => {
                if let Some(record) = self.records.get_mut(grant) {
                    record.last_use = LastUse::Seen {
                        at: event.recorded_at(),
                        route: *route,
                        index,
                    };
                    record.uses = record.uses.saturating_add(1);
                    // A one-time grant is spent by the leaf that records its
                    // use, so no second append is needed to end it.
                    if record.grant.is_once() && record.revoked.is_none() {
                        record.revoked = Some(Revocation {
                            operation: event.operation(),
                            index,
                            at: event.recorded_at(),
                            reason: ONE_TIME_SPENT.to_owned(),
                        });
                    }
                }
            }
        }
        Ok(())
    }

    /// The changes to relationships the book records at or after `from`: the
    /// index of each grant's issue and of each revocation, by index. An index
    /// not named is a use or a refused event, and changes no relationship.
    pub(crate) fn changes_from(&self, from: u64) -> BTreeMap<u64, Changed> {
        let mut changes = BTreeMap::new();
        for record in self.records.values() {
            let id = record.grant.id();
            if record.index >= from {
                changes.insert(record.index, Changed::Issued(id));
            }
            if let Some(revocation) = &record.revoked
                && revocation.index >= from
            {
                changes.insert(revocation.index, Changed::Revoked(id));
            }
        }
        changes
    }

    /// Keep a committed event the book refused, by name, granting nothing.
    pub fn refuse(&mut self, event: &GrantEvent, index: u64, refusal: GrantError) {
        self.operations.entry(event.operation()).or_insert(index);
        self.refused.insert(index, (event.operation(), refusal));
    }
}

#[path = "book_state.rs"]
pub(crate) mod state;

#[cfg(test)]
#[path = "projection_index_tests.rs"]
mod index_tests;
