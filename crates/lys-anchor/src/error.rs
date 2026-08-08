//! [`AnchorError`] — the anchor's error type.
//!
//! # Every failure named here is about the operator's own log
//!
//! `lys-core` deliberately collapses *verification* failures into
//! indistinguishable variants, so an attacker probing a verifier learns nothing
//! about which check rejected them. Nothing in this type is reachable that way:
//! each variant reports on local state the operator already owns and could read
//! off their own disk, exactly as `StoreError` argues for itself. Detail is the
//! product here.
//!
//! **That is a property of the current surface, not a licence for the next
//! variant.** The moment this crate grows a path a stranger can drive — a
//! submission, a verification, anything taking bytes from someone else — a
//! distinguishable refusal on that path is a parsing oracle, and the variant
//! that carries it must collapse instead. The constraint is written down here
//! so a future addition is judged against it rather than pattern-matched onto
//! the variants below.
//!
//! # The submit path arrived, and was judged against that constraint
//!
//! `Anchor::submit` takes bytes from a stranger, so the paragraph above is now
//! live rather than anticipatory. Its refusals are still detailed, and the
//! reason is specific rather than a plea for exemption: **no variant reachable
//! from `submit` is a function of the submitted bytes.** The anchor does not
//! parse, validate, size-check or otherwise decide anything about a statement
//! — it appends it — so there is no verdict on the stranger's input for a
//! distinguishable refusal to disclose. What the path can report is that
//! storage failed, or that the log is too small to produce a conforming
//! receipt: facts about the operator's own machine that are identical for
//! every possible submission.
//!
//! `Anchor::receipt_for` does take a stranger-choosable index, and
//! [`AnchorError::NoSuchLeaf`] names the tree size in its message. That is not
//! a leak either, and not because the number is unimportant: the tree size is
//! the second line of every checkpoint the anchor signs and hands out. A
//! refusal cannot disclose what the artifact it exists to support already
//! publishes. `Anchor::inclusion_artifact` takes the same stranger-choosable
//! index and reuses that same refusal for the same reason; its own variant,
//! [`AnchorError::InclusionArtifact`], is not a verdict on anything a caller
//! supplied at all — it reports that this anchor failed to build a proof about
//! its own tree.
//!
//! **This reasoning expires with the first variant whose outcome depends on a
//! statement's content, and one is already scheduled.** An `AdmissionPolicy`
//! refusal *is* a verdict on the submitted bytes, and a distinguishable one
//! hands a submitter an oracle for the policy — probe until the message
//! changes, and the rule has been read out without ever being published. When
//! that variant is written it must collapse: one refusal for every reason a
//! submission was not admitted, with the detail going to the operator's log
//! and not to the caller. This note is here so that is decided before the code
//! is, rather than argued about after.
//!
//! # The expiry arrived, and this is how it was honoured
//!
//! `AdmissionPolicy` now exists (DP23), so the paragraph above is spent rather
//! than anticipatory. [`AnchorError::NotAdmitted`] is the variant it predicted,
//! and it was built to the constraint written above it:
//!
//! - **It has no fields.** Not a reason, not a policy name, not a size, not an
//!   origin, not a source error. Every refusal by every policy — a length rule,
//!   an absent credential, a certificate from the wrong authority, a subject
//!   key that is not on the list — arrives as the same value with the same
//!   `Display` and the same `Debug`. A submitter cannot learn which rule
//!   refused them, and cannot learn which policy the anchor is running.
//! - **The collapse is structural, not disciplinary.** The anchor is never told
//!   why. `AdmissionPolicy::admit` returns
//!   [`NotAdmitted`](crate::admission::NotAdmitted), a zero-sized value with
//!   nowhere to put a cause, so there is no reason-carrying value crossing into
//!   this crate that a later "just add the source" change could pick up. The
//!   thing that would have to be leaked does not exist here.
//! - **The operator's detail lives with the policy, which is the operator's
//!   object.** They construct it; an implementation that wants to record its
//!   own refusals does so in its own state, through whatever its deployment
//!   already uses. This crate ships no logging framework and deliberately
//!   invents none — a reason routed through the anchor is a reason living one
//!   edit away from the error type that must not carry it. **This version
//!   offers no operator channel of its own**, and that is a statement of what
//!   is missing rather than a claim that nothing is.
//! - **What it does not buy** is stated where the trait is:
//!   [`admission::policy`](crate::admission::policy) names the timing channel
//!   the collapse cannot close.
//!
//! The variants above are unchanged. They remain facts about the operator's own
//! machine, and the argument for their detail was never that the crate had no
//! stranger-driven path — it was that no *variant* was a function of the
//! stranger's bytes. Exactly one now is, and it is the one carrying nothing.
//!
//! # The two federation variants, judged against the same constraint
//!
//! `CascadeTooDeep` and `CascadeJoinMismatch` — named in plain text here rather
//! than linked, because a link from these ungated docs to a gated item resolves
//! under `--all-features` and breaks the default `cargo doc`, which is a gate —
//! exist only with the `federation` feature, and both are detailed. They were
//! put to the rule above rather than pattern-matched onto the variants beside
//! them, and they pass it for a reason that is about *who drives the path*
//! rather than about how interesting the numbers are: **bundle assembly is a
//! producer-side operation on the operator's own artifacts.** A stranger cannot
//! reach `bundle_for` — it takes the operator's anchor, an index into the
//! operator's log, and a cascade the operator built by pinning. Nothing a
//! submitter sends changes which variant comes back or what it says, so there
//! is no verdict on anyone's bytes for the detail to disclose.
//!
//! The reader who checks bundles is a stranger, and *their* refusal is
//! `lys-core`'s single non-oracle `BundleVerification` — deliberately
//! indistinguishable across a malformed container, a bad receipt and a chain
//! that does not join. These two variants do not soften that. They are what the
//! operator is told about a bundle their own code declined to emit; a bundle
//! that reaches a verifier gets the one collapsed answer, unchanged.
//!
//! They are `#[cfg(feature = "federation")]` because the default build cannot
//! produce them: with federation off there is no `upward` module and no call
//! site, and a variant nothing can construct is a promise in the error surface
//! consumers actually get.

use lys_core::error::TrustError;
use lys_log_store::StoreError;

/// Errors returned by an [`Anchor`](crate::Anchor) operation.
///
/// # Stability
///
/// `#[non_exhaustive]`: an anchor that discovers a new precondition must be
/// able to name it without a major version bump. The alternative is pressure to
/// smuggle a new failure into an existing variant's free text, and a failure
/// reported under the wrong name is worse than a new name to match on.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum AnchorError {
    /// The underlying log or its storage refused the operation.
    ///
    /// Transparent because the storage layer's message already names the index,
    /// path or pin involved, and restating it here would only add a second
    /// place for that wording to drift.
    #[error(transparent)]
    Store(#[from] StoreError),

    /// The anchor's admission policy refused the submission.
    ///
    /// **The one variant in this type that is a verdict on a stranger's bytes,
    /// and the only one carrying nothing.** Every reason a submission can be
    /// refused — a length rule, a missing credential, a certificate from
    /// another authority, a subject key that is not on the list, a rule in a
    /// policy this crate has never seen — produces this exact value, with this
    /// exact message. That is not tidiness: a refusal that varied by cause
    /// would let a submitter read the policy out by probing, and with a size
    /// rule it would be a free binary search on the threshold.
    ///
    /// It carries no origin either. The origin is public — it is the first line
    /// of every checkpoint this anchor signs — so naming it would disclose
    /// nothing, and it is still absent, because a field that could vary is a
    /// field a later change can make vary.
    ///
    /// **Nothing was appended.** Admission is decided before the append, so a
    /// refused submission occupies no index and leaves no trace in the log.
    ///
    /// The module docs say where an operator's own detail belongs, since it
    /// does not belong here.
    #[error(
        "the anchor's admission policy did not admit this submission; no further detail is available to a submitter, by design"
    )]
    NotAdmitted,

    /// The log holds no leaves, so it has no genesis leaf and never can.
    ///
    /// **Terminal, not a state to initialize out of.** `LeafStore` offers no
    /// `insert` and no way to rewrite a leaf, so position 0 cannot be filled
    /// after any other entry exists — and it cannot be filled *now* either
    /// without deciding, on the log's behalf, what its first entry says.
    /// [`Anchor::create`](crate::Anchor::create) is where genesis bytes are
    /// supplied, by the caller, once.
    #[error(
        "the log for {origin} has no genesis leaf: an anchor's leaf 0 is written when it is created and can never be inserted afterwards"
    )]
    NoGenesisLeaf {
        /// The origin of the log that was opened, as its store reports it.
        origin: String,
    },

    /// `lys-core` refused to assemble the genesis delegation.
    ///
    /// Reachable two ways, and they are not the same fault:
    ///
    /// - The claim is one `lys-core`'s own decoder would reject — in practice an
    ///   origin long enough to push the artifact past the size cap, since the
    ///   other refusals (an empty origin, an unusable delegated key) are
    ///   unreachable from a store whose origin was validated at creation and a
    ///   signer whose key was validated at load.
    /// - The signature the root signer produced does not verify against the key
    ///   that signer advertises. That is a broken
    ///   [`Signer`](crate::Signer) implementation, and it is caught here rather
    ///   than becoming a leaf 0 nothing can ever verify.
    ///
    /// Detailed rather than collapsed, on the rule the module docs set: creation
    /// is the operator's own act on their own store, with their own key. No
    /// stranger can drive it and nothing about a stranger's bytes is disclosed.
    ///
    /// **Nothing was appended.** The delegation is built and verified before the
    /// log is touched, because leaf 0 cannot be replaced.
    ///
    /// `#[cfg(feature = "unstable-anchor")]` because the delegation format is,
    /// so the default build has no call site and could not construct this.
    #[cfg(feature = "unstable-anchor")]
    #[error("failed to build the genesis delegation for {origin}: {source}")]
    GenesisDelegation {
        /// The origin of the log genesis was being written into, as its store
        /// reports it.
        origin: String,
        /// `lys-core`'s reason for refusing to assemble or to verify it.
        source: TrustError,
    },

    /// Creation was asked to delegate an anchor's operational role **to its own
    /// root key** — the two signers advertise the same public key.
    ///
    /// # Why this is refused rather than merely discouraged
    ///
    /// The artifact it would produce is **perfectly valid and completely
    /// hollow**. It is a well-formed `lys/delegation/v1` domain delegation, it
    /// verifies, its pair is in the table, and it says the offline root key has
    /// delegated the operational role to the offline root key. DP16's entire
    /// reason for two keys — that the key which signs every checkpoint is *not*
    /// the key an operator can keep air-gapped — is void, and **nothing
    /// downstream can tell.** There is no verifier that would flag it, because
    /// there is nothing malformed to flag.
    ///
    /// That lands at leaf 0, which `LeafStore` can never correct: no insert, no
    /// rewrite. So this is refused at the one moment refusing is still possible,
    /// on exactly the argument that fixes the subject kind in the same
    /// constructor — a single mis-passed argument must not be able to make an
    /// anchor permanently mean something other than what it appears to mean.
    ///
    /// # Why the check lives here and not in `lys-core`
    ///
    /// A delegation whose subject delegates to the signing key is a *format*
    /// question, and the format has not ruled on it: there may be subjects for
    /// which self-delegation is meaningful. What is not in question is DP16's
    /// two-key model, and that model lives in this constructor. So
    /// `sign_delegation` and `assemble_delegation` still permit it and this
    /// entry point does not.
    ///
    /// Descriptive rather than collapsed, for the same reason as
    /// [`Self::GenesisDelegation`]: this is an issuing-path fault on the
    /// operator's own store with the operator's own keys, so naming it costs
    /// nothing and saves an operator staring at a valid-looking anchor.
    ///
    /// **Nothing was appended.** The comparison happens before the claim is
    /// built.
    ///
    /// `#[cfg(feature = "unstable-anchor")]` because the delegation format is,
    /// so the default build has no call site and could not construct this.
    #[cfg(feature = "unstable-anchor")]
    #[error(
        "the root signer and the operational signer for {origin} advertise the same public key, \
         so leaf 0 would delegate the operational role to the root key itself: DP16's two-key \
         model would be void in a delegation nothing can distinguish from an honest one, at the \
         one position a log can never correct"
    )]
    GenesisRootKeyIsOperationalKey {
        /// The origin of the log genesis was being written into, as its store
        /// reports it.
        origin: String,
    },

    /// Creation was asked to write genesis into a log that already has entries.
    ///
    /// Appending here would put the genesis bytes at whatever the next free
    /// index happens to be, producing a log whose leaf 0 is something else
    /// entirely while every later check passes. Refused rather than appended:
    /// there is exactly one position genesis can occupy, and it is taken.
    #[error(
        "refusing to write genesis into the log for {origin}: it already holds {tree_size} leaves, and genesis is leaf 0 or nothing"
    )]
    GenesisAlreadyWritten {
        /// The origin of the log that was opened, as its store reports it.
        origin: String,
        /// The number of leaves already present.
        tree_size: u64,
    },

    /// The anchor's signing key could not be loaded from its file.
    ///
    /// The path is carried because `lys-core`'s own reason does not name it —
    /// `std::fs` errors do not include the path they were raised for — and an
    /// operator holding "failed to read identity key: No such file or
    /// directory" has been told everything except the one fact they need.
    ///
    /// **A missing file reaches here rather than being repaired.** Generating a
    /// key for a caller who asked to load one produces an anchor that publishes
    /// under an identity nobody was ever told about, and reports success while
    /// doing it.
    #[error("failed to load the anchor's signing key from {path}: {source}")]
    SignerKey {
        /// The key file path, as it was given.
        path: String,
        /// `lys-core`'s reason for refusing the key file.
        source: TrustError,
    },

    /// A [`Signer`](crate::keys::Signer) declined to produce a signature.
    ///
    /// **This variant exists because the trait promises a failure this enum
    /// could not express.** `Signer::sign` is documented as fallible "for the
    /// remote custody this trait is shaped for, where the network, the device or
    /// the operator's authorization can all decline" — but every other variant
    /// here describes the operator's *own* log or key file, so a remote signer
    /// had no honest name for its own refusal and had to borrow
    /// [`SignerKey`](Self::SignerKey), which reports a key-file problem for
    /// something that is not one.
    ///
    /// The gap stopped being hypothetical when genesis-as-delegation landed
    /// bounded on `Signer` rather than `InProcessSigner`: an offline or remote
    /// root signer is a real, reachable path now, and it is the one signing call
    /// in this crate that a network or a human can refuse.
    ///
    /// `#[non_exhaustive]` on this enum means a downstream implementor **cannot**
    /// add a variant themselves, so a trait whose contract promises a failure
    /// mode obliges the crate that owns the error type to name it. Free text
    /// rather than a structured cause because the reasons are the implementor's
    /// domain — an HSM's refusal, a declined touch, a timeout — and inventing a
    /// taxonomy for devices this crate has never seen would be guessing at
    /// somebody else's failure modes.
    #[error("the signer declined to sign: {reason}")]
    SignerDeclined {
        /// The signer's own account of why it refused.
        reason: String,
    },

    /// The anchor could not sign a checkpoint over its own log.
    ///
    /// Every precondition `lys-core` checks here is already satisfied by
    /// construction — the origin was validated when the store was created, and
    /// the body is machine-generated — so this variant reports something
    /// genuinely unexpected rather than a routine refusal. It is propagated
    /// with its cause instead of being treated as impossible, because a
    /// precondition that "cannot" fail is exactly the one nobody notices
    /// changing.
    #[error("failed to publish a checkpoint for {origin}: {source}")]
    Checkpoint {
        /// The origin the checkpoint was being published for.
        origin: String,
        /// `lys-core`'s reason for refusing to encode or sign it.
        source: TrustError,
    },

    /// A receipt was asked for on a log that holds only its genesis leaf.
    ///
    /// **Not an internal limitation being passed on.** RFC 9942 types an
    /// inclusion path as one-or-more nodes, and the sole leaf of a one-leaf
    /// tree has an empty path, so no conforming receipt exists to issue —
    /// `lys-core` refuses to sign one and is right to. Named here rather than
    /// forwarded because this layer can see the condition coming: it knows the
    /// tree size before it asks, and "your log has nothing in it but genesis"
    /// is the sentence an operator can act on, where a message about CDDL
    /// cardinality is not.
    ///
    /// The remedy is to submit something. The condition disappears at the
    /// first submission and can never return, because a log does not shrink.
    #[error(
        "the log for {origin} holds only its genesis leaf ({tree_size}): a conforming receipt needs a tree of at least two leaves, because a one-leaf tree's inclusion path is empty — submit a statement first"
    )]
    TreeTooSmallForReceipt {
        /// The origin of the log a receipt was requested from.
        origin: String,
        /// The log's size, which is 1 whenever this variant is produced. Kept
        /// as a field rather than written into the text as a literal so the
        /// message reports what was read, not what was expected.
        tree_size: u64,
    },

    /// A receipt was asked for at an index the log does not have.
    ///
    /// Reported with the size rather than as a bare "not found": an index past
    /// the end and an index inside a log that has since been truncated are
    /// very different situations for an operator, and only the size
    /// distinguishes them. Naming it discloses nothing — the size is the
    /// second line of every checkpoint this anchor publishes and signs.
    ///
    /// **Never a reason to append.** The obliging behaviour — log the
    /// statement and hand back a receipt for its new index — would answer a
    /// question about the past with an event in the present, and a caller
    /// asking "what was at index 9" is not asking for index 9 to be created.
    #[error("the log for {origin} has no leaf at index {leaf_index}: it holds {tree_size} leaves")]
    NoSuchLeaf {
        /// The origin of the log that was asked.
        origin: String,
        /// The index that was requested.
        leaf_index: u64,
        /// The log's size when it was asked.
        tree_size: u64,
    },

    /// An inclusion proof's byte encoding was not a whole number of 32-byte
    /// digests.
    ///
    /// A tree emits whole SHA-256 digests, so this cannot arise from a proof
    /// this crate produced — which is the reason it is a named refusal and not
    /// an assumption. The alternative to checking is `chunks_exact` silently
    /// dropping the short tail, and a receipt signed over a silently shortened
    /// path is internally consistent, verifies against itself, and attests to
    /// a root no tree ever held. A malformed proof must be a failure to issue,
    /// never an issued artifact about a fiction.
    #[error(
        "an inclusion proof of {byte_len} bytes is not a whole number of 32-byte digests, so it is not an RFC 6962 path"
    )]
    MalformedInclusionPath {
        /// The length that could not be split into digests.
        byte_len: usize,
    },

    /// The anchor could not prove inclusion of one of its own leaves, or could
    /// not sign the receipt over it.
    ///
    /// Covers both halves of issuing a receipt because both are `lys-core`
    /// refusals about the same request and neither is reachable for an
    /// in-range index on a well-formed anchor: the index was checked against
    /// the log before the proof was requested, and the path handed to the
    /// signer is the one the anchor's own tree produced. Propagated with its
    /// cause rather than treated as impossible — a precondition that "cannot"
    /// fail is exactly the one nobody notices changing.
    #[error(
        "failed to issue a receipt for leaf {leaf_index} of {origin} at tree size {tree_size}: {source}"
    )]
    Receipt {
        /// The origin of the log the receipt was for.
        origin: String,
        /// The index the receipt was requested for.
        leaf_index: u64,
        /// The tree size the receipt would have been issued against.
        tree_size: u64,
        /// `lys-core`'s reason for refusing to prove or to sign.
        source: TrustError,
    },

    /// The anchor could not build the JSON inclusion artifact for one of its
    /// own leaves.
    ///
    /// Separate from [`AnchorError::Receipt`] rather than folded into it,
    /// because the two failures are not the same failure wearing two names.
    /// A receipt fails at proving or at COSE signing; an artifact additionally
    /// fails at the 2^53 JSON-number bound, at checkpoint encoding under the
    /// origin, and — the one that matters — at `lys-core`'s **build-time
    /// self-verification**, which runs the third party's whole verification
    /// path over the artifact before it is returned. An operator told "failed
    /// to issue a receipt" for a self-verification failure has been pointed at
    /// the wrong artifact and the wrong code.
    ///
    /// **Reachable in practice only if this crate is wrong.** The index is
    /// checked against the log first, the leaf bytes handed to the builder are
    /// the ones read back out of that same log at that same index, and the
    /// origin was validated when the store was created. It is propagated with
    /// its cause instead of being treated as impossible, for the reason the
    /// neighbouring variants give: a precondition that "cannot" fail is
    /// exactly the one nobody notices changing.
    #[error(
        "failed to build an inclusion artifact for leaf {leaf_index} of {origin} at tree size {tree_size}: {source}"
    )]
    InclusionArtifact {
        /// The origin of the log the artifact was for.
        origin: String,
        /// The index the artifact was requested for.
        leaf_index: u64,
        /// The tree size the artifact would have been built against.
        tree_size: u64,
        /// `lys-core`'s reason for refusing to build or to self-verify it.
        source: TrustError,
    },

    /// A cascade was handed to bundle assembly with more links than
    /// `lys-core`'s `MAX_LINKS` cap allows.
    ///
    /// Refused at assembly rather than emitted, because the workspace already
    /// knows the answer: `verify_bundle` rejects a bundle past the cap, so
    /// producing one would hand a caller an artifact nothing can accept. The
    /// cap exists so an untrusted bundle cannot ask a verifier for unbounded
    /// work, and a chain this deep is pathological on its own terms — each link
    /// is one anchor notarizing the one below it.
    ///
    /// **`max` is carried as a field rather than written into the text as a
    /// literal**, so the message reports the cap the code actually enforced
    /// instead of the one this sentence remembers.
    #[cfg(feature = "federation")]
    #[error(
        "refusing to assemble a verification bundle for {origin} from a cascade of {links} links: a verifier accepts at most {max}, so a deeper chain would produce an artifact nothing can check"
    )]
    CascadeTooDeep {
        /// The origin of the log the bundle was being assembled for.
        origin: String,
        /// The number of links the cascade held.
        links: usize,
        /// The cap, as read from `lys-core` at the point of refusal.
        max: usize,
    },

    /// The first link of the cascade does not notarize the checkpoint the
    /// freshly built inclusion artifact carries.
    ///
    /// A bundle's first link is what joins the notarization to the inclusion
    /// proof: `verify_bundle` requires `links[0].checkpoint` to be
    /// **byte-identical** to `inclusion_proof.checkpoint`, or the notarization
    /// is about some other log whose checkpoint also happens to verify.
    ///
    /// **The usual cause is an append between the pin and the assembly.** An
    /// inclusion artifact embeds a checkpoint signed over the tree at the moment
    /// it is built, so a statement admitted in between moves the artifact to a
    /// later size while the pinned checkpoint still states the earlier one. Both
    /// notes are valid and neither is corrupt; they are photographs of one log
    /// taken at two moments. The remedy is to pin again from the current tree,
    /// or to assemble the bundle from the artifact taken at the size that was
    /// pinned.
    ///
    /// Both sizes are named because that is what distinguishes this from a pin
    /// against an entirely different log, and neither discloses anything: a
    /// tree size is the second line of every checkpoint this anchor signs and
    /// publishes.
    #[cfg(feature = "federation")]
    #[error(
        "the cascade's first link notarizes a checkpoint of {origin} at tree size {pinned_tree_size}, but the inclusion artifact for leaf {leaf_index} carries one at tree size {artifact_tree_size}: a bundle's first link must notarize the very checkpoint its inclusion proof was verified against"
    )]
    CascadeJoinMismatch {
        /// The origin of the log the bundle was being assembled for.
        origin: String,
        /// The index the bundle was being assembled for.
        leaf_index: u64,
        /// The tree size the pinned checkpoint committed to.
        pinned_tree_size: u64,
        /// The tree size the inclusion artifact's checkpoint committed to.
        artifact_tree_size: u64,
    },
}

/// Convenience alias for `Result<T, AnchorError>`.
pub type AnchorResult<T> = Result<T, AnchorError>;
