//! What the review decisions' log folds to, and how that fold is sealed in
//! the log's signed snapshot so a start reads only the leaves after it.

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

/// The snapshot domain the decisions' folded state is sealed under.
pub const DOMAIN: &str = "lys/identity/review-decisions-state/v1";

const FORMAT: &str = "lys-review-decisions-state/v1";

/// A decision to keep a grant, as it is recorded.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct Kept {
    /// The grant kept.
    pub grant: String,
    /// The person who kept it.
    pub kept_by: String,
    /// The person's words.
    pub note: String,
    /// The operation id it was kept under, which names the decision.
    pub operation: String,
    /// When it was kept, in seconds since the Unix epoch.
    pub at: u64,
    /// The grants' revision the decision was made at.
    pub revision: u64,
}

impl Kept {
    /// Whether `other` asks for the same decision: the same grant, by the
    /// same person, in the same words, whenever it was asked.
    pub fn same_words(&self, other: &Self) -> bool {
        self.operation == other.operation
            && self.grant == other.grant
            && self.kept_by == other.kept_by
            && self.note == other.note
    }
}

/// The decisions as their log folds them, in the order recorded.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, from = "Records")]
pub struct Held {
    /// The decisions.
    pub kept: Vec<Kept>,
    #[serde(skip)]
    index: Arc<Index>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct Index {
    operations: HashMap<String, usize>,
    grants: HashMap<String, usize>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Records {
    kept: Vec<Kept>,
}

impl From<Records> for Held {
    fn from(records: Records) -> Self {
        let mut index = Index::default();
        for (position, kept) in records.kept.iter().enumerate() {
            index
                .operations
                .entry(kept.operation.clone())
                .or_insert(position);
            index.grants.insert(kept.grant.clone(), position);
        }
        Self {
            kept: records.kept,
            index: Arc::new(index),
        }
    }
}

#[derive(Serialize)]
struct Sealing<'a> {
    format: &'static str,
    held: &'a Held,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Sealed {
    format: String,
    held: Held,
}

impl Held {
    /// The decision recorded under `operation`.
    pub fn operation(&self, operation: &str) -> Option<&Kept> {
        self.index.operations.get(operation).and_then(|position| {
            #[cfg(test)]
            crate::folded_work::visit(crate::folded_work::Work::Review);
            self.kept.get(*position)
        })
    }

    /// The latest decision to keep `grant`.
    pub fn last_for(&self, grant: &str) -> Option<&Kept> {
        self.index.grants.get(grant).and_then(|position| {
            #[cfg(test)]
            crate::folded_work::visit(crate::folded_work::Work::Review);
            self.kept.get(*position)
        })
    }

    /// Fold one decision, refusing an operation already recorded.
    pub fn hold(&mut self, kept: Kept) -> Result<(), String> {
        if self.index.operations.contains_key(&kept.operation) {
            return Err(format!(
                "operation `{}` already names a recorded decision",
                kept.operation
            ));
        }
        let index = Arc::make_mut(&mut self.index);
        index
            .operations
            .insert(kept.operation.clone(), self.kept.len());
        index.grants.insert(kept.grant.clone(), self.kept.len());
        self.kept.push(kept);
        Ok(())
    }

    /// Fold every leaf of `tail`, in order.
    pub fn fold(&mut self, tail: &lys_log_store::Tail) -> Result<(), String> {
        for (index, bytes) in (tail.from..).zip(&tail.leaves) {
            let kept = serde_json::from_slice(bytes)
                .map_err(|error| format!("leaf {index} is not a review decision: {error}"))?;
            self.hold(kept)
                .map_err(|reason| format!("leaf {index}: {reason}"))?;
        }
        Ok(())
    }

    /// The state a snapshot seals.
    pub fn encode(&self) -> Result<Vec<u8>, String> {
        serde_json::to_vec(&Sealing {
            format: FORMAT,
            held: self,
        })
        .map_err(|error| format!("review decisions state: {error}"))
    }

    /// The state a snapshot sealed, refused by reason unless it reads whole
    /// in this format.
    pub fn decode(bytes: &[u8]) -> Result<Self, String> {
        let sealed: Sealed = serde_json::from_slice(bytes)
            .map_err(|error| format!("review decisions state: {error}"))?;
        if sealed.format != FORMAT {
            return Err(format!(
                "review decisions state is in format {}, not {FORMAT}",
                sealed.format
            ));
        }
        Ok(sealed.held)
    }
}
