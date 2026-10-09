//! The book's live indexes (ACCESS-006 R5): each holder's live grants and
//! the live grants on each resource, so a membership page or an admission
//! reads only the authority that currently stands, never the retired
//! history.
//!
//! A grant is live while it and every grant it derives from stand
//! unrevoked and unspent. The indexes are derived state: an issue adds a
//! live grant, a revocation or a one-time grant's spending prunes that grant
//! and everything derived from it, and decoding the snapshot rebuilds them,
//! so a book restored from its snapshot and tail holds exactly the indexes a
//! book that applied every event holds. When a holder's or a resource's last
//! live grant goes, its entry goes with it. Expiry is not pruned here: a
//! window is judged at the moment a decision is made, by that decision.

use std::collections::{BTreeMap, BTreeSet};

use super::{GrantBook, GrantRecord};
use crate::grants::types::{GrantId, Resource, Source};
use crate::id::IdentityId;

/// The live grants, by holder and by resource.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Live {
    by_holder: BTreeMap<IdentityId, BTreeSet<GrantId>>,
    by_resource: BTreeMap<Resource, BTreeSet<GrantId>>,
}

fn remove<K: Ord>(index: &mut BTreeMap<K, BTreeSet<GrantId>>, key: &K, id: GrantId) {
    if let Some(ids) = index.get_mut(key) {
        ids.remove(&id);
        if ids.is_empty() {
            index.remove(key);
        }
    }
}

fn entries<K>(index: &BTreeMap<K, BTreeSet<GrantId>>) -> usize {
    index.values().map(BTreeSet::len).sum()
}

impl GrantBook {
    /// Whether `id` and every grant it derives from stand unrevoked. A
    /// grant whose source the book does not hold does not stand.
    fn stands(&self, id: GrantId) -> bool {
        let mut seen = BTreeSet::new();
        let mut next = Some(id);
        while let Some(current) = next {
            if !seen.insert(current) {
                return false;
            }
            let Some(record) = self.records.get(&current) else {
                return false;
            };
            if record.revoked.is_some() {
                return false;
            }
            next = match record.grant.source() {
                Source::Grant(source) => Some(source),
                Source::Root => None,
            };
        }
        true
    }

    /// Index `id` as live when it stands.
    pub(crate) fn index_live(&mut self, id: GrantId) {
        if !self.stands(id) {
            return;
        }
        let Some((holder, resource)) = self
            .records
            .get(&id)
            .map(|record| (record.grant.holder(), record.grant.resource().clone()))
        else {
            return;
        };
        self.live.by_holder.entry(holder).or_default().insert(id);
        self.live
            .by_resource
            .entry(resource)
            .or_default()
            .insert(id);
    }

    /// Prune `id` and every grant derived from it from the live indexes,
    /// dropping a holder's or a resource's entry with its last live grant.
    pub(crate) fn prune_live(&mut self, id: GrantId) {
        let mut gone = self.descendants(id);
        gone.push(id);
        for grant in gone {
            let Some((holder, resource)) = self
                .records
                .get(&grant)
                .map(|record| (record.grant.holder(), record.grant.resource().clone()))
            else {
                continue;
            };
            remove(&mut self.live.by_holder, &holder, grant);
            remove(&mut self.live.by_resource, &resource, grant);
        }
    }

    /// Rebuild the live indexes from the records, as a decoded snapshot
    /// must.
    pub(crate) fn rebuild_live(&mut self) {
        self.live = Live::default();
        let ids: Vec<GrantId> = self.records.keys().copied().collect();
        for id in ids {
            self.index_live(id);
        }
    }

    /// Every live grant `holder` holds, in id order, read from the live
    /// index: a revoked, spent or orphaned grant is never visited.
    pub fn live_held_by(&self, holder: IdentityId) -> impl Iterator<Item = &GrantRecord> {
        self.live
            .by_holder
            .get(&holder)
            .into_iter()
            .flatten()
            .filter_map(|id| self.records.get(id))
    }

    /// Every live grant on `resource`, in id order, read from the live index.
    pub fn live_on_resource<'a>(
        &'a self,
        resource: &Resource,
    ) -> impl Iterator<Item = &'a GrantRecord> {
        self.live
            .by_resource
            .get(resource)
            .into_iter()
            .flatten()
            .filter_map(|id| self.records.get(id))
    }

    /// How many entries the live indexes keep: (holder, grant) pairs and
    /// (resource, grant) pairs. Both return to their earlier value when
    /// the last live chain a revocation reaches is pruned.
    pub fn live_entries(&self) -> (usize, usize) {
        (
            entries(&self.live.by_holder),
            entries(&self.live.by_resource),
        )
    }
}

#[cfg(test)]
#[path = "projection_live_tests.rs"]
mod tests;
