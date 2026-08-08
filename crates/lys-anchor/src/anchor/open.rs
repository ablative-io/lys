//! [`Anchor`] — creating and opening an anchor over a [`Log`].
//!
//! # Genesis is leaf 0 or it is nothing
//!
//! [`Anchor::create`] appends the caller's genesis bytes at index 0 and refuses
//! a store that already holds leaves; [`Anchor::open`] refuses a log with no
//! leaves at all. Between them, a value of this type always has a leaf 0 that
//! was placed there by the party that created the log.
//!
//! # There is a second genesis constructor, and this one does not interpret anything
//!
//! `Anchor::create_with_delegated_genesis` — named in plain text because a link
//! from these ungated docs to a gated item resolves under `--all-features` and
//! breaks the default `cargo doc`, which is a gate — builds leaf 0 as a
//! `lys/anchor-delegation/v1` artifact signed by the anchor's offline root key.
//! It is the DP16 constructor and it lives behind `unstable-anchor`, because the
//! delegation format does.
//!
//! [`Anchor::create`] is deliberately **not** that, and is not a deprecated
//! path either: it stores the operator's bytes verbatim, and it is the only
//! genesis a default-features build has. **A default build therefore cannot
//! create a DP16-conformant anchor**, and cannot until the delegation format is
//! ratified and its gate comes off. That is stated here rather than in the
//! gated module alone, because a reader arriving at `create` is exactly the
//! reader who needs to know its leaf 0 means nothing in particular.
//! `anchor::genesis`'s module docs carry the four alternatives that were
//! rejected to reach two constructors.
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
//! # The admission policy is held too, and there is no way to not choose one
//!
//! An anchor's third type parameter is its [`AdmissionPolicy`]. Both
//! constructors take one by value and neither has an overload that omits it,
//! because DP23's ruling is that **no deployment inherits an admission rule
//! nobody chose** — and a convenience constructor that picked one would undo
//! that in a line. There is no `Default` on any policy this crate ships and
//! none should be added downstream.
//!
//! **Creating an anchor does not consult the policy.** Genesis is the
//! operator's own bytes, handed to [`Anchor::create`] at construction, not a
//! stranger's submission; policing it would make an anchor's existence depend
//! on its access-control rule agreeing with its operator, and a policy that
//! refuses everything would leave nothing to run it on. So `create` writes leaf
//! 0 regardless, and the policy governs submissions only. That is asserted in
//! this module's tests rather than left as an intention.
//!
//! # The bounds live on the impl blocks, not on the type
//!
//! [`Anchor`]'s declaration names no bound on its signer or its policy
//! parameter; each `impl` block names the bounds its methods actually need.
//! That is not stylistic. It is what lets one type carry two capabilities:
//! `create` and `open` require `K: InProcessSigner` and `P: AdmissionPolicy`,
//! so an anchor built through them can sign and can admit — while
//! [`Anchor::open_read_only`](crate::Anchor::open_read_only) produces an
//! `Anchor<S, NoSigner, NoPolicy>`, and [`NoSigner`](crate::NoSigner)
//! implements neither trait, so **the signing and admitting methods are not in
//! scope on that value at all**. The refusal is a resolution failure at compile
//! time, not a runtime check that could be forgotten, and there is no
//! `Option<K>` anywhere for a signerless anchor to travel through a signing
//! path inside.
//!
//! The reads below — origin, size, root, leaf bytes, repair, config — are on
//! the unbounded block for the same reason: none of them signs anything, so
//! none of them should need the ability to.
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

use lys_core::merkle::RootHash;
use lys_log_store::{LeafStore, Log};

use crate::admission::AdmissionPolicy;
use crate::config::AnchorConfig;
use crate::error::{AnchorError, AnchorResult};
use crate::keys::InProcessSigner;

/// A transparency anchor: one append-only log, with a genesis leaf.
///
/// Generic over the storage backend so a file-backed anchor is a type
/// parameter rather than a commitment, over the signer so key custody is a
/// caller's choice rather than this crate's, and over the admission policy so
/// no deployment inherits an admission rule nobody chose. Holds no origin and
/// no peer — every operation completes with nothing else in existence.
///
/// `K` and `P` carry no bound here; see the [module docs](self) for why the
/// bounds sit on the `impl` blocks and what that buys — a read-only anchor for
/// which the signing methods do not exist.
pub struct Anchor<S: LeafStore, K, P> {
    // Visible to the rest of `anchor::` — `publish_checkpoint` and everything
    // after it live in sibling files to keep each one small, and they need the
    // log and the key. Not `pub(crate)`: no module outside `anchor::` has any
    // business reaching past the accessors.
    pub(super) log: Log<S>,
    pub(super) signer: K,
    pub(super) config: AnchorConfig,
    pub(super) policy: P,
}

impl<S: LeafStore, K, P> std::fmt::Debug for Anchor<S, K, P> {
    /// Summarizes the anchor without dumping log content.
    ///
    /// **The admission policy is deliberately not among the fields.** A
    /// policy's `Debug` names its rule — a size limit, an issuer key, an
    /// allow-list — and `Debug` output travels: into panic messages, error
    /// reports and log lines that a submitter may well end up reading. The
    /// refusal path spent an entire type on not disclosing the rule, and a
    /// derived formatter would hand it over for free. An operator who wants it
    /// has [`Anchor::policy`].
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Anchor")
            .field("origin", &self.origin())
            .field("tree_size", &self.tree_size())
            .field("recovered_to", &self.recovered_to())
            .finish_non_exhaustive()
    }
}

impl<S: LeafStore, K: InProcessSigner, P: AdmissionPolicy> Anchor<S, K, P> {
    /// Creates an anchor over `store`, appending `genesis` as leaf 0.
    ///
    /// The genesis bytes are supplied by the caller and are not interpreted:
    /// they are stored verbatim, like any other leaf, and nothing downstream may
    /// conclude anything from them. For an anchor whose leaf 0 is a signed
    /// delegation from its root key to its operational key — DP16's two-key
    /// model, and the shape a real anchor must have — reach
    /// `Anchor::create_with_delegated_genesis`, behind the `unstable-anchor`
    /// feature. See the [module docs](self) for why both exist.
    ///
    /// The store must already
    /// exist and must be empty — it carries the origin, which is fixed at its
    /// creation and is never chosen here. `signer` is the key this anchor will
    /// publish under; it is never consulted while creating the log. `policy`
    /// governs submissions and **is not consulted for genesis** — see the
    /// [module docs](self) for why an anchor's own creation is not subject to
    /// its access-control rule.
    ///
    /// # Errors
    ///
    /// [`AnchorError::GenesisAlreadyWritten`] if the store already holds
    /// leaves, and [`AnchorError::Store`] for anything the log or its storage
    /// refuses — including an integrity failure found while opening.
    pub fn create(
        store: S,
        genesis: &[u8],
        signer: K,
        policy: P,
        config: AnchorConfig,
    ) -> AnchorResult<Self> {
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
            policy,
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
    /// `policy` is named here for the same reason it is named in
    /// [`create`](Self::create): an admission rule is not a property of the
    /// stored log and is not recovered from it. Two processes may open the same
    /// store under two different policies, and the log carries no record of
    /// which one admitted what — an anchor's log is what was admitted, not why.
    ///
    /// # Errors
    ///
    /// [`AnchorError::NoGenesisLeaf`] if the log has no leaves, and
    /// [`AnchorError::Store`] for anything the log or its storage refuses —
    /// notably `StoreError::PinMismatch` when the stored leaves no longer
    /// rebuild to the pinned root.
    pub fn open(store: S, signer: K, policy: P, config: AnchorConfig) -> AnchorResult<Self> {
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
            policy,
        })
    }

    /// The signer this anchor publishes under.
    ///
    /// Exposed so a caller can obtain the public key to hand a verifier — the
    /// only thing anybody outside needs from it. The trait it is bounded by
    /// offers no route to the private key.
    pub fn signer(&self) -> &K {
        &self.signer
    }

    /// The admission policy this anchor was constructed with.
    ///
    /// Borrowed, never replaced: there is no setter. An anchor whose admission
    /// rule could be swapped mid-life would have a log whose contents were
    /// admitted under rules nobody can reconstruct — and the log records no
    /// rule, so the change would be invisible in the artifact afterwards.
    /// Changing the policy means constructing an anchor with the new one.
    pub fn policy(&self) -> &P {
        &self.policy
    }
}

/// The reads. **Nothing in this block signs, and nothing in it admits**, which
/// is why it names no bound on `K` or `P`: the methods here are reachable on a
/// read-only anchor, and the compiler is what says so rather than a comment.
impl<S: LeafStore, K, P> Anchor<S, K, P> {
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

    /// The RFC 6962 Merkle Tree Hash of the log as it stands, together with the
    /// leaf count it was computed at — [`RootHash`] carries both, so the two
    /// cannot be reported out of step.
    ///
    /// **Reading the root signs nothing.** Before this existed the only route
    /// to an anchor's current root was
    /// [`publish_checkpoint`](Self::publish_checkpoint), which signs a note, so
    /// a command that merely wanted to *report* the root emitted a signed
    /// artifact as a side effect. That inverts the division
    /// `crates/lys/src/commands/log/store.rs` records — observing your own
    /// append-only log should not require the ability to sign for it — and the
    /// inversion is why this accessor is on the unbounded block: a value that
    /// cannot sign can still be asked what its root is.
    ///
    /// This is not a substitute for a checkpoint and cannot be handed to a
    /// third party as evidence of anything. A root nobody signed is a number
    /// the holder read off their own disk; what makes it a claim is
    /// `publish_checkpoint`, and a verifier should be given that instead.
    ///
    /// Read through to the tree on every call, like every other accessor here.
    pub fn root(&self) -> RootHash {
        self.log.tree().root()
    }

    /// The bytes stored at `leaf_index`, or `None` if the log holds no such
    /// index.
    ///
    /// Verbatim: exactly what was appended, not canonicalized and not parsed.
    /// This is the read side a receipt needs — `verify_receipt` takes the leaf
    /// as well as the receipt, so an anchor that could issue one but never show
    /// the bytes it was over would be handing out artifacts nobody holding only
    /// the anchor could check.
    ///
    /// Read through to the log on every call. Nothing is cached here, so this
    /// cannot disagree with storage.
    pub fn leaf_bytes(&self, leaf_index: u64) -> Option<&[u8]> {
        self.log.leaf_bytes(leaf_index)
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
}

#[cfg(test)]
#[path = "open_tests.rs"]
mod tests;
