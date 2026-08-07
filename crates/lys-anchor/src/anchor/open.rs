//! [`Anchor`] — creating and opening an anchor over a [`Log`].
//!
//! # Genesis is leaf 0 or it is nothing
//!
//! [`Anchor::create`] appends the caller's genesis bytes at index 0 and refuses
//! a store that already holds leaves; [`Anchor::open`] refuses a log with no
//! leaves at all. Between them, a value of this type always has a leaf 0 that
//! was placed there by the party that created the log.
//!
//! Both refusals exist because the position cannot be repaired later.
//! `LeafStore` offers no `insert`, no `rewrite` and no `fork` — deliberately —
//! so nothing can move an entry aside to make room at 0. An anchor initialised
//! without genesis is not a log missing an entry; it is a log that can never
//! issue a receipt for its first submission, because `lys-core`'s receipt
//! signing declines a tree of size 1 and the only way past size 1 without a
//! genesis leaf is to have already logged the entry the receipt is for.
//!
//! # The origin is read through, never held
//!
//! [`Anchor::origin`] forwards to `Log::origin`, which forwards to
//! `LeafStore::origin`, which returns the value fixed when the store was
//! created. This module holds no origin field and no origin constant. That is
//! not restraint — it is that neither [`Anchor::create`] nor [`Anchor::open`]
//! is given an origin to hold, so the store's is the only one in existence.
//!
//! # The signing key is held, and only through the custody trait
//!
//! An anchor holds its signer from construction rather than being handed one
//! per call: a key passed in at each signing site is a key every caller must
//! hold, which is the leak the boundary exists to prevent. The type parameter
//! is [`InProcessSigner`] and not [`Signer`](crate::keys::Signer), which is not
//! an oversight — see
//! [`keys::signer`](crate::keys::signer) for what that bound records and what
//! would remove it.
//!
//! # A repair is returned, not reported
//!
//! `Log::open` repairs exactly one divergence — storage one leaf ahead of the
//! pin, which is an append interrupted between the two writes — and reports it
//! through `recovered_to`. [`Anchor::recovered_to`] forwards that fact
//! unchanged. This layer neither prints it nor decides it is unimportant: a
//! library that writes to stderr has chosen for its caller how a repair gets
//! reported, and a repair the operator never hears about is indistinguishable
//! from one that never happened.

use lys_log_store::{LeafStore, Log};

use crate::config::AnchorConfig;
use crate::error::{AnchorError, AnchorResult};
use crate::keys::InProcessSigner;

/// A transparency anchor: one append-only log, with a genesis leaf.
///
/// Generic over the storage backend so a file-backed anchor is a type
/// parameter rather than a commitment, and over the signer so key custody is a
/// caller's choice rather than this crate's. Holds no origin and no peer —
/// every operation completes with nothing else in existence.
pub struct Anchor<S: LeafStore, K: InProcessSigner> {
    // Visible to the rest of `anchor::` — `publish_checkpoint` and everything
    // after it live in sibling files to keep each one small, and they need the
    // log and the key. Not `pub(crate)`: no module outside `anchor::` has any
    // business reaching past the accessors.
    pub(super) log: Log<S>,
    pub(super) signer: K,
    pub(super) config: AnchorConfig,
}

impl<S: LeafStore, K: InProcessSigner> std::fmt::Debug for Anchor<S, K> {
    /// Summarizes the anchor without dumping log content.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Anchor")
            .field("origin", &self.origin())
            .field("tree_size", &self.tree_size())
            .field("recovered_to", &self.recovered_to())
            .finish_non_exhaustive()
    }
}

impl<S: LeafStore, K: InProcessSigner> Anchor<S, K> {
    /// Creates an anchor over `store`, appending `genesis` as leaf 0.
    ///
    /// The genesis bytes are supplied by the caller and are not interpreted:
    /// they are stored verbatim, like any other leaf. The store must already
    /// exist and must be empty — it carries the origin, which is fixed at its
    /// creation and is never chosen here. `signer` is the key this anchor will
    /// publish under; it is never consulted while creating the log.
    ///
    /// # Errors
    ///
    /// [`AnchorError::GenesisAlreadyWritten`] if the store already holds
    /// leaves, and [`AnchorError::Store`] for anything the log or its storage
    /// refuses — including an integrity failure found while opening.
    pub fn create(store: S, genesis: &[u8], signer: K, config: AnchorConfig) -> AnchorResult<Self> {
        let mut log = Log::open(store)?;
        let tree_size = log.tree().len();
        if tree_size != 0 {
            return Err(AnchorError::GenesisAlreadyWritten {
                origin: log.origin().to_string(),
                tree_size,
            });
        }
        log.append(genesis)?;
        Ok(Self {
            log,
            signer,
            config,
        })
    }

    /// Opens an anchor over an existing `store`, refusing a log with no leaves.
    ///
    /// Opening rebuilds the tree from stored leaves and reconciles it with the
    /// pin, so a leaf altered on disk is caught here rather than at first use.
    /// If an interrupted append was repaired, [`recovered_to`](Self::recovered_to)
    /// returns the size it was repaired to; the repair is never silent and this
    /// layer never consumes it.
    ///
    /// # Errors
    ///
    /// [`AnchorError::NoGenesisLeaf`] if the log has no leaves, and
    /// [`AnchorError::Store`] for anything the log or its storage refuses —
    /// notably `StoreError::PinMismatch` when the stored leaves no longer
    /// rebuild to the pinned root.
    pub fn open(store: S, signer: K, config: AnchorConfig) -> AnchorResult<Self> {
        let log = Log::open(store)?;
        if log.tree().is_empty() {
            return Err(AnchorError::NoGenesisLeaf {
                origin: log.origin().to_string(),
            });
        }
        Ok(Self {
            log,
            signer,
            config,
        })
    }

    /// The anchor's origin, as its store reports it.
    ///
    /// Read through to storage on every call. This crate has no origin of its
    /// own to return instead.
    pub fn origin(&self) -> &str {
        self.log.origin()
    }

    /// The number of leaves in the log, genesis included.
    ///
    /// Never zero for a value of this type: `create` writes leaf 0 and `open`
    /// refuses a log without one.
    pub fn tree_size(&self) -> u64 {
        self.log.tree().len()
    }

    /// The tree size an interrupted append was recovered to when this anchor
    /// was opened, if one was. `None` means the log opened clean.
    ///
    /// Forwarded from the log rather than acted on. A caller that ignores it
    /// has decided to; a library that swallowed it would have decided for them.
    pub fn recovered_to(&self) -> Option<u64> {
        self.log.recovered_to()
    }

    /// The configuration this anchor was constructed with.
    pub fn config(&self) -> &AnchorConfig {
        &self.config
    }

    /// The signer this anchor publishes under.
    ///
    /// Exposed so a caller can obtain the public key to hand a verifier — the
    /// only thing anybody outside needs from it. The trait it is bounded by
    /// offers no route to the private key.
    pub fn signer(&self) -> &K {
        &self.signer
    }
}

#[cfg(test)]
#[path = "open_tests.rs"]
mod tests;
