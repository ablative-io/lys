//! Rights snapshots reused only while their authority and time boundaries stand.

use std::collections::{BTreeMap, VecDeque};

use lys_identity::{IdentityId, projection::Projection};
use lys_pass::{Holder, Right};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Read {
    pub(super) rights: Vec<Right>,
    pub(super) paths: BTreeMap<String, Vec<String>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Key {
    pub(super) holder: IdentityId,
    pub(super) app: String,
    pub(super) revision: u64,
    pub(super) bind: bool,
}

#[derive(Clone)]
pub(super) struct Cached {
    pub(super) asserted: Holder,
    pub(super) read: Read,
    pub(super) directory: Projection,
    pub(super) apps_revision: u64,
    pub(super) read_at: u64,
    pub(super) valid_until: Option<u64>,
}

// One entry per holder/app pair, enough for every person and agent we have.
const CAPACITY: usize = 512;

#[derive(Default)]
pub(crate) struct Cache {
    entries: VecDeque<(Key, Cached)>,
}

impl Cache {
    pub(super) fn get(
        &self,
        key: &Key,
        directory: &Projection,
        apps_revision: u64,
        at: u64,
    ) -> Option<Cached> {
        self.entries
            .iter()
            .find(|(held, value)| {
                held == key
                    && value.directory == *directory
                    && value.apps_revision == apps_revision
                    && at >= value.read_at
                    && value.valid_until.is_none_or(|end| at < end)
            })
            .map(|(_, value)| value.clone())
    }

    pub(super) fn replace(&mut self, key: Key, value: Cached) {
        self.entries
            .retain(|(held, _)| held.holder != key.holder || held.app != key.app);
        if self.entries.len() == CAPACITY {
            self.entries.pop_front();
        }
        self.entries.push_back((key, value));
    }
}

#[cfg(test)]
#[path = "rights_cache_tests.rs"]
mod tests;
