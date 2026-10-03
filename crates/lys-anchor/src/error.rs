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
//! [`ProofError::NoSuchLeaf`] names the tree size in its message. That is not
//! a leak either, and not because the number is unimportant: the tree size is
//! the second line of every checkpoint the anchor signs and hands out. A
//! refusal cannot disclose what the artifact it exists to support already
//! publishes. `Anchor::inclusion_artifact` takes the same stranger-choosable
//! index and reuses that same refusal for the same reason; its own variant,
//! [`ProofError::InclusionArtifact`], is not a verdict on anything a caller
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
//! # The federation variant, judged against the same constraint
//!
//! `CascadeJoinMismatch` — named in plain text here rather than linked, because
//! a link from these ungated docs to a gated item resolves under
//! `--all-features` and breaks the default `cargo doc`, which is a gate —
//! exists only with the `federation` feature, and is detailed. It was put to
//! the rule above rather than pattern-matched onto the variants beside them,
//! and it passes it for a reason that is about *who drives the path*
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
//! that does not join. This variant does not soften that. It is what the
//! operator is told about a bundle their own code declined to emit; a bundle
//! that reaches a verifier gets the one collapsed answer, unchanged.
//!
//! It is `#[cfg(feature = "federation")]` because the default build cannot
//! produce it: with federation off there is no `upward` module and no call
//! site, and a variant nothing can construct is a promise in the error surface
//! consumers actually get.

use lys_log_store::StoreError;

#[cfg(feature = "federation")]
mod cascade;
mod genesis;
mod proof;
mod signing;

#[cfg(feature = "federation")]
pub use cascade::CascadeError;
pub use genesis::GenesisError;
pub use proof::ProofError;
pub use signing::SigningError;

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

    /// The anchor's genesis leaf could not be written, or does not hold what
    /// it must; the family names which.
    #[error(transparent)]
    Genesis(#[from] GenesisError),

    /// The anchor could not load its key or produce a signature; the family
    /// names which.
    #[error(transparent)]
    Signing(#[from] SigningError),

    /// A receipt, an inclusion path or an inclusion artifact could not be
    /// produced; the family names which.
    #[error(transparent)]
    Proof(#[from] ProofError),

    /// A verification bundle could not be assembled from a cascade; the
    /// family names which.
    #[cfg(feature = "federation")]
    #[error(transparent)]
    Cascade(#[from] CascadeError),
}

/// Convenience alias for `Result<T, AnchorError>`.
pub type AnchorResult<T> = Result<T, AnchorError>;
