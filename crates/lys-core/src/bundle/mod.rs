//! The `lys/verification-bundle/v1` container: a whole provenance chain in one
//! file, checkable by a stranger with no lys installed and no service to ask.
//!
//! A bundle carries the artifacts needed to establish that a leaf was logged and
//! that the log's checkpoint was notarized by an anchor — the inclusion proof
//! verbatim, the leaf, and an ordered chain of `{checkpoint, receipt}` links.
//!
//! # Invariants
//!
//! - **It is packaging, and must never become a trust statement.** Every
//!   security property comes from the artifacts inside. The container's only
//!   jobs are to not lose or reorder them, and to make their *relationships*
//!   checkable.
//! - **The bundle is never signed, by absence rather than by convention.** There
//!   is no signature field, no signing function, and nothing in this module ever
//!   holds a private key. A signature over the wrapper would invite verifiers to
//!   check it and skip the contents — a green tick that verified nothing.
//! - **The bundle names no keys.** Trust inputs are supplied to
//!   [`verify_bundle`] by the caller. A container that named the keys to trust
//!   would be asserting who is trustworthy.
//! - **Chain links are mandatory and validated, never assumed.** A pile of
//!   individually valid receipts proves nothing about their relationship. See
//!   [`verify`] for the exact rungs.
//! - **Artifacts are embedded verbatim** — re-encoding a frozen artifact inside
//!   a container is how byte-identity gets lost.
//! - **Non-oracle verification:** every failure collapses to
//!   [`TrustError::BundleVerification`].
//!
//! # What a verified bundle does and does not establish
//!
//! It establishes that the leaf is in a log at a signed checkpoint, and that
//! each anchor in the chain signed a root containing the checkpoint below it.
//!
//! It does **not** establish that any log or anchor is honest, or that any tree
//! is append-only. Detecting a log that rewrote history needs a consistency
//! proof between two checkpoints, or a second witness. What the chain buys is
//! narrower and still the point: a log's owner can no longer keep two histories
//! and show each party whichever suits, because divergence would have to appear
//! in an anchor's tree that the owner does not control.
//!
//! An **empty** chain is a valid bundle asserting a weaker thing — a leaf in an
//! unnotarized log. [`VerifiedBundle::notarizations`] reports it rather than
//! letting a reader assume notarization happened.
//!
//! [`TrustError::BundleVerification`]: crate::error::TrustError::BundleVerification
//!
//! # Availability
//!
//! Behind the off-by-default `unstable-anchor` feature, and **exempt from
//! semantic versioning** until the format is ratified. Publishing a crate that
//! exposes a format is one of the two things that freezes it — the other is
//! signing a durable artifact under its tag — and this one is still a draft with
//! no production anchor to issue under it.

pub mod artifact;
pub mod verify;

pub use artifact::{BundleLink, MAX_LINKS, VERIFICATION_BUNDLE_FORMAT, VerificationBundle};
pub use verify::{Notarization, VerifiedBundle, verify_bundle};
