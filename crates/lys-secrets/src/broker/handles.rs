//! The handle records, with the indexes kept beside them so a presented
//! token, a handle's children and a person's ending are each found by a
//! lookup instead of by walking every handle ever issued.
//!
//! Invariants. Every index is changed only through the methods here, so it
//! names exactly the records the map holds. A record's token digest is
//! decoded from its hex once, when the record enters; a digest that does not
//! decode is never found, as a scan never found it. The token index is keyed
//! by the SHA-256 of a 32-byte random token, so a lookup's timing tells
//! nothing of any other token. Whoever sets a record's ending does it through
//! [`Handles::end`] or [`Handles::relay`], which keep the endings index.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::ops::Deref;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::encoding::unhex;

use super::{Ended, HandleRecord};

/// The records by id.
pub(super) type Records = BTreeMap<String, HandleRecord>;

/// A person and the operation id an ending was carried under.
type EndingKey = (String, String);

/// The handle records and their indexes.
#[derive(Debug, Clone, Default)]
pub(super) struct Handles {
    records: Records,
    /// Each token digest to the id of the handle it opens.
    by_digest: HashMap<[u8; 32], String>,
    /// Each handle to the handles derived from it.
    children: BTreeMap<String, BTreeSet<String>>,
    /// Each (person, operation) to every handle ended under it.
    endings: BTreeMap<EndingKey, BTreeSet<String>>,
    /// Each (person, operation) to the handles ended under it that are their
    /// own ending's root.
    roots: BTreeMap<EndingKey, BTreeSet<String>>,
}

/// The work lookups did, counted so a test can prove each finds what it
/// asks for without walking what it does not.
#[derive(Debug, Default)]
pub(super) struct Work {
    /// Searches for a presented token.
    pub(super) searches: AtomicU64,
    /// Handle records whose digest was compared with a presented one.
    pub(super) compared: AtomicU64,
    /// Handle records visited to answer a lineage or ending question.
    pub(super) visited: AtomicU64,
}

impl Work {
    /// Counts `count` more on `counter`.
    pub(super) fn count(counter: &AtomicU64, count: u64) {
        counter.fetch_add(count, Ordering::Relaxed);
    }
}

impl Deref for Handles {
    type Target = Records;

    fn deref(&self) -> &Records {
        &self.records
    }
}

impl FromIterator<HandleRecord> for Handles {
    fn from_iter<I: IntoIterator<Item = HandleRecord>>(records: I) -> Self {
        let mut handles = Self::default();
        for record in records {
            handles.insert(record);
        }
        handles
    }
}

/// The digest `text` spells, when it spells 32 bytes.
fn decoded(text: &str) -> Option<[u8; 32]> {
    unhex(text).and_then(|bytes| bytes.try_into().ok())
}

impl Handles {
    /// Adds `record`, replacing any record of its id, and indexes it.
    pub(super) fn insert(&mut self, mut record: HandleRecord) {
        if let Some(old) = self.records.remove(&record.id) {
            self.forget(&old);
        }
        record.held = decoded(&record.digest);
        if let Some(digest) = record.held {
            self.by_digest.insert(digest, record.id.clone());
        }
        if let Some(parent) = &record.parent {
            self.children
                .entry(parent.clone())
                .or_default()
                .insert(record.id.clone());
        }
        if let Some(ended) = &record.ended {
            self.index_ending(&record.id, ended);
        }
        self.records.insert(record.id.clone(), record);
    }

    /// Takes `record` out of every index.
    fn forget(&mut self, record: &HandleRecord) {
        if let Some(digest) = record.held {
            self.by_digest.remove(&digest);
        }
        if let Some(parent) = &record.parent {
            if let Some(children) = self.children.get_mut(parent) {
                children.remove(&record.id);
            }
        }
        if let Some(ended) = &record.ended {
            self.unindex_ending(&record.id, ended);
        }
    }

    fn index_ending(&mut self, id: &str, ended: &Ended) {
        let key = (ended.by.clone(), ended.operation.clone());
        if ended.root == id {
            self.roots
                .entry(key.clone())
                .or_default()
                .insert(id.to_owned());
        }
        self.endings.entry(key).or_default().insert(id.to_owned());
    }

    fn unindex_ending(&mut self, id: &str, ended: &Ended) {
        let key = (ended.by.clone(), ended.operation.clone());
        for index in [&mut self.endings, &mut self.roots] {
            if let Some(ids) = index.get_mut(&key) {
                ids.remove(id);
            }
        }
    }

    /// The record `id`, to change its counts. Its ending is set through
    /// [`Handles::end`], never through this.
    pub(super) fn get_mut(&mut self, id: &str) -> Option<&mut HandleRecord> {
        self.records.get_mut(id)
    }

    /// The record whose token digest is `digest`.
    pub(super) fn by_digest(&self, digest: &[u8; 32]) -> Option<&HandleRecord> {
        self.by_digest
            .get(digest)
            .and_then(|id| self.records.get(id))
    }

    /// The ids of the handles derived from `id`, in order of id.
    pub(super) fn children(&self, id: &str) -> Vec<&str> {
        self.children
            .get(id)
            .into_iter()
            .flatten()
            .map(String::as_str)
            .collect()
    }

    /// Every handle `person` ended under `operation`, in order of id.
    pub(super) fn ended_by(&self, person: &str, operation: &str) -> Vec<&str> {
        Self::named(&self.endings, person, operation)
    }

    /// The handles `person` ended under `operation` that are their own
    /// ending's root, in order of id.
    pub(super) fn roots_ended_by(&self, person: &str, operation: &str) -> Vec<&str> {
        Self::named(&self.roots, person, operation)
    }

    fn named<'a>(
        index: &'a BTreeMap<EndingKey, BTreeSet<String>>,
        person: &str,
        operation: &str,
    ) -> Vec<&'a str> {
        index
            .get(&(person.to_owned(), operation.to_owned()))
            .into_iter()
            .flatten()
            .map(String::as_str)
            .collect()
    }

    /// Marks the handle `id` dropped and ended as `ended` says.
    pub(super) fn end(&mut self, id: &str, ended: &Ended) {
        let Some(record) = self.records.get_mut(id) else {
            return;
        };
        record.dropped = true;
        let before = record.ended.replace(ended.clone());
        if let Some(before) = before {
            self.unindex_ending(id, &before);
        }
        self.index_ending(id, ended);
    }

    /// Applies `each` to every record, then indexes the endings again from
    /// what the records now say.
    pub(super) fn relay(&mut self, mut each: impl FnMut(&str, &mut HandleRecord)) {
        for (id, record) in &mut self.records {
            each(id, record);
        }
        self.endings.clear();
        self.roots.clear();
        let ended: Vec<(String, Ended)> = self
            .records
            .iter()
            .filter_map(|(id, record)| record.ended.clone().map(|ended| (id.clone(), ended)))
            .collect();
        for (id, ended) in &ended {
            self.index_ending(id, ended);
        }
    }
}
