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
//!   never.** [`Anchor::create`] appends the caller's genesis bytes as leaf 0
//!   and does not interpret them; `Anchor::create_with_delegated_genesis` —
//!   plain text, because linking from these ungated docs to a gated item breaks
//!   the default `cargo doc` — makes leaf 0 a `lys/delegation/v1`
//!   artifact signed by the anchor's offline root key, which is what DP16 asks
//!   for and is behind `unstable-anchor` because the format is.
//!   **A default-features build has only the uninterpreted form**, and
//!   `anchor::genesis` records why that cannot be collapsed into one
//!   constructor until the format is ratified. [`Anchor::open`] refuses a log
//!   that has none. The refusal is not fussiness:
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
//!   which is precisely the judgement this crate does not make.
//! - **The gate is on the receipt, not on the append.** [`Anchor::append`] is
//!   ungated: it runs the admission policy, adds the leaf, and returns an
//!   [`AppendOutcome`] carrying three numbers and no wire format. `submit` is
//!   `append` followed by `receipt_for`, and *it* is behind the off-by-default
//!   `unstable-anchor` feature, which forwards `lys-core`'s gate on the draft
//!   receipt format. The split was made after the previous arrangement was
//!   measured: with the gate on the verb, **a default-features anchor was
//!   frozen at tree size 1 forever**, so the only leaf the ungated JSON
//!   artifact could describe was genesis, and the claim that ungating it put
//!   DP2 in the feature graph was false. [`anchor::append`] carries the
//!   account.
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
//! - **Admission is an object the operator chooses, and there is no default.**
//!   [`Anchor`] is generic over an [`AdmissionPolicy`]; [`Anchor::create`] and
//!   [`Anchor::open`] both require one and neither has an overload that omits
//!   it. No policy in this crate implements [`Default`] — the absence is
//!   checked by `compile_fail` doctests, not merely intended — so an anchor
//!   cannot come into existence under an admission rule nobody named. That is
//!   DP23: DP9 wants a certificate-gated write path and DP13 wants an anchor
//!   that will sign anything for anyone, and those are reconcilable only if
//!   the gate is a policy the deployment picks rather than behaviour the
//!   library ships.
//! - **What a policy is told about the submitter carries its provenance in the
//!   type.** [`SubmitterContext`] separates bytes the submitter asserted from a
//!   peer a transport authenticated, and the authenticated arm holds an
//!   [`AuthenticatedPeer`], which has no public field and one named
//!   constructor — so claiming an authentication is a line of code that says
//!   so, not a field assignment nobody can see afterwards. It is an
//!   attestation by that caller and not a fact this crate verifies, which
//!   [`admission::context`] states rather than lets the type imply.
//!   [`Submission`] keeps its single field: what a submitter *is* travels
//!   beside the statement, never inside it, and never reaches a leaf.
//! - **Every admission refusal is one indistinguishable value.**
//!   [`AdmissionPolicy::admit`] can return only [`NotAdmitted`], which has no
//!   fields, and `Anchor::submit` turns it into
//!   [`AnchorError::NotAdmitted`], which also has no fields. A submitter
//!   learns neither which rule refused them nor which policy the anchor runs.
//!   This is the one variant in [`AnchorError`] whose outcome depends on a
//!   stranger's bytes, and [`error`] records why it had to be the one carrying
//!   nothing — an admission policy is a function of the submitted bytes, so a
//!   refusal that varied by cause is a read-out of the rule.
//!
//! # The signer boundary works for the root key and is reserved for the rest
//!
//! An anchor signs through [`Signer`], never through a key its callers hold.
//! **A remote signer — HSM, KMS, anything off-process — can implement [`Signer`]
//! today, issue an anchor's genesis delegation, and still not publish a
//! checkpoint or a receipt**, because `lys-core`'s *other* signing entry points
//! take a concrete `Ed25519Identity`. The gap is carried as the
//! [`InProcessSigner`] bound rather than as a promise, so the compiler refuses
//! the swap instead of an integration discovering it. [`keys::signer`] names
//! exactly what would have to change and what must not be done instead.
//!
//! Genesis is the exception because the delegation format was specified with a
//! two-phase preimage-then-assemble API *for* an offline root key, and an entry
//! point designed for absent key material passes a custody boundary for free.
//! That is the shape every other signing path is missing, not a special case.
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
//!
//! **That paragraph is also a value.** [`Anchor::status`] returns an
//! [`AnchorStatus`] whose `posture` field is a [`WitnessPosture`], and
//! `WitnessPosture`'s `Display` is [`STANDALONE_DISCLOSURE`] — the same
//! sentence, whole. A caller cannot obtain a status without obtaining the
//! posture, and cannot display the posture without displaying the disclosure,
//! so it is not a sentence a renderer can quietly drop. [`anchor::status`] says
//! what is and is not yet established by "the posture is computed, never
//! stored".
//!
//! # Reading an anchor does not require the ability to sign for it
//!
//! [`Anchor::open_read_only`] opens an anchor with no signer and no admission
//! policy, and returns an [`Anchor<S, NoSigner, NoPolicy>`](Anchor) on which
//! the signing and appending methods **do not resolve** — [`NoSigner`]
//! implements no signing trait and [`NoPolicy`] no policy trait, and the bounds
//! that gate those methods live on their `impl` blocks. There is no
//! `Option<signer>` for a signerless anchor to travel through a signing path
//! inside; the refusal is the compiler's. [`Anchor::root`] is on the same
//! unbounded surface, so reporting a root no longer requires emitting a signed
//! checkpoint as a side effect.
//!
//! # Witnessing is additive, off by default, and endorses nothing
//!
//! Being that external party for somebody else is the `witness` module, behind
//! the off-by-default `federation` feature. It adds one function, `observe`,
//! and two report types, `Observation` and `Relation`; it adds no wire format,
//! no admission rule, and no artifact.
//!
//! Three properties are held by the shape rather than by this paragraph:
//!
//! - **The default build is the standalone one.** With `federation` off the
//!   module does not exist, so no path described above can reach it. A core
//!   path that named a witness item would fail the default build, which is a
//!   gate — the compiler is the second party, on the only axis the claim is
//!   made on.
//! - **A witness emits nothing a plain recorder could not.** `observe` records
//!   the checkpoint note through `Anchor::submit` and returns that submission's
//!   receipt unchanged. There is no witness receipt type and no field in which
//!   "I also checked this" could be written, so a reader cannot infer
//!   endorsement from a receipt: the receipt does not distinguish the two
//!   cases. The report — what this witness previously recorded, and how the new
//!   checkpoint sits against it — is outside every signature.
//! - **The record precedes the check, unconditionally.** An equivocating
//!   checkpoint is appended and receipted like any other statement, because
//!   equivocation is caught by two durable memories disagreeing and a witness
//!   that refused to hold the second one would have destroyed the evidence.
//!
//! # Pinning upward is the same act from the other side, and it freezes nothing
//!
//! Being the party that *asks* to be remembered is the `upward` module, behind
//! the same feature. `pin` publishes this anchor's checkpoint and submits it to
//! a parent; `bundle_for` packages a leaf, its inclusion proof and the resulting
//! cascade into a `lys/verification-bundle/v1`. DP14 asks that cascading and
//! witnessing be one mechanism rather than two, and they are — the same
//! `Anchor::submit`, over the same bytes, producing the same receipt.
//!
//! Three properties, again held by shape:
//!
//! - **No wire format is introduced.** The leaf a parent records is this
//!   anchor's checkpoint note *verbatim* — an ordinary C2SP signed note. There
//!   is no cascade envelope, no new domain-separation tag and no new leaf
//!   encoding, so nothing in this path can be frozen by a durable append. The
//!   first format this crate freezes is the delegation entry, deliberately
//!   last.
//! - **The parent's `submit` is unchanged and cannot tell.** `pin` passes the
//!   two arguments every submission passes. `Submission` has one field, so
//!   there is nowhere for "this is a cascade" to be written, and the parent's
//!   receipt is therefore identical to any other submitter's — which is the
//!   same structural guard that keeps a witness's receipt from reading as an
//!   endorsement.
//! - **Federation is a caller of the core, never a layer the core is built
//!   through.** `pin` calls `Anchor::submit`; nothing in `Anchor::submit` knows
//!   `upward` exists. A core path that named it would break the default build.
//!
//! An upward pin appends **nothing to this anchor's own log** — publishing is
//! not an append, and the parent's leaf lands in the parent's log — so a pinned
//! anchor still holds no local evidence that anybody remembers it.
//! [`anchor::status`] says what that means for [`WitnessPosture`].

pub mod admission;
pub mod anchor;
pub mod config;
pub mod error;
pub mod keys;
#[cfg(feature = "federation")]
pub mod upward;
pub mod wire;
#[cfg(feature = "federation")]
pub mod witness;

pub use admission::{
    AcceptAll, AdmissionPolicy, AuthenticatedPeer, MaxSize, NotAdmitted, RecognisedCertificate,
    SubmitterContext,
};
#[cfg(feature = "unstable-anchor")]
pub use anchor::GENESIS_SEQUENCE;
pub use anchor::{
    Anchor, AnchorStatus, NoPolicy, NoSigner, PublishedCheckpoint, ReadOnlyAnchor,
    STANDALONE_DISCLOSURE, WitnessPosture, proof_nodes,
};
pub use config::AnchorConfig;
pub use error::{AnchorError, AnchorResult};
pub use keys::{FileSigner, InProcessSigner, Signer};
#[cfg(feature = "federation")]
pub use upward::{UpwardPin, bundle_for, pin};
#[cfg(feature = "unstable-anchor")]
pub use wire::SubmissionOutcome;
pub use wire::{AppendOutcome, Submission};
#[cfg(feature = "federation")]
pub use witness::{Observation, OriginState, Relation, WitnessProjection, observe};
