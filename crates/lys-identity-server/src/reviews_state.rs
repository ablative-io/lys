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
    #[must_use]
    pub fn same_words(&self, other: &Self) -> bool {
        self.operation == other.operation
            && self.grant == other.grant
            && self.kept_by == other.kept_by
            && self.note == other.note
    }
}

/// One recorded act keeping exactly the grants the reviewer saw.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct KeptBatch {
    /// The operation naming the whole act.
    pub operation: String,
    /// The agent whose access was checked.
    pub agent: String,
    /// One decision per grant, in the order the reviewer sent them.
    pub kept: Vec<Kept>,
}

/// A durable leaf: old single-grant decisions keep their original encoding.
#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub(crate) enum Decision {
    Single(Kept),
    Batch(KeptBatch),
}

/// Whether the written period has elapsed since the last check. Giving is
/// the first check; a future instant never counts as an elapsed period.
#[must_use]
pub fn period_passed(starts_at: u64, last: Option<&Kept>, at: u64, days: u32) -> bool {
    let checked_at = last.map_or(starts_at, |kept| kept.at);
    at.checked_sub(checked_at)
        .is_some_and(|elapsed| elapsed >= u64::from(days) * 86_400)
}

/// The decisions as their log folds them, in the order recorded.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, from = "Records")]
pub struct Held {
    /// The decisions.
    pub kept: Vec<Kept>,
    batches: Vec<KeptBatch>,
    #[serde(skip)]
    index: Arc<Index>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct Index {
    operations: HashMap<String, usize>,
    grants: HashMap<String, usize>,
    batches: HashMap<String, usize>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Records {
    kept: Vec<Kept>,
    #[serde(default)]
    batches: Vec<KeptBatch>,
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
        for (position, batch) in records.batches.iter().enumerate() {
            index.batches.insert(batch.operation.clone(), position);
        }
        Self {
            kept: records.kept,
            batches: records.batches,
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
    /// The whole act recorded under an operation, if it was a batch.
    #[must_use]
    pub fn batch(&self, operation: &str) -> Option<&KeptBatch> {
        self.index
            .batches
            .get(operation)
            .and_then(|position| self.batches.get(*position))
    }

    /// Fold a whole leaf atomically, validating it before changing the state.
    pub(crate) fn hold_decision(&mut self, decision: Decision) -> Result<(), String> {
        match decision {
            Decision::Single(kept) => self.hold(kept),
            Decision::Batch(batch) => {
                let mut grants = std::collections::BTreeSet::new();
                let Some(first) = batch.kept.first() else {
                    return Err("a review batch contains at least one decision".to_owned());
                };
                if self.index.operations.contains_key(&batch.operation)
                    || batch.kept.iter().any(|kept| {
                        kept.operation != batch.operation
                            || kept.kept_by != first.kept_by
                            || !grants.insert(kept.grant.clone())
                    })
                {
                    return Err("a review batch reuses an operation or is not one reviewer's distinct grants".to_owned());
                }
                let index = Arc::make_mut(&mut self.index);
                index
                    .operations
                    .insert(batch.operation.clone(), self.kept.len());
                index
                    .batches
                    .insert(batch.operation.clone(), self.batches.len());
                for kept in &batch.kept {
                    index.grants.insert(kept.grant.clone(), self.kept.len());
                    self.kept.push(kept.clone());
                }
                self.batches.push(batch);
                Ok(())
            }
        }
    }

    /// The decision recorded under `operation`.
    #[must_use]
    pub fn operation(&self, operation: &str) -> Option<&Kept> {
        self.index.operations.get(operation).and_then(|position| {
            #[cfg(test)]
            crate::folded_work::visit(crate::folded_work::Work::Review);
            self.kept.get(*position)
        })
    }

    /// The latest decision to keep `grant`.
    #[must_use]
    pub fn last_for(&self, grant: &str) -> Option<&Kept> {
        self.index.grants.get(grant).and_then(|position| {
            #[cfg(test)]
            crate::folded_work::visit(crate::folded_work::Work::Review);
            self.kept.get(*position)
        })
    }

    /// Fold one decision, refusing an operation already recorded.
    ///
    /// # Errors
    /// Refuses an operation that already names a retained review decision.
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
    ///
    /// # Errors
    /// Refuses a malformed review decision or a reused decision operation,
    /// naming the leaf that failed.
    pub fn fold(&mut self, tail: &lys_log_store::Tail) -> Result<(), String> {
        for (index, bytes) in (tail.from..).zip(&tail.leaves) {
            let kept = serde_json::from_slice(bytes)
                .map_err(|error| format!("leaf {index} is not a review decision: {error}"))?;
            self.hold_decision(kept)
                .map_err(|reason| format!("leaf {index}: {reason}"))?;
        }
        Ok(())
    }

    /// The state a snapshot seals.
    ///
    /// # Errors
    /// Returns the serialization failure if the review decisions snapshot
    /// cannot be encoded.
    pub fn encode(&self) -> Result<Vec<u8>, String> {
        serde_json::to_vec(&Sealing {
            format: FORMAT,
            held: self,
        })
        .map_err(|error| format!("review decisions state: {error}"))
    }

    /// The state a snapshot sealed, refused by reason unless it reads whole
    /// in this format.
    ///
    /// # Errors
    /// Refuses bytes that do not read as a whole review decisions snapshot in
    /// the review decisions state format.
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
