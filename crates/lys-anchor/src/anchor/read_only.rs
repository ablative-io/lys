//! [`Anchor::open_read_only`] — opening an anchor with no signer and no policy,
//! and the two marker types that make the absence checkable by the compiler.
//!
//! # The principle, which is not this crate's
//!
//! `crates/lys/src/commands/log/store.rs` records it: **observing your own
//! append-only log should not require the ability to sign for it.** Before this
//! module, [`Anchor::open`] demanded both a signer and an admission policy, so a
//! `status` command had to supply a signing key to answer questions it never
//! signs for and an admission rule to answer questions no rule governs. Loading
//! a key to read is a key held for longer, in more processes, for no reason.
//!
//! # The shape, and why this one
//!
//! [`NoSigner`] and [`NoPolicy`] are field-less types that implement **nothing**
//! — not [`Signer`](crate::Signer), not
//! [`InProcessSigner`](crate::InProcessSigner), not
//! [`AdmissionPolicy`](crate::AdmissionPolicy). `open_read_only` returns an
//! [`Anchor<S, NoSigner, NoPolicy>`](Anchor), and every method that signs or
//! admits lives on an `impl` block bounded by the trait the corresponding
//! marker does not implement. So `publish_checkpoint`, `inclusion_artifact`,
//! `append`, `submit` and `receipt_for` are **not in scope** on that value; the
//! reads — `origin`, `tree_size`, `root`, `leaf_bytes`, `recovered_to`,
//! `config`, `status` — are.
//!
//! Three alternatives were available and are worse:
//!
//! - **`Option<K>` on the anchor.** Explicitly refused: a signerless anchor
//!   would then reach a signing path and be turned back at run time, which
//!   makes "cannot sign" a check somebody has to remember to write instead of a
//!   fact about the type. The compiler should refuse it.
//! - **A separate `AnchorReader` type.** It would need its own copy of every
//!   read accessor, and two copies of a read surface are two things that can
//!   drift — the objection this repo raises against a second copy of an encoder
//!   applies to a second copy of an accessor set for the same reason.
//! - **Marker types that implement the traits but fail at run time.** That is
//!   the `Option<K>` defect wearing a type parameter.
//!
//! The cost, named rather than discovered later: [`Anchor`]'s declaration can
//! no longer carry `K: InProcessSigner, P: AdmissionPolicy` as bounds, because
//! `NoSigner` satisfies neither. Bounds moved to the `impl` blocks —
//! [`anchor::open`](super::open) says what that buys. `Anchor<S, String, u32>`
//! is now a nameable type; it is also an uninhabitable one, because no
//! constructor in this crate produces one.
//!
//! # A read-only anchor is still an anchor, and still refuses a log without
//! genesis
//!
//! `open_read_only` performs exactly the checks [`Anchor::open`] performs: the
//! tree is rebuilt from stored leaves and reconciled with the pin, an
//! interrupted append is repaired and reported through
//! [`recovered_to`](Anchor::recovered_to), and a log with no leaves is refused.
//! Dropping the genesis check for a reader would make the read path accept logs
//! the write path rejects, so a `status` command would report happily on a log
//! that could never issue a receipt.

use lys_log_store::{LeafStore, Log};

use crate::config::AnchorConfig;
use crate::error::{AnchorError, AnchorResult};

use super::open::Anchor;

/// The signer of an anchor that cannot sign.
///
/// Implements no signing trait, deliberately and permanently. Its whole purpose
/// is to be a type argument the signing `impl` blocks do not accept, so that an
/// attempt to publish a checkpoint from a read-only anchor fails to compile.
///
/// **Do not implement [`Signer`](crate::Signer) or
/// [`InProcessSigner`](crate::InProcessSigner) for this type**, here or
/// downstream. Doing so would silently give every read-only anchor in existence
/// a signing path, and nothing else in the crate would change to say so.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NoSigner;

/// The admission policy of an anchor that admits nothing because it never
/// appends.
///
/// Implements no policy trait, deliberately. It is not "a policy that refuses
/// everything" — that would be [`MaxSize`](crate::MaxSize) with a limit of
/// zero, or an operator's own type, and it would still be a *decision an
/// operator made*. This is the absence of a decision, which is correct exactly
/// when there is no write path for a decision to govern.
///
/// It therefore also carries DP23 unchanged: no deployment inherits an
/// admission rule nobody chose, because this is not a rule and no value of this
/// type can be reached from anything that appends.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NoPolicy;

/// An anchor opened for reading: no key, no admission rule, no write path.
///
/// A type alias for the shape [`Anchor::open_read_only`] returns, so a caller
/// can name it in a signature without spelling out both markers.
pub type ReadOnlyAnchor<S> = Anchor<S, NoSigner, NoPolicy>;

impl<S: LeafStore> Anchor<S, NoSigner, NoPolicy> {
    /// Opens an anchor over an existing `store` for reading only, with no
    /// signer and no admission policy.
    ///
    /// Refuses a log with no leaves, repairs and reports an interrupted append,
    /// and rebuilds the tree from stored leaves — the same checks
    /// [`Anchor::open`] performs, for the reason in the [module docs](self).
    ///
    /// # What this value can and cannot do
    ///
    /// It can be asked what it holds. It cannot sign, append, or admit, and the
    /// second half of that is a compile-time fact rather than a runtime
    /// refusal. The pair of doctests below is the check.
    ///
    /// The first establishes that the probe compiles at all — a `compile_fail`
    /// doctest passes for *any* compilation error, including a typo or an
    /// unresolved name, so on its own it cannot tell a method that is absent
    /// from a snippet that never worked:
    ///
    /// ```
    /// # use lys_anchor::{AcceptAll, Anchor, AnchorConfig, FileSigner, WitnessPosture};
    /// # use lys_log_store::FileLeafStore;
    /// # let dir = tempfile::tempdir()?;
    /// # let path = dir.path().join("anchor");
    /// # let key = dir.path().join("anchor.key");
    /// # std::fs::write(&key, [7u8; 32])?;
    /// # let store = FileLeafStore::create(&path, "example.com/lys/doctest")?;
    /// # let created = Anchor::create(
    /// #     store, b"genesis", FileSigner::load(&key)?, AcceptAll,
    /// #     AnchorConfig::unconfigured(),
    /// # )?;
    /// # drop(created);
    /// let reader =
    ///     Anchor::open_read_only(FileLeafStore::open(&path)?, AnchorConfig::unconfigured())?;
    ///
    /// // The reads resolve, and a status cannot be had without its posture.
    /// let status = reader.status();
    /// assert_eq!(status.tree_size(), 1);
    /// assert_eq!(status.posture, WitnessPosture::Unwitnessed);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// The second differs from it in exactly one respect: it asks the same
    /// value to sign.
    ///
    /// ```compile_fail
    /// # use lys_anchor::{AcceptAll, Anchor, AnchorConfig, FileSigner};
    /// # use lys_log_store::FileLeafStore;
    /// # let dir = tempfile::tempdir()?;
    /// # let path = dir.path().join("anchor");
    /// # let key = dir.path().join("anchor.key");
    /// # std::fs::write(&key, [7u8; 32])?;
    /// # let store = FileLeafStore::create(&path, "example.com/lys/doctest")?;
    /// # let created = Anchor::create(
    /// #     store, b"genesis", FileSigner::load(&key)?, AcceptAll,
    /// #     AnchorConfig::unconfigured(),
    /// # )?;
    /// # drop(created);
    /// let reader =
    ///     Anchor::open_read_only(FileLeafStore::open(&path)?, AnchorConfig::unconfigured())?;
    ///
    /// // No signer, so there is no `publish_checkpoint` to call.
    /// reader.publish_checkpoint()?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    ///
    /// [`AnchorError::NoGenesisLeaf`] if the log has no leaves, and
    /// [`AnchorError::Store`] for anything the log or its storage refuses —
    /// notably `StoreError::PinMismatch` when the stored leaves no longer
    /// rebuild to the pinned root.
    pub fn open_read_only(store: S, config: AnchorConfig) -> AnchorResult<Self> {
        let log = Log::open(store)?;
        if log.tree().is_empty() {
            return Err(AnchorError::NoGenesisLeaf {
                origin: log.origin().to_string(),
            });
        }
        Ok(Self {
            log,
            signer: NoSigner,
            config,
            policy: NoPolicy,
        })
    }
}

#[cfg(test)]
#[path = "read_only_tests.rs"]
mod tests;
