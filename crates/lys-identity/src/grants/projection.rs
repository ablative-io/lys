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
use super::error::GrantError;
use super::events::{GrantChange, GrantEvent};
use super::lineage::{self, Lineage, check_hop};
use super::types::{Grant, GrantId, Source};
use crate::id::IdentityId;
use crate::operation::OperationId;

/// Where and why a grant was revoked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Revocation {
    /// The operation that revoked it.
    pub operation: OperationId,
    /// The index of the revoking event.
    pub index: u64,
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

    /// Its own revocation, if it was revoked directly.
    pub fn revoked(&self) -> Option<&Revocation> {
        self.revoked.as_ref()
    }
}

/// Every grant, the operations that made them, and the committed events refused.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GrantBook {
    records: BTreeMap<GrantId, GrantRecord>,
    children: BTreeMap<GrantId, BTreeSet<GrantId>>,
    operations: HashMap<OperationId, u64>,
    refused: BTreeMap<u64, (OperationId, GrantError)>,
}

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
        self.records.values()
    }

    /// Every grant `holder` holds, in id order.
    pub fn held_by(&self, holder: IdentityId) -> impl Iterator<Item = &GrantRecord> {
        self.records
            .values()
            .filter(move |record| record.grant.holder() == holder)
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
                if record.grant.holder() == event.caller() {
                    Ok(())
                } else {
                    Err(GrantError::EventMismatch {
                        reason: "a use is observed for the grant's own holder",
                    })
                }
            }
        }
    }

    /// Advance the book by `event`, committed at `index`.
    pub fn apply(&mut self, event: &GrantEvent, index: u64) -> Result<(), GrantError> {
        self.check(event)?;
        self.operations.insert(event.operation(), index);
        match event.change() {
            GrantChange::Issue(grant) => {
                if let Source::Grant(source) = grant.source() {
                    self.children.entry(source).or_default().insert(grant.id());
                }
                self.records.insert(
                    grant.id(),
                    GrantRecord {
                        grant: grant.as_ref().clone(),
                        index,
                        revoked: None,
                        last_use: LastUse::NotSeen,
                    },
                );
            }
            GrantChange::Revoke { grant, reason } => {
                if let Some(record) = self.records.get_mut(grant) {
                    record.revoked = Some(Revocation {
                        operation: event.operation(),
                        index,
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
                }
            }
        }
        Ok(())
    }

    /// Keep a committed event the book refused, by name, granting nothing.
    pub fn refuse(&mut self, event: &GrantEvent, index: u64, refusal: GrantError) {
        self.operations.entry(event.operation()).or_insert(index);
        self.refused.insert(index, (event.operation(), refusal));
    }
}
