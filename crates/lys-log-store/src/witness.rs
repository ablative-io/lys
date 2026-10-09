//! Ephemeral, bounded tail evidence supplied by an explicit coherent provider.

use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

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
        formatter
            .debug_struct("TailLeaf")
            .field("index", &self.index)
            .field("bytes", &self.bytes.len())
            .finish()
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
        Arc::ptr_eq(&self.owner, &other.owner)
            && self.origin == other.origin
            && self.lower == other.lower
            && self.upper == other.upper
            && self.leaves == other.leaves
    }
}

impl Eq for TailWitness {}

impl std::fmt::Debug for TailWitness {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("TailWitness")
            .field("origin", &self.origin)
            .field("lower_size", &self.lower.tree_size)
            .field("upper_size", &self.upper.tree_size)
            .field("leaves", &self.leaves.len())
            .finish_non_exhaustive()
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
    let refused = |reason: &'static str| StoreError::TailWitnessRefused {
        path: path.to_path_buf(),
        reason,
    };
    let lower = PinnedRoot {
        tree_size: settled.size(),
        root: settled.root(),
    };
    if witness.origin != origin {
        return Err(refused("tail witness names a different log origin"));
    }
    if witness.lower != lower {
        return Err(refused(
            "tail witness does not begin at the trusted settled frontier",
        ));
    }
    if witness.upper != upper || upper.tree_size < lower.tree_size {
        return Err(refused(
            "tail witness does not end at the certified current frontier",
        ));
    }
    let count = u64::try_from(witness.leaves.len()).map_err(|source| {
        StoreError::LeafCountUnrepresentable {
            count: upper.tree_size,
            source,
        }
    })?;
    if count != upper.tree_size - lower.tree_size {
        return Err(refused(
            "tail witness does not contain the entire contiguous range",
        ));
    }
    let mut extended = settled.clone();
    for (index, leaf) in (lower.tree_size..upper.tree_size).zip(&witness.leaves) {
        if leaf.index != index {
            return Err(refused(
                "tail witness indexes are omitted, duplicated or reordered",
            ));
        }
        extended.push(&leaf.bytes);
    }
    if extended.size() != upper.tree_size || extended.root() != upper.root {
        return Err(refused(
            "tail witness leaves do not extend to the certified root",
        ));
    }
    Ok(())
}

/// The provider step an instance-private fault refuses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TailFaultStep {
    /// [`TailWitnessProvider::acquire`].
    Acquire,
    /// [`TailWitnessProvider::verify`], before its reading is called.
    Verify,
}

type MakeError = Box<dyn Fn() -> StoreError + Send + Sync>;

/// The faults and counts of one [`FaultTailProvider`], shared only with the
/// party that made it: no process-wide switch reaches another instance.
#[derive(Default)]
pub struct TailFaults {
    acquire: Mutex<Option<MakeError>>,
    verify: Mutex<Option<MakeError>>,
    acquisitions: AtomicU64,
    verifications: AtomicU64,
    readings: AtomicU64,
}

impl std::fmt::Debug for TailFaults {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("TailFaults")
            .field("acquisitions", &self.acquisitions())
            .field("verifications", &self.verifications())
            .field("readings", &self.readings())
            .finish_non_exhaustive()
    }
}

impl TailFaults {
    fn slot(&self, step: TailFaultStep) -> &Mutex<Option<MakeError>> {
        match step {
            TailFaultStep::Acquire => &self.acquire,
            TailFaultStep::Verify => &self.verify,
        }
    }

    /// Refuse every later call of `step` with the error `make` builds, until
    /// [`TailFaults::clear`]. The refusal is returned as built, unchanged.
    ///
    /// # Errors
    ///
    /// [`StoreError::LockPoisoned`] when the step's lock was poisoned.
    pub fn refuse(
        &self,
        step: TailFaultStep,
        make: impl Fn() -> StoreError + Send + Sync + 'static,
    ) -> StoreResult<()> {
        *self.slot(step).lock().map_err(|_| poisoned())? = Some(Box::new(make));
        Ok(())
    }

    /// Let `step` reach the wrapped provider again.
    ///
    /// # Errors
    ///
    /// [`StoreError::LockPoisoned`] when the step's lock was poisoned.
    pub fn clear(&self, step: TailFaultStep) -> StoreResult<()> {
        *self.slot(step).lock().map_err(|_| poisoned())? = None;
        Ok(())
    }

    fn armed(&self, step: TailFaultStep) -> StoreResult<Option<StoreError>> {
        Ok(self
            .slot(step)
            .lock()
            .map_err(|_| poisoned())?
            .as_ref()
            .map(|make| make()))
    }

    /// Acquisitions asked of the provider, refused or not.
    #[must_use]
    pub fn acquisitions(&self) -> u64 {
        self.acquisitions.load(Ordering::SeqCst)
    }

    /// Verifications asked of the provider, refused or not.
    #[must_use]
    pub fn verifications(&self) -> u64 {
        self.verifications.load(Ordering::SeqCst)
    }

    /// Readings the wrapped provider certified and handed on.
    #[must_use]
    pub fn readings(&self) -> u64 {
        self.readings.load(Ordering::SeqCst)
    }
}

/// A complete tail capability whose every step can be refused by its own
/// instance's faults. Unrefused, each step is the wrapped provider's own.
pub struct FaultTailProvider<P> {
    inner: P,
    faults: Arc<TailFaults>,
}

impl<P: TailWitnessProvider> FaultTailProvider<P> {
    /// Wrap `inner`, answering the faults only this instance obeys.
    #[must_use]
    pub fn new(inner: P) -> (Self, Arc<TailFaults>) {
        let faults = Arc::new(TailFaults::default());
        (
            Self {
                inner,
                faults: Arc::clone(&faults),
            },
            faults,
        )
    }
}

impl<P: TailWitnessProvider> TailWitnessProvider for FaultTailProvider<P> {
    fn acquire(&self, settled: &Frontier) -> StoreResult<TailWitness> {
        self.faults.acquisitions.fetch_add(1, Ordering::SeqCst);
        if let Some(error) = self.faults.armed(TailFaultStep::Acquire)? {
            return Err(error);
        }
        self.inner.acquire(settled)
    }

    fn verify(
        &self,
        witness: &TailWitness,
        settled: &Frontier,
        reading: &mut dyn FnMut(&TailWitness) -> StoreResult<()>,
    ) -> StoreResult<()> {
        self.faults.verifications.fetch_add(1, Ordering::SeqCst);
        if let Some(error) = self.faults.armed(TailFaultStep::Verify)? {
            return Err(error);
        }
        self.inner.verify(witness, settled, &mut |certified| {
            self.faults.readings.fetch_add(1, Ordering::SeqCst);
            reading(certified)
        })
    }
}

/// A fault step's lock was poisoned: refused by name, never read past.
fn poisoned() -> StoreError {
    StoreError::LockPoisoned {
        what: "tail fault step",
    }
}
