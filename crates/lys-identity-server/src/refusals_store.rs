//! The tool calls the runners' judges denied, as the server keeps them.
//!
//! A refusal is read from its runner's feed and kept as a leaf of the
//! budgets' log, beside the uses and crossings it explains, so it is sealed
//! in the same signed snapshot and replayed from the same tail. It is kept
//! once for its runner and attempt: the same feed read again after a lost
//! answer, or after a restart that replays from the snapshot, keeps no
//! second record. How far each runner's feed has been read is kept the same
//! way, so a start reads on from there and never the whole feed again.

use std::collections::{BTreeMap, BTreeSet};

use lys_runner::refusals::RefusalRecord;
use serde::{Deserialize, Serialize};

/// Every refusal kept, and how far each runner's feed has been read.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Refusals {
    /// Each refusal, in the order kept.
    pub records: Vec<RefusalRecord>,
    /// The runner and attempt of each, so none is kept twice.
    pub kept: BTreeSet<String>,
    /// The cursor each runner's feed was last read to.
    pub cursors: BTreeMap<String, String>,
}

/// How far a runner's feed has been read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FeedRead {
    /// The runner.
    pub runner: String,
    /// The cursor after the last entry kept.
    pub cursor: String,
}

/// The key a refusal is kept once under.
pub fn key(record: &RefusalRecord) -> String {
    format!("{}\n{}", record.source, record.attempt)
}

impl Refusals {
    /// Hold `record`, once.
    pub fn hold(&mut self, record: RefusalRecord) {
        if self.kept.insert(key(&record)) {
            self.records.push(record);
        }
    }

    /// Hold how far a runner's feed has been read.
    pub fn read_to(&mut self, read: FeedRead) {
        self.cursors.insert(read.runner, read.cursor);
    }

    /// Whether `record` is kept already.
    pub fn holds(&self, record: &RefusalRecord) -> bool {
        self.kept.contains(&key(record))
    }

    /// `agent`'s refusals, newest first.
    pub fn of_agent(&self, agent: &str) -> Vec<RefusalRecord> {
        self.records
            .iter()
            .rev()
            .filter(|record| record.agent == agent)
            .cloned()
            .collect()
    }
}
