//! Ephemeral, bounded tail evidence supplied by an explicit coherent provider.

use std::path::Path;
use std::sync::Arc;

use crate::{Frontier, PinnedRoot, StoreError, StoreResult};

#[path = "file/witness.rs"]
mod file_provider;
pub use file_provider::FileTailProvider;

/// One opaque leaf and its exact position in a certified tail.
#[derive(Clone, PartialEq, Eq)]
pub struct TailLeaf {
    /// The contiguous log index.
    pub index: u64,
    /// Original leaf bytes, without decoding or rewriting.
    pub bytes: Vec<u8>,
}

impl std::fmt::Debug for TailLeaf {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("TailLeaf").field("index", &self.index)
            .field("bytes", &self.bytes.len()).finish()
    }
}

/// A complete tail extending a trusted frontier to one coherently read head.
/// It is usable only inside a successful provider verification callback.
#[derive(Clone)]
pub struct TailWitness {
    owner: Arc<()>,
    /// Immutable log origin, checked against the actual provider.
    pub origin: String,
    /// The exact trusted settled frontier.
    pub lower: PinnedRoot,
    /// The freshly certified upper frontier.
    pub upper: PinnedRoot,
    /// Every leaf in the contiguous range between the two bounds.
    pub leaves: Vec<TailLeaf>,
}

impl PartialEq for TailWitness {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.owner, &other.owner) && self.origin == other.origin
            && self.lower == other.lower && self.upper == other.upper && self.leaves == other.leaves
    }
}

impl Eq for TailWitness {}

impl std::fmt::Debug for TailWitness {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("TailWitness").field("origin", &self.origin)
            .field("lower_size", &self.lower.tree_size)
            .field("upper_size", &self.upper.tree_size)
            .field("leaves", &self.leaves.len()).finish()
    }
}

/// An explicitly supplied capability, separate from opaque byte storage.
/// Verification holds append coherence only for the supplied reading callback.
pub trait TailWitnessProvider {
    /// Acquire every leaf after the trusted frontier under one coherent head.
    ///
    /// # Errors
    /// Returns the original store refusal if the head, tail or bound is invalid.
    fn acquire(&self, settled: &Frontier) -> StoreResult<TailWitness>;

    /// Revalidate both bounds and every leaf against the current provider head.
    /// An append after acquisition refuses before the callback is called.
    ///
    /// # Errors
    /// Returns the original coherence, corruption or callback refusal unchanged.
    fn verify(
        &self,
        witness: &TailWitness,
        settled: &Frontier,
        reading: &mut dyn FnMut(&TailWitness) -> StoreResult<()>,
    ) -> StoreResult<()>;
}

pub(super) fn validate(
    witness: &TailWitness,
    settled: &Frontier,
    origin: &str,
    upper: PinnedRoot,
    path: &Path,
) -> StoreResult<()> {
    let corrupt = |reason: &str| StoreError::Corrupt {
        path: path.to_path_buf(), reason: reason.to_owned(),
    };
    let lower = PinnedRoot { tree_size: settled.size(), root: settled.root() };
    if witness.origin != origin {
        return Err(corrupt("tail witness names a different log origin"));
    }
    if witness.lower != lower {
        return Err(corrupt("tail witness does not begin at the trusted settled frontier"));
    }
    if witness.upper != upper || upper.tree_size < lower.tree_size {
        return Err(corrupt("tail witness does not end at the certified current frontier"));
    }
    let count = u64::try_from(witness.leaves.len()).map_err(|source|
        StoreError::LeafCountUnrepresentable { count: upper.tree_size, source })?;
    if count != upper.tree_size - lower.tree_size {
        return Err(corrupt("tail witness does not contain the entire contiguous range"));
    }
    let mut extended = settled.clone();
    for (index, leaf) in (lower.tree_size..upper.tree_size).zip(&witness.leaves) {
        if leaf.index != index {
            return Err(corrupt("tail witness indexes are omitted, duplicated or reordered"));
        }
        extended.push(&leaf.bytes);
    }
    if extended.size() != upper.tree_size || extended.root() != upper.root {
        return Err(corrupt("tail witness leaves do not extend to the certified root"));
    }
    Ok(())
}
