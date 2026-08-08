//! Key delegation: the `lys/delegation/v1` tagged `COSE_Sign1` artifact
//! (RFC 9052) by which the holder of an **offline root key** states that
//! another key holds a role for a **typed subject**.
//!
//! A delegation says exactly one thing: *"the holder of root key `K_root`
//! states that, for subject `S` of kind `K`, key `K_op` holds role `R` from
//! time `T` onward, at sequence `N`."* It is a single signed claim, never
//! mutated — revocation is an append of a later, superseding entry.
//!
//! # The subject is typed, and the type is half the security content
//!
//! The subject is an **opaque typed string**: an anchor passes a *domain* (a
//! DNS-style origin), a seat consumer passes a *seat identifier*. Both kinds
//! ship in `v1` because unknown kinds are refused, so adding the second one
//! later would be a compatibility event rather than an extension.
//!
//! **The `typed` half is load-bearing and a bare string would have lost a
//! security property.** With a fixed single meaning, "this is a domain, and a
//! lapsed domain orphans every proof" was a property of the artifact. As a bare
//! string, an anchor could pass a seat identifier or a consumer a domain and
//! *nothing would notice* — the delegation would verify perfectly and mean
//! something no verifier could pin down. So the kind rides beside the value,
//! [`verify_delegation`] requires the caller to **name the kind it expects**, and
//! cross-kind confusion is refused at decode rather than discovered downstream.
//! That is the discipline this crate applies *between* COSE artifact kinds,
//! applied *within* one.
//!
//! Two rules make that structural rather than documented, and both live in
//! [`DelegationSubjectKind`]:
//!
//! - **`(subject_kind, role)` is validated as a PAIR** — `(1 domain,
//!   2 operational)` and `(2 seat, 3 speaks-for)` are the only pairs `v1`
//!   defines, and `(domain, speaks-for)` / `(seat, operational)` are refused
//!   despite being made of individually valid values.
//! - **The two vocabularies are offset so that no valid pair has
//!   `subject_kind == role`.** Both are `uint`s in one map; numbering roles from
//!   `1` would have made the two fields encode to the same byte in every valid
//!   artifact, so an implementation that transposed them would be
//!   byte-identical and undetectable. Offset, a transposition lands outside the
//!   pair table and is refused. See [`DelegationSubjectKind::permits`] — and do
//!   not renumber [`DelegationRole::Operational`] to `1` to make the table look
//!   tidier.
//!
//! ⚠️ **[`DelegationRole::SpeaksFor`] has defined semantics, no implementation
//! and no consumer.** Nothing reads it. It is here because adding it later would
//! be a compatibility event; do not invent a consumer for it.
//!
//! # ⛔ `sequence` orders delegations. Log position cannot, and assuming it could was exploitable
//!
//! This is the most important paragraph in the module, because the rule it
//! states replaced the opposite rule, and the opposite rule looked obviously
//! right.
//!
//! Every payload field is chosen by the signer and Ed25519 is deterministic, so
//! **two issuances of the same claim are byte-identical** — the crate's own
//! `issuance_is_deterministic` test establishes that for an unrelated reason.
//! Under "ordered by position in the append-only log, last wins":
//!
//! 1. Leaf 0: root `R` delegates to `K1`.
//! 2. `K1` is compromised, so the operator appends leaf *N*: `R` delegates to
//!    `K2`. Revocation is an append; leaf 0 is still there.
//! 3. An attacker **holding no key material whatsoever** appends leaf *N+1*, a
//!    verbatim copy of leaf 0. Those bytes are public — they are *in the log*,
//!    which is the entire point of the log.
//! 4. The revoked `K1` is current again.
//!
//! A replay and a legitimate re-delegation to `K1` produce identical bytes, so
//! **no fold over the log can distinguish them**. There is nothing to detect.
//!
//! With `sequence` (payload label 6, strictly increasing per
//! `(subject_kind, subject_value, role)`),
//! a replay carries a number already superseded and loses, and a replay of the
//! *current* delegation is byte-identical to what is already current and changes
//! nothing. **Replay becomes a no-op rather than a rollback.**
//!
//! The log still establishes existence and publication — "the key history *is*
//! log history" survives intact. It is simply not the sort key. Ordering by a
//! monotonic value the *trusted offline signer* chose is what a certificate
//! serial number has always been.
//!
//! ## The obligation this places on a fold, which this format cannot discharge
//!
//! A fold over the log sees more than one delegation, so it owes two rules that
//! a single artifact cannot express. **They differ on one word, and getting that
//! word wrong hands the attacker back the capability `sequence` just removed:**
//!
//! - **Byte-identical payloads at the same
//!   `(subject_kind, subject_value, role, sequence)` ⇒ ignore as a duplicate.**
//!   This is the replay, and it is *supposed* to be a no-op.
//! - **Differing payloads at the same
//!   `(subject_kind, subject_value, role, sequence)` ⇒ REFUSE.**
//!   That is root-key equivocation: the root key has said two incompatible
//!   things at one position. Not "prefer the earlier", not "prefer the longer
//!   chain" — refuse. A derived view may refuse on its own authority and must
//!   never permit on it, and refusing turns a duplicated sequence into a
//!   *detected fault* instead of an ambiguity resolved by luck.
//!
//! ### Why the first branch exists, and what happens without it
//!
//! **The obvious rule — "more than one entry at this key ⇒ refuse" — is a
//! denial of service that the same keyless attacker can mount.** A replay is
//! byte-identical by construction; that is the property this whole section
//! exists because of. So a fold that dedupes on the sequence key alone and
//! refuses on any collision can be permanently jammed by re-appending the
//! *current* delegation, whose bytes are public.
//!
//! This is the **second-order form of the very defect the review had just
//! fixed**: the attacker keeps exactly the same capability — append public bytes
//! — and only the outcome moves, from rollback to denial. It is the trap anyone
//! re-deriving the rule will fall into, which is why both branches are written
//! out rather than left to be inferred from "distinct".
//!
//! Nothing in this module enforces either branch, and nothing in it can: a
//! single delegation cannot exhibit monotonicity or equivocation, both of which
//! are properties of a set. What this module *does* enforce is
//! `sequence <= u64::MAX - 1`, so that a successor always exists — see
//! [`DelegationClaim::sequence`].
//!
//! ## Two questions deliberately left open, and when they must be answered
//!
//! Recorded here because both become *harder* to answer later, not easier:
//!
//! - **Whether the counter is shared across roles or restarts per role must be
//!   decided WITH the `v2` that adds a role to an existing kind, never after
//!   it.** `sequence` is specified as increasing per
//!   `(subject_kind, subject_value, role)`, and each kind `v1` defines has
//!   exactly **one** permitted role, so today the partition is effectively
//!   per-subject and the distinction is untestable. The moment a kind gains a
//!   second role, the question is about *already-signed* `v1` entries whose
//!   sequences were allocated under a one-role-per-kind regime — a compatibility
//!   question about frozen bytes, which is the worst kind to answer
//!   retroactively.
//! - **Nothing marks a genesis delegation.** `sequence = 0` is a convention, not
//!   a claim the format can make. A fold reading from a non-zero log offset
//!   therefore cannot tell whether it has seen the true minimum. This is not
//!   attacker-reachable on its own — forging any delegation needs the root key —
//!   but it constrains any partial-log verifier, and it interacts with
//!   genesis-as-delegation.
//!
//! # Invariants
//!
//! - **The artifact is the only durable form.** [`AnchorDelegation`] implements
//!   no `serde`; the wire shape is exactly the tagged `COSE_Sign1` emitted by
//!   [`AnchorDelegation::to_cose_bytes`] — protected headers
//!   `{1: -8 (EdDSA), 3: "application/vnd.lys.delegation.v1+cbor",
//!   4: <raw 32-byte root key>}`, an **empty** unprotected map, an **embedded**
//!   payload `{1: subject_kind, 2: subject_value, 3: delegated key, 4: role,
//!   5: not_before_unix_ms, 6: sequence}`, and a 64-byte Ed25519 signature over
//!   the RFC 9052 §4.4 `Sig_structure` with `external_aad = h''`.
//! - **The unprotected header is empty and a non-empty one is refused.** See
//!   below; this is a requirement, not a default.
//! - **Canonical encoding is normative on the wire.** RFC 8949 §4.2 core
//!   deterministic encoding is a requirement *on the artifact*, not a
//!   by-product of how this crate verifies: RFC 9052 §9 does not constrain map
//!   key ordering, so without the format saying so a conforming stranger would
//!   accept a permuted map as a second valid artifact for one statement.
//!   [`AnchorDelegation::from_cose_bytes`] rejects any input that is not
//!   byte-identical to the canonical re-encoding of its parsed fields — even
//!   inputs whose signature is cryptographically valid.
//! - **Non-oracle verification, in the return value *and* in the work done.**
//!   Every failure collapses to the single
//!   [`TrustError::DelegationVerification`] value, and [`verify_delegation`]
//!   additionally runs its signature check unconditionally so that a wrong root
//!   key, a wrong subject and a bad signature cost the same. A collapsed error
//!   type alone is not a non-oracle; see [`sign`].
//! - **Verification names the root key, the subject KIND and the subject VALUE
//!   it expects.** [`verify_delegation`] takes all three as required arguments.
//!   There is no unattributed verify; see [`sign`]. The kind is not redundant
//!   with the value: a seat identifier is text chosen elsewhere, so a value-only
//!   verifier can be handed a seat delegation whose identifier equals the origin
//!   it cares about, and that collision is an attacker's choice rather than an
//!   accident.
//! - **Unknown roles and unknown subject kinds are refused at decode**, not
//!   carried, and so is any `(subject_kind, role)` pair outside the table. See
//!   [`DelegationRole`] and [`DelegationSubjectKind`].
//! - **Every constraint the decoder enforces is refused at encode too** — a
//!   non-empty subject value, a `(subject_kind, role)` pair this version defines,
//!   a delegated key strict Ed25519 could accept, and the artifact size cap. An issuing path that can emit what the verifying path
//!   refuses is a defect, and it was one.
//!
//! # Why the payload is embedded, when a receipt's is detached
//!
//! A receipt detaches its payload because the verifier does not read the signed
//! value — it *recomputes* a Merkle root from leaves it already holds, and the
//! signature is checked against what the verifier derived. There is no
//! equivalent here. A delegation's payload is an **assertion**: nobody can
//! recompute "this key holds this role for this subject from this time" from
//! anything else they hold. An assertion nobody can independently derive must
//! travel with the signature that carries it, so the payload is embedded and
//! signature-covered.
//!
//! # Why the unprotected header must be empty
//!
//! An unprotected header is unsigned by definition. A receipt can safely put
//! its inclusion proof there because the proof is checked by reconstruction, so
//! tampering with it produces a root the anchor never signed — it is
//! authenticated by consequence. A delegation has no such consequence-check:
//! anything in its unprotected bucket would be attacker-controllable data
//! sitting inside an artifact whose entire purpose is to be trusted, next to
//! signed data, with nothing to distinguish the two once a consumer has read
//! the whole COSE message. Refusing a non-empty bucket is what makes "there is
//! nothing unsigned in here" a property rather than a hope.
//!
//! # What this format deliberately does not answer
//!
//! Recorded here so their absence cannot later read as "nearly done":
//!
//! - **Which delegation is current.** That is a fold over the log, and the
//!   key-history artifact that would carry such a fold is its own future
//!   format. Nothing in this one is an ordering key — in particular
//!   `not_before_unix_ms` is a number the signer chose and must never be used
//!   to order two delegations. Two delegations sharing a `not_before`, or a
//!   later one bearing an earlier `not_before`, are both perfectly well-formed.
//! - **Whether the root key is trustworthy.** Nothing attests it. A verifier is
//!   always *told* whom to trust, or we have rebuilt a certificate authority.
//!   That is why [`verify_delegation`] cannot be called without naming a root
//!   key.
//! - **Expiry.** There is no `not_after`. Freshness tolerance is an input to
//!   the verify decision with no default, and an expiry baked into the artifact
//!   would be exactly that missing default frozen into bytes that outlive the
//!   decision.
//!
//! # Domain separation from the other lys COSE artifacts
//!
//! A delegation's signing preimage begins `0x84 0x6A "Signature1"` —
//! **identical** to a receipt's, a consistency receipt's and a
//! `lys/attestation/v2` attestation's, because all four are `COSE_Sign1`.
//! Byte-0 disjointness does not separate them and it would be a mistake to
//! re-derive that comfort here. What separates them is the content type inside
//! the signed bytes, together with each verifier pinning *its own* constant
//! rather than reading the wire's. See [`sign`].
//!
//! [`TrustError::DelegationVerification`]: crate::error::TrustError::DelegationVerification
//!
//! # Availability
//!
//! Behind the off-by-default `unstable-anchor` feature, and **exempt from
//! semantic versioning** until the format is ratified. Publishing a crate that
//! exposes a format is one of the two things that freezes it — the other is
//! signing a durable artifact under its tag — and this one is still a draft
//! with no production anchor to issue under it. No delegation may be signed
//! outside tests until it is.

pub mod artifact;
mod encoding;
pub mod sign;

pub use artifact::{AnchorDelegation, DelegationClaim, DelegationRole, DelegationSubjectKind};
pub use sign::{assemble_delegation, delegation_preimage, sign_delegation, verify_delegation};
