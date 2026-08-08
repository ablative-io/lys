//! `lys-anchor` — a transparency anchor over a durable append-only log.
//!
//! An anchor is a log that publishes about itself. This crate holds the part
//! that is neither a cryptographic primitive (`lys-core`) nor durable storage
//! (`lys-log-store`): the object that owns one log, keeps its invariants, and
//! will later sign about it.
//!
//! # The invariants this crate holds
//!
//! - **An anchor's log always has a leaf 0, and it is put there at creation or
//!   never.** [`Anchor::create`] appends the caller's genesis bytes as leaf 0;
//!   [`Anchor::open`] refuses a log that has none. The refusal is not fussiness:
//!   `lys-core`'s receipt signing declines a tree of size 1 (RFC 9942 types an
//!   inclusion path as one-or-more, and a one-leaf tree's path is empty), so an
//!   anchor without a genesis leaf cannot issue a receipt for its first real
//!   entry — and `LeafStore` has no `insert`, no `rewrite` and no `fork`, so
//!   leaf 0 can never be supplied afterwards. An anchor initialised without one
//!   is not repairable, which is why this is checked at open rather than at
//!   first use.
//! - **The origin is read through to storage and never held here.** An anchor's
//!   origin is whatever the store was created with; [`Anchor::origin`] forwards
//!   to `Log::origin`, which forwards to `LeafStore::origin`, fixed immutably
//!   when a store was created. This crate contains no origin constant, no
//!   default and no fallback, and [`AnchorConfig`] has no origin field — there
//!   is nowhere for such a value to live, which is how the rule is kept
//!   structurally rather than by discipline.
//! - **A repair discovered at open is returned, never printed and never
//!   dropped.** [`Anchor::recovered_to`] forwards `Log::recovered_to` for the
//!   reason already written into the CLI: a library that writes to stderr has
//!   decided for its caller how a repair gets reported, and a silently repaired
//!   log is indistinguishable from one that never needed repairing.
//! - **A checkpoint is signed under the log's own origin, and carries no
//!   time.** [`Anchor::publish_checkpoint`] emits an ordinary C2SP signed note
//!   over the current root; the signed-note key name is the origin, which
//!   `lys-core`'s `verify_checkpoint` binds, so one log's checkpoint can never
//!   be accepted for another. Publishing does not append: an anchor does not
//!   log its own checkpoints.
//! - **A submitted statement is appended verbatim and never interpreted.**
//!   [`Submission`] carries the bytes and nothing else — no content type, no
//!   producer, no kind — because an anchor that knew what a statement meant
//!   would have taken a position on which statements are well-formed, and its
//!   receipts would be endorsements rather than observations. Two identical
//!   submissions are two events at two indices with two receipts: recognising
//!   a repeat would mean deciding that two byte strings mean the same thing,
//!   which is precisely the judgement this crate does not make. `submit` and
//!   the receipt it returns are behind the off-by-default `unstable-anchor`
//!   feature, which forwards `lys-core`'s gate on the draft receipt format.
//! - **The JSON proof of inclusion is *not* behind that gate, and that is a
//!   property rather than a convenience.** [`Anchor::inclusion_artifact`] emits
//!   the `lys/log-inclusion-proof/v1` artifact — an RFC 6962 path plus a signed
//!   checkpoint over the root it leads to — from `lys_core::tlog`, which is
//!   ungated. Gating it would have meant the artifact any stock tooling can
//!   verify is opt-in while the draft binary receipt is what a default build
//!   gets, inverting the rule that verification must outlive the vendor. The
//!   shape consumers get by default therefore includes the JSON proof and
//!   excludes the draft receipt, which is checkable by building rather than by
//!   reading. An artifact and a receipt taken at two tree sizes describe two
//!   different moments and will disagree about size and root; both are correct,
//!   and [`anchor::artifact`] says what a caller who needs them to agree must
//!   do.
//!
//! # The signer boundary is reserved, not usable, and that is stated on purpose
//!
//! An anchor signs through [`Signer`], never through a key its callers hold.
//! **But a remote signer — HSM, KMS, anything off-process — can implement
//! [`Signer`] today and still cannot drive this crate**, because `lys-core`'s
//! signing entry points take a concrete `Ed25519Identity`. The gap is carried
//! as the [`InProcessSigner`] bound rather than as a promise, so the compiler
//! refuses the swap instead of an integration discovering it. [`keys::signer`]
//! names exactly what would have to change and what must not be done instead.
//!
//! # The limit of a standalone anchor, stated here because it cannot be fixed here
//!
//! **An anchor with no witnesses can equivocate undetectably. No local check
//! catches it.** It can hold two histories and show each observer whichever
//! suits, and nothing in its own storage, its own pin, or its own signed
//! artifacts detects that. This is not a gap awaiting better local checking — it
//! cannot be closed locally at all, for the reason already written into the
//! storage layer: an actor able to rewrite both the leaves and the pin can
//! present a consistent shorter log, and no purely local check can catch that.
//!
//! What changes it is one external party keeping its own durable memory of this
//! anchor's checkpoints. Nothing in this crate substitutes for that.

pub mod anchor;
pub mod config;
pub mod error;
pub mod keys;
pub mod wire;

pub use anchor::{Anchor, PublishedCheckpoint, proof_nodes};
pub use config::AnchorConfig;
pub use error::{AnchorError, AnchorResult};
pub use keys::{FileSigner, InProcessSigner, Signer};
pub use wire::Submission;
#[cfg(feature = "unstable-anchor")]
pub use wire::SubmissionOutcome;
