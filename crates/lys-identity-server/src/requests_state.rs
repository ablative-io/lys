//! What the access requests' log folds to, and how that fold is sealed in
//! the log's signed snapshot so a start reads only the leaves after it.

use std::collections::BTreeMap;
use std::sync::Arc;

use lys_log_store::{Frontier, LeafStore, PinnedRoot, StoreError, StoreResult, Tail};
use serde::{Deserialize, Serialize};

use crate::requests_store::{Asked, Decided, Intended, Line};

/// The snapshot domain the requests' folded state is sealed under.
pub const DOMAIN: &str = "lys/identity/access-requests-state/v1";

const FORMAT: &str = "lys-requests-state/v1";

/// The requests as their log folds them.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, from = "Records")]
pub(crate) struct Held {
    pub(crate) asked: Vec<Asked>,
    pub(crate) intended: BTreeMap<String, Intended>,
    pub(crate) decided: BTreeMap<String, Decided>,
    #[serde(skip)]
    pub(crate) ordered: Arc<BTreeMap<String, Vec<usize>>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Records {
    asked: Vec<Asked>,
    intended: BTreeMap<String, Intended>,
    decided: BTreeMap<String, Decided>,
}

impl From<Records> for Held {
    fn from(records: Records) -> Self {
        let mut ordered: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        for (position, asked) in records.asked.iter().enumerate() {
            ordered.entry(asked.id.clone()).or_default().push(position);
        }
        Self {
            asked: records.asked,
            intended: records.intended,
            decided: records.decided,
            ordered: Arc::new(ordered),
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
    /// Fold one line.
    pub(crate) fn hold(&mut self, line: Line) {
        match line {
            Line::Asked(asked) => {
                Arc::make_mut(&mut self.ordered)
                    .entry(asked.id.clone())
                    .or_default()
                    .push(self.asked.len());
                self.asked.push(asked);
            }
            Line::Intended(intended) => {
                self.intended.insert(intended.id.clone(), intended);
            }
            Line::Withdrawn(withdrawn) => {
                if self
                    .intended
                    .get(&withdrawn.id)
                    .is_some_and(|kept| kept.operation == withdrawn.operation)
                {
                    self.intended.remove(&withdrawn.id);
                }
            }
            Line::Decided(decided) => {
                self.intended.remove(&decided.id);
                self.decided.insert(decided.id.clone(), decided);
            }
        }
    }

    /// Fold every leaf of `tail`, in order.
    pub(crate) fn fold(&mut self, tail: &Tail) -> Result<(), String> {
        for (index, bytes) in (tail.from..).zip(&tail.leaves) {
            let line = serde_json::from_slice(bytes)
                .map_err(|error| format!("leaf {index} is not a request line: {error}"))?;
            self.hold(line);
        }
        Ok(())
    }

    /// The state a snapshot seals.
    pub(crate) fn encode(&self) -> Result<Vec<u8>, String> {
        serde_json::to_vec(&Sealing {
            format: FORMAT,
            held: self,
        })
        .map_err(|error| format!("requests state: {error}"))
    }

    /// The state a snapshot sealed, refused by reason unless it reads whole
    /// in this format.
    pub(crate) fn decode(bytes: &[u8]) -> Result<Self, String> {
        let sealed: Sealed =
            serde_json::from_slice(bytes).map_err(|error| format!("requests state: {error}"))?;
        if sealed.format != FORMAT {
            return Err(format!(
                "requests state is in format {}, not {FORMAT}",
                sealed.format
            ));
        }
        Ok(sealed.held)
    }
}

/// Pin the leaves of a requests log written before its leaves were pinned:
/// such a log holds leaves and no pin, and no snapshot. Answers how many
/// leaves were pinned, zero when the log needed none.
pub(crate) fn pin_unpinned<S: LeafStore>(store: &mut S) -> StoreResult<u64> {
    let extent = store.extent();
    if store.pinned().tree_size != 0 || extent < 2 || store.snapshot()?.is_some() {
        return Ok(0);
    }
    let mut frontier = Frontier::new();
    for index in 0..extent {
        let bytes = store
            .leaf(index)?
            .ok_or(StoreError::LeafMissingWithinExtent { index, extent })?;
        frontier.push(&bytes);
    }
    store.pin(PinnedRoot {
        tree_size: frontier.size(),
        root: frontier.root(),
    })?;
    Ok(extent)
}
