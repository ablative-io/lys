//! The `lys/delegation/v1` fixed vectors, pinned as literal hex.
//!
//! Three vectors are frozen here — **A** (domain), **B** (seat) and **C** — and
//! two more are specified but deliberately not generated. What each is *for* is
//! recorded below, because a vector whose purpose is not written down is a
//! vector somebody later "simplifies".
//!
//! # This vector was regenerated for the fourth time, and the fourth time matters
//!
//! The values below are the **fourth** generation of vector A. It has been
//! invalidated by adding `sequence`, by the content-type rename, and most
//! recently by the **role renumbering** — and each time it was regenerated from
//! scratch by both parties separately, never copied from one to the other.
//!
//! ⚠️ **[`ROLE_WIRE_VALUE`] is the one worth reading twice**, because it looks
//! like gratuitous churn and is a correctness fix. A draft numbered `domain = 1`
//! *and* `operational = 1`, with the only valid pairs `(1,1)` and `(2,2)` — so
//! `subject_kind == role` for **every valid `v1` artifact**. Both are `uint`s in
//! one map, so an implementation that wired label 1 into its role and label 4
//! into its kind would emit byte-identical output for every valid delegation, and
//! **no vector could ever have caught it** — not this one, not a better-chosen
//! one. Offsetting the roles to `2` and `3` makes a transposition both visible
//! *and* refused, because the swapped pair falls outside the table. **The defect
//! was in the numbering, so the fix had to be too.**
//!
//! That is worth stating in the file it most embarrasses: a golden vector is the
//! strongest instrument here and it is still blind to any defect that leaves the
//! bytes unchanged. It pins what was emitted; it cannot argue that the encoding
//! was capable of expressing the distinction in the first place.
//!
//! # What each vector is FOR
//!
//! Not decoration. Each vector exists because of something the others
//! structurally cannot reach, and deleting one silently reopens exactly that gap.
//!
//! - **A — the domain arm.** `(subject_kind, role) = (1, 2)`, the pair an
//!   anchor's genesis actually writes. It supplies the **two**-argument-byte
//!   integer (`sequence = 300` → `19012c`) and the **eight**-argument-byte
//!   integer (`not_before_unix_ms` → `1b…`), and it is the vector the
//!   `verify_delegation` positive control runs on.
//! - **B — the seat arm**, `(2, 3)`, which no other vector exercises and which is
//!   half the format. It also supplies three things A cannot:
//!   - an **inline `not_before_unix_ms`** (`7`). A's value needs the eight-byte
//!     head *anyway*, so an implementation emitting a fixed `0x1b` head passes A.
//!     This is the case that catches it, and it is the gap A's own rationale
//!     identified and left open.
//!   - `sequence = 24` → `1818`, the exact value at which CBOR stops inlining.
//!     Off-by-one head logic passes at `23` and at `300` and fails only here.
//!   - **no label/value collision anywhere.** `not_before` is `7` rather than `1`
//!     precisely because `1` equals label 1, and a positional decoder reading a
//!     shifted label-value stream is the bug class B exists to expose.
//!
//!   Its `subject_value` is **32 bytes, not 31**, and the reason is structural:
//!   at 31 the payload is 79 bytes, tying the protected bucket, so both `bstr`
//!   heads inside the `Sig_structure` read `584f` and an implementation deriving
//!   one length from the other would be invisible. At 32 the payload is 80
//!   (`5850`). 32 also buys coverage nothing else has — labels 2 and 3 carry
//!   `7820` and `5820`, the **same argument byte under different major types**,
//!   adjacent, so a right-length/wrong-major-type bug is caught within one field
//!   of itself.
//! - **C — the widths A and B jump over.** A and B between them cover zero, one,
//!   two and eight argument bytes and skip **four** entirely, so a broken
//!   four-byte branch passes both: `sequence = 70000` → `1a00011170` closes it.
//!   C also supplies the first *length* heads above one argument byte — a
//!   300-byte `subject_value` (`79012c`) and a payload `bstr` head of `590168` —
//!   because until C the two-argument-byte width had appeared in major type 0
//!   only, so an implementation correct for integers and wrong for lengths was
//!   invisible to both.
//!
//! # Vectors D and E are specified and NOT generated
//!
//! Stated here rather than left to be inferred, because a reader who finds three
//! vectors and no note assumes the set is complete. Specification §6.1.1 defines
//! two more that this file does **not** carry:
//!
//! - **D — genesis.** `sequence = 0`, a 23-byte `subject_value`,
//!   `not_before_unix_ms = 23`. It closes the *low* side of the inline boundary
//!   and is the only vector that would pin the shape an anchor's leaf 0 actually
//!   takes.
//! - **E — `not_before_unix_ms = 2⁶³`.** The value §1.2 argues the `u64` bound
//!   over: wire-legal, and undecodable by any implementation modelling the field
//!   as `i64`. Nothing in this crate tests it as a frozen vector.
//!
//! ⚠️ **And the honest measure of what three vectors buy: head width is a
//! property of a range, but shortest-form is a property of a boundary — the
//! format has ten of them (four each in `not_before_unix_ms` and `sequence` at
//! 23|24, 255|256, 65535|65536 and 2³²−1|2³², plus two in the `subject_value`
//! length, the third being unreachable under the 4096-byte cap), and A+B+C place
//! a value immediately adjacent to exactly one of them — `sequence = 24` — from
//! above only, with no vector on any low side.**
//!
//! # Provenance — where these bytes came from, and why that is the whole point
//!
//! **A golden vector whose provenance is not written down decays into "a number
//! someone once printed."** Its entire authority is that it did not come from
//! the code it checks, and that claim is only worth something for as long as
//! somebody can still tell you how it was produced. So:
//!
//! 1. The values were derived **from the specification and the RFCs**
//!    (RFC 8032, RFC 8949 §4.2, RFC 9052 §4.4) by a third party, using
//!    `openssl` for the Ed25519 key derivation and signature and hand-assembled
//!    CBOR for the encoding. **Not** `ed25519-dalek`, so the signature is not
//!    this crate's signer agreeing with itself, and not this crate's `cbor`
//!    module, so the encoding is not this crate's encoder agreeing with itself.
//! 2. Independently, this crate's encoder produced its own bytes for the same
//!    inputs, without either side seeing the other's output first.
//! 3. The two were compared **programmatically, byte for byte**, on all five
//!    values of each vector — protected header, payload, `Sig_structure`,
//!    signature, and the complete tagged artifact. They matched exactly.
//!
//! Two encoders, two signing implementations, one shared input: the
//! specification. That agreement is what is frozen here.
//!
//! A vector regenerated by one side and copied by the other is one party
//! agreeing with itself, which is the defect this file exists to prevent rather
//! than to demonstrate. So each regeneration repeated step 2 and step 3 in full,
//! and **B and C were generated the same way** — independently on both sides,
//! then compared. An identical Ed25519 signature under one key implies an
//! identical message, so agreement on the signature transitively confirms the
//! preimage, and therefore the protected header and the payload.
//!
//! For the fourth generation the independent party had **no CBOR library and no
//! crypto library**, and hand-wrote both from RFC 8949 §4.2 and RFC 8032. Its
//! Ed25519 was checked against all five RFC 8032 §7.1 test vectors parsed out of
//! the RFC text — derived key, signature, and its own verifier accepting the
//! RFC's signature — before it computed anything here. So the agreement is
//! independent of **implementation and of library**, not merely of author:
//! nothing of `ciborium` or `ed25519-dalek` appears on the other side.
//!
//! # Which axis of independence this is, and which it is not
//!
//! **Encoding, and key derivation.** Derived from the documents, by a different
//! tool, without reading the Rust.
//!
//! **Not platform, not custody, not review.** One machine, one toolchain. And a
//! frozen vector proves the code still emits what it emitted when the vector was
//! taken; it does **not** prove those bytes were the right ones to freeze. That
//! judgement is the specification's and the adversarial review's, and no test
//! can supply it.
//!
//! # Why every byte below is typed out rather than imported
//!
//! A test that imported `CONTENT_TYPE`, or built its expectation by calling the
//! encoder it is checking, moves wherever the code moves. It would be blind to
//! exactly the drift it exists to catch, which is not a hypothetical in this
//! repository: reversing the two public keys in the seal `info` left the entire
//! suite green, and so did changing that construction's domain tag. The fix was
//! `tests/seal_derivation.rs`, whose tag is the byte literal
//! `b"lys-sealed-envelope/v1"` and not the crate's constant. This file is the
//! same register applied to a wire format.
//!
//! Everything here is a constant this file can be **wrong** about, which is the
//! property that makes it worth having.
//!
//! # What one comparison pins
//!
//! A's `HEX_PREIMAGE` alone pins, in a single assertion: the `Sig_structure`
//! array shape and its `"Signature1"` context, the protected map's key order
//! `{1, 3, 4}`, the `alg = -8` encoding, the exact 38-byte content type string,
//! the `kid` being a **bstr** and not a tstr, `external_aad = h''`, the payload
//! map's key order `{1, 2, 3, 4, 5, 6}`, both halves of the `(subject_kind,
//! role)` pair and their **distinct** wire values, and the head widths of
//! `not_before_unix_ms` (eight argument bytes) and `sequence` (two). Any of those
//! changing in `src/delegation/` changes these bytes.
//!
//! What A does **not** pin, stated because the list above reads as exhaustive:
//! the shortest-form rule for `not_before_unix_ms`. A's value needs the
//! eight-byte head anyway, so an implementation emitting a fixed `0x1b` head
//! passes it. **Vector B closes that** with `not_before_unix_ms = 7`, and it is
//! the reason B exists at all.
//!
//! `HEX_ARTIFACT` adds the tag `18` (`0xD2`), the four-element array, and the
//! **empty** unprotected bucket `0xA0`.
//!
//! `HEX_SIGNATURE` adds the one thing the two above cannot: that the private key
//! derivation and the Ed25519 signature itself match a different implementation.
//! Ed25519 is deterministic (RFC 8032), so for one seed and one message there is
//! exactly one correct signature — an agreement here is not a coincidence that a
//! looser verifier could also produce.
//!
//! # The positive control
//!
//! A file made only of equality assertions against literals cannot tell a
//! correct implementation from one that is broken in the same direction as the
//! literals. So every artifact is also **verified**, through `verify_delegation`,
//! against the root key and subject named here — and the reconstructed claim is
//! compared field by field. If the literals and the implementation were wrong
//! together in a way that broke the signature, that check would fail.
//!
//! # Availability
//!
//! Gated with the format it pins: with `unstable-anchor` off there is no
//! `delegation` module and this file compiles to nothing.

#![cfg(feature = "unstable-anchor")]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use lys_core::Ed25519Identity;
use lys_core::delegation::{
    AnchorDelegation, DelegationClaim, DelegationRole, DelegationSubjectKind, assemble_delegation,
    delegation_preimage, sign_delegation, verify_delegation,
};

// ---------------------------------------------------------------------------
// Shared inputs — the seeds and the content type, from spec §1.1 and §6.1.
// ---------------------------------------------------------------------------

/// Root Ed25519 seed: bytes `0x00..=0x1f`. Shared by all three vectors, so every
/// protected header below is byte-identical and each vector isolates its payload.
const ROOT_SEED: [u8; 32] = [
    0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f,
    0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f,
];

/// Delegated Ed25519 seed: bytes `0x20..=0x3f`. Shared by all three vectors.
const DELEGATED_SEED: [u8; 32] = [
    0x20, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27, 0x28, 0x29, 0x2a, 0x2b, 0x2c, 0x2d, 0x2e, 0x2f,
    0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x3a, 0x3b, 0x3c, 0x3d, 0x3e, 0x3f,
];

/// The frozen content type, spec §1.1 — transcribed from the specification table
/// and **never imported** from `encoding::CONTENT_TYPE`.
///
/// This is the literal a stale vector fails against: when the payload gained a
/// typed subject and the type was renamed, a test that imported the constant
/// would have followed it silently. Importing it would make that failure
/// impossible, which is the whole reason it is typed out here.
const CONTENT_TYPE: &str = "application/vnd.lys.delegation.v1+cbor";

// ---------------------------------------------------------------------------
// Vector A — the domain arm. Inputs from the specification's §6.1 table.
// ---------------------------------------------------------------------------

/// Vector A's `subject_value`, for `subject_kind = domain`. `example.test` is
/// a reserved name and deliberately not a production origin: a committed
/// constant naming a real one is precisely what DP15 forbids.
const ORIGIN: &str = "example.test";

/// Vector A's `not_before_unix_ms`. Written as a number, not imported.
const NOT_BEFORE_UNIX_MS: u64 = 1_700_000_000_000;

/// Vector A's role, as the **wire integer**. Compared against
/// `DelegationRole::Operational.wire_value()` so the enum cannot silently
/// re-number itself.
///
/// **`2`, not `1`, and the gap below it is deliberate.** `1` is
/// `DelegationSubjectKind::Domain`'s wire value; the role vocabulary starts
/// above the kind vocabulary so that no valid `(kind, role)` pair has equal
/// halves. See the module docs. Anyone "tidying" this to `1` reintroduces a
/// defect no vector can detect.
const ROLE_WIRE_VALUE: u64 = 2;

/// Vector A's `subject_kind`, as the **wire integer**. Written out for the
/// same reason as the role: the pair is the thing that must not drift, and
/// pinning one half would leave the other free to move.
const SUBJECT_KIND_WIRE_VALUE: u64 = 1;

/// Vector A's `sequence` (spec §6.1). Chosen to expose bugs rather than to
/// look natural: not `0`, which is indistinguishable from a field an
/// implementation forgot to write and defaulted; not `1` or `2`, which
/// `subject_kind` and `role` already carry, so a field swap would be masked; and
/// large enough to need a **two-argument-byte** head, a third width alongside
/// `not_before`'s eight and the two inline enums — so a head-width bug has
/// nowhere to hide.
///
/// `300` is `0x012c`, so the canonical encoding is `19012c`. A draft of the
/// specification once wrote that encoding as `190130`, which is 304: a
/// hand-written literal that was never recomputed. That is precisely the failure
/// mode this file exists to catch, appearing in the document rather than in the
/// code — and it is why `sequence` is asserted here as a *number* whose encoding
/// the frozen hex must contain, never as a hex literal copied from prose.
const SEQUENCE: u64 = 300;

// ---------------------------------------------------------------------------
// Vector B — the seat arm. Inputs from the specification's §6.1.1 table.
// ---------------------------------------------------------------------------

/// Vector B's `subject_value`: a seat identifier, **32 bytes**.
///
/// The length is load-bearing twice over, and neither reason is aesthetic. See
/// the module docs: 31 would tie the protected bucket's length, and 32 puts
/// `7820` beside `5820` — one argument byte under two different major types.
const B_SUBJECT_VALUE: &str = "lys-seat-00000000000000000000001";

/// Vector B's `subject_kind`: `2`, seat. The arm no other vector reaches.
const B_SUBJECT_KIND_WIRE_VALUE: u64 = 2;

/// Vector B's role: `3`, speaks-for. Pair `(2, 3)`.
const B_ROLE_WIRE_VALUE: u64 = 3;

/// Vector B's `not_before_unix_ms`: **`7`**, and every digit of that choice is
/// deliberate.
///
/// It must be **inline**, because vector A's value needs the eight-byte head
/// anyway and so cannot test the shortest-form rule at all. It must not be `1`,
/// because `1` is payload label 1's value *and* label 1's own number, and a
/// positional decoder reading a shifted label-value stream is exactly the bug
/// class this vector exists to expose. `7` is inline, distinct from `2`, `3` and
/// `24`, and collides with no label in the map.
const B_NOT_BEFORE_UNIX_MS: u64 = 7;

/// Vector B's `sequence`: `24`, encoded `1818`.
///
/// The exact value at which CBOR stops inlining an argument. Off-by-one head
/// logic passes at `23` and at `300` and fails only here, which is why the
/// boundary is worth a vector rather than a comment.
const B_SEQUENCE: u64 = 24;

// ---------------------------------------------------------------------------
// Vector C — the widths A and B jump over. Spec §6.1.1.
// ---------------------------------------------------------------------------

/// The length of vector C's `subject_value`, in bytes: 300 ASCII `a` (`0x61`).
///
/// Written as a length rather than as a 300-character literal, because a literal
/// that long is unreadable and its own length unverifiable by eye. The value is
/// still checked against the frozen hex: the cursor walk derives the `tstr` head
/// from *this* number and compares it, so a wrong count fails rather than
/// silently agreeing with itself.
const C_SUBJECT_VALUE_LEN: usize = 300;

/// Vector C's `sequence`: `70000`, encoded `1a00011170` — the **four**-argument-
/// byte width that A and B jump straight over.
const C_SEQUENCE: u64 = 70_000;

// ---------------------------------------------------------------------------
// The vectors' outputs. Literal hex; nothing here is computed from lys-core.
// ---------------------------------------------------------------------------

/// Root public key, which is also the protected `kid`.
const HEX_ROOT_PUBLIC_KEY: &str =
    "03a107bff3ce10be1d70dd18e74bc09967e4d6309ba50d5f1ddc8664125531b8";

/// The public key being delegated to.
const HEX_DELEGATED_PUBLIC_KEY: &str =
    "29acbae141bccaf0b22e1a94d34d0bc7361e526d0bfe12c89794bc9322966dd7";

/// The protected header map, 79 bytes, **without** its bstr wrapper.
const HEX_PROTECTED: &str = "a301270378266170706c69636174696f6e2f766e642e6c79732e64656c6567617469\
6f6e2e76312b63626f7204582003a107bff3ce10be1d70dd18e74bc09967e4d6309ba50d5f1ddc8664125531b8";

/// The embedded payload map, 68 bytes, **without** its bstr wrapper. Six
/// entries: the head is `0xa6`.
const HEX_PAYLOAD: &str = "a60101026c6578616d706c652e7465737403582029acbae141bccaf0b22e1a94d34d\
0bc7361e526d0bfe12c89794bc9322966dd70402051b0000018bcfe568000619012c";

/// The RFC 9052 §4.4 `Sig_structure`, 164 bytes — the bytes the signature
/// covers.
const HEX_PREIMAGE: &str = "846a5369676e617475726531584fa301270378266170706c69636174696f6e2f766e\
642e6c79732e64656c65676174696f6e2e76312b63626f7204582003a107bff3ce10be1d70dd18e74bc09967e4d6309ba\
50d5f1ddc8664125531b8405844a60101026c6578616d706c652e7465737403582029acbae141bccaf0b22e1a94d34d0b\
c7361e526d0bfe12c89794bc9322966dd70402051b0000018bcfe568000619012c";

/// The Ed25519 signature, 64 bytes. Deterministic per RFC 8032, so this is the
/// only correct value for this key and this message.
const HEX_SIGNATURE: &str = "478bf10b8ff704eab09a24efba6728eca75d9924a4cd006b46ba202d9f43fd198e6\
4363a36ad4433918ab9e96956ead3ecb22da4bca69ef95ab0f9ec782f340f";

/// The complete tagged `COSE_Sign1`, 220 bytes.
const HEX_ARTIFACT: &str = "d284584fa301270378266170706c69636174696f6e2f766e642e6c79732e64656c65\
676174696f6e2e76312b63626f7204582003a107bff3ce10be1d70dd18e74bc09967e4d6309ba50d5f1ddc8664125531b\
8a05844a60101026c6578616d706c652e7465737403582029acbae141bccaf0b22e1a94d34d0bc7361e526d0bfe12c897\
94bc9322966dd70402051b0000018bcfe568000619012c5840478bf10b8ff704eab09a24efba6728eca75d9924a4cd006\
b46ba202d9f43fd198e64363a36ad4433918ab9e96956ead3ecb22da4bca69ef95ab0f9ec782f340f";

/// Vector B's protected header, 79 bytes — **byte-identical to A's**, because B
/// changes only the payload. That identity is the point: it isolates every
/// difference below to the fields under test.
const B_HEX_PROTECTED: &str = "a301270378266170706c69636174696f6e2f766e642e6c79732e64656c65676174696f6e2e76312b63626f7204582003a1\
07bff3ce10be1d70dd18e74bc09967e4d6309ba50d5f1ddc8664125531b8";

/// Vector B's payload map, **80 bytes**. Not 79: at 79 it would have tied the
/// protected bucket's length and both `bstr` heads in the `Sig_structure` would
/// have read `584f`, so an implementation deriving one length from the other
/// would have been invisible. See the module docs.
const B_HEX_PAYLOAD: &str = "a601020278206c79732d736561742d303030303030303030303030303030303030303030303103582029acbae141bccaf0\
b22e1a94d34d0bc7361e526d0bfe12c89794bc9322966dd704030507061818";

/// Vector B's `Sig_structure`, 176 bytes.
const B_HEX_PREIMAGE: &str = "846a5369676e617475726531584fa301270378266170706c69636174696f6e2f766e642e6c79732e64656c65676174696f\
6e2e76312b63626f7204582003a107bff3ce10be1d70dd18e74bc09967e4d6309ba50d5f1ddc8664125531b8405850a601\
020278206c79732d736561742d303030303030303030303030303030303030303030303103582029acbae141bccaf0b22e\
1a94d34d0bc7361e526d0bfe12c89794bc9322966dd704030507061818";

/// Vector B's Ed25519 signature, 64 bytes.
const B_HEX_SIGNATURE: &str = "980cbf7397de5032a45e5a065a1e4d18210e8051826cfc3a9baa355c8df1a111e7ce0d2844cca63b4a09a341f422e62618\
1b011f51c0181a7b9466a20e00c402";

/// Vector B's complete tagged `COSE_Sign1`, 232 bytes.
const B_HEX_ARTIFACT: &str = "d284584fa301270378266170706c69636174696f6e2f766e642e6c79732e64656c65676174696f6e2e76312b63626f7204\
582003a107bff3ce10be1d70dd18e74bc09967e4d6309ba50d5f1ddc8664125531b8a05850a601020278206c79732d7365\
61742d303030303030303030303030303030303030303030303103582029acbae141bccaf0b22e1a94d34d0bc7361e526d\
0bfe12c89794bc9322966dd7040305070618185840980cbf7397de5032a45e5a065a1e4d18210e8051826cfc3a9baa355c\
8df1a111e7ce0d2844cca63b4a09a341f422e626181b011f51c0181a7b9466a20e00c402";

/// Vector C's protected header, 79 bytes — identical to A's and B's, for the
/// same reason.
const C_HEX_PROTECTED: &str = "a301270378266170706c69636174696f6e2f766e642e6c79732e64656c65676174696f6e2e76312b63626f7204582003a1\
07bff3ce10be1d70dd18e74bc09967e4d6309ba50d5f1ddc8664125531b8";

/// Vector C's payload map, 360 bytes. Large enough that its own `bstr` head in
/// the envelope needs **two** argument bytes (`590168`), which is the first time
/// any vector exercises a length head above one argument byte.
const C_HEX_PAYLOAD: &str = "a601010279012c616161616161616161616161616161616161616161616161616161616161616161616161616161616161\
61616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161\
61616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161\
61616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161\
61616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161\
61616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161\
6161616161616161616161616103582029acbae141bccaf0b22e1a94d34d0bc7361e526d0bfe12c89794bc9322966dd704\
02051b0000018bcfe56800061a00011170";

/// Vector C's `Sig_structure`, 457 bytes.
const C_HEX_PREIMAGE: &str = "846a5369676e617475726531584fa301270378266170706c69636174696f6e2f766e642e6c79732e64656c65676174696f\
6e2e76312b63626f7204582003a107bff3ce10be1d70dd18e74bc09967e4d6309ba50d5f1ddc8664125531b840590168a6\
01010279012c61616161616161616161616161616161616161616161616161616161616161616161616161616161616161\
61616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161\
61616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161\
61616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161\
61616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161\
61616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161\
61616161616161616161616103582029acbae141bccaf0b22e1a94d34d0bc7361e526d0bfe12c89794bc9322966dd70402\
051b0000018bcfe56800061a00011170";

/// Vector C's Ed25519 signature, 64 bytes.
const C_HEX_SIGNATURE: &str = "5a949d8ba76fe8fb6549b7be732cabbcd6f29934cdf883fac361d6d693375b85b79162844027d4871075dd2328f9db64d1\
2fbf53a53253986d47c2e9ccc07f0e";

/// Vector C's complete tagged `COSE_Sign1`, 513 bytes — comfortably inside the
/// 4096-byte artifact cap, which is what makes the cap's own boundary a separate
/// concern from head widths.
const C_HEX_ARTIFACT: &str = "d284584fa301270378266170706c69636174696f6e2f766e642e6c79732e64656c65676174696f6e2e76312b63626f7204\
582003a107bff3ce10be1d70dd18e74bc09967e4d6309ba50d5f1ddc8664125531b8a0590168a601010279012c61616161\
61616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161\
61616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161\
61616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161\
61616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161\
61616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161\
61616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161\
616103582029acbae141bccaf0b22e1a94d34d0bc7361e526d0bfe12c89794bc9322966dd70402051b0000018bcfe56800\
061a0001117058405a949d8ba76fe8fb6549b7be732cabbcd6f29934cdf883fac361d6d693375b85b79162844027d48710\
75dd2328f9db64d12fbf53a53253986d47c2e9ccc07f0e";

// ---------------------------------------------------------------------------
// Helpers. None of these calls into `lys-core`.
// ---------------------------------------------------------------------------

fn to_hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    bytes.iter().fold(String::new(), |mut acc, byte| {
        let _ = write!(acc, "{byte:02x}");
        acc
    })
}

fn from_hex(hex: &str) -> Vec<u8> {
    assert!(hex.len() % 2 == 0, "odd-length hex literal");
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).expect("hex digit"))
        .collect()
}

/// The identity for a fixed seed, via the public `load` route. The `TempDir` is
/// returned because it must outlive the read.
fn identity(seed: &[u8; 32]) -> (tempfile::TempDir, Ed25519Identity) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("key");
    std::fs::write(&path, seed).unwrap();
    let key = Ed25519Identity::load(&path).unwrap();
    (dir, key)
}

/// The RFC 8949 §4.2 shortest head for `value` under `major`, computed from the
/// **number** — never copied from prose, never taken from `lys_core::cbor`.
///
/// This is a second encoder, and deliberately so: it exists to disagree. It
/// encodes heads only, is confined to this file, and is what turns "the frozen
/// hex is internally consistent" into "the frozen hex is the shortest-form
/// encoding of the specification's table". A head is an **initial byte plus N
/// argument bytes**; `N` is `0`, `1`, `2`, `4` or `8`.
///
/// The additional-information values are written in hex (`0x18`..`0x1b`) rather
/// than as `24`..`27`, because they are bit fields of the initial byte and every
/// head in this file's comments and in the specification is quoted in hex.
fn shortest_head(major: u8, value: u64) -> Vec<u8> {
    let m = major << 5;
    let mut out = Vec::new();
    match value {
        // Additional information 0..=23: the value is the low five bits itself,
        // so the head is a single byte and there are no argument bytes.
        0..=23 => out.push(m | u8::try_from(value).unwrap()),
        24..=0xff => {
            out.push(m | 0x18);
            out.push(u8::try_from(value).unwrap());
        }
        0x100..=0xffff => {
            out.push(m | 0x19);
            out.extend_from_slice(&u16::try_from(value).unwrap().to_be_bytes());
        }
        0x1_0000..=0xffff_ffff => {
            out.push(m | 0x1a);
            out.extend_from_slice(&u32::try_from(value).unwrap().to_be_bytes());
        }
        _ => {
            out.push(m | 0x1b);
            out.extend_from_slice(&value.to_be_bytes());
        }
    }
    out
}

/// Consume `expected` at `at`, or fail naming the field.
fn take(bytes: &[u8], at: &mut usize, expected: &[u8], what: &str) {
    let end = *at + expected.len();
    assert!(
        end <= bytes.len(),
        "{what}: the frozen hex ends before this field"
    );
    assert_eq!(&bytes[*at..end], expected, "{what}");
    *at = end;
}

fn expect_uint(bytes: &[u8], at: &mut usize, value: u64, what: &str) {
    take(bytes, at, &shortest_head(0, value), what);
}

fn expect_text(bytes: &[u8], at: &mut usize, text: &str, what: &str) {
    take(
        bytes,
        at,
        &shortest_head(3, u64::try_from(text.len()).unwrap()),
        what,
    );
    take(bytes, at, text.as_bytes(), what);
}

fn expect_bstr(bytes: &[u8], at: &mut usize, value: &[u8], what: &str) {
    take(
        bytes,
        at,
        &shortest_head(2, u64::try_from(value.len()).unwrap()),
        what,
    );
    take(bytes, at, value, what);
}

// ---------------------------------------------------------------------------
// The frozen vectors.
// ---------------------------------------------------------------------------

/// One frozen vector: the §6.1 table values *and* the hex they must encode to.
///
/// Both halves are literals declared above. The table half is what the hex half
/// is judged against, and neither is derived from the other — that is the whole
/// arrangement, and it is what a previous version of this file lacked.
struct FrozenVector {
    name: &'static str,
    subject_kind: DelegationSubjectKind,
    subject_kind_wire: u64,
    subject_value: String,
    role: DelegationRole,
    role_wire: u64,
    not_before_unix_ms: u64,
    sequence: u64,
    hex_protected: &'static str,
    hex_payload: &'static str,
    hex_preimage: &'static str,
    hex_signature: &'static str,
    hex_artifact: &'static str,
}

impl FrozenVector {
    /// The claim this vector describes, built from the **table** constants.
    fn claim(&self, delegated_public_key: [u8; 32]) -> DelegationClaim {
        DelegationClaim {
            subject_kind: self.subject_kind,
            subject_value: self.subject_value.clone(),
            delegated_public_key,
            role: self.role,
            not_before_unix_ms: self.not_before_unix_ms,
            sequence: self.sequence,
        }
    }
}

fn vector_a() -> FrozenVector {
    FrozenVector {
        name: "A (domain)",
        subject_kind: DelegationSubjectKind::Domain,
        subject_kind_wire: SUBJECT_KIND_WIRE_VALUE,
        subject_value: ORIGIN.to_string(),
        role: DelegationRole::Operational,
        role_wire: ROLE_WIRE_VALUE,
        not_before_unix_ms: NOT_BEFORE_UNIX_MS,
        sequence: SEQUENCE,
        hex_protected: HEX_PROTECTED,
        hex_payload: HEX_PAYLOAD,
        hex_preimage: HEX_PREIMAGE,
        hex_signature: HEX_SIGNATURE,
        hex_artifact: HEX_ARTIFACT,
    }
}

fn vector_b() -> FrozenVector {
    FrozenVector {
        name: "B (seat)",
        subject_kind: DelegationSubjectKind::Seat,
        subject_kind_wire: B_SUBJECT_KIND_WIRE_VALUE,
        subject_value: B_SUBJECT_VALUE.to_string(),
        role: DelegationRole::SpeaksFor,
        role_wire: B_ROLE_WIRE_VALUE,
        not_before_unix_ms: B_NOT_BEFORE_UNIX_MS,
        sequence: B_SEQUENCE,
        hex_protected: B_HEX_PROTECTED,
        hex_payload: B_HEX_PAYLOAD,
        hex_preimage: B_HEX_PREIMAGE,
        hex_signature: B_HEX_SIGNATURE,
        hex_artifact: B_HEX_ARTIFACT,
    }
}

fn vector_c() -> FrozenVector {
    FrozenVector {
        name: "C (wide heads)",
        subject_kind: DelegationSubjectKind::Domain,
        subject_kind_wire: SUBJECT_KIND_WIRE_VALUE,
        subject_value: "a".repeat(C_SUBJECT_VALUE_LEN),
        role: DelegationRole::Operational,
        role_wire: ROLE_WIRE_VALUE,
        not_before_unix_ms: NOT_BEFORE_UNIX_MS,
        sequence: C_SEQUENCE,
        hex_protected: C_HEX_PROTECTED,
        hex_payload: C_HEX_PAYLOAD,
        hex_preimage: C_HEX_PREIMAGE,
        hex_signature: C_HEX_SIGNATURE,
        hex_artifact: C_HEX_ARTIFACT,
    }
}

fn all_vectors() -> Vec<FrozenVector> {
    vec![vector_a(), vector_b(), vector_c()]
}

// ---------------------------------------------------------------------------
// Tests.
// ---------------------------------------------------------------------------

/// Every frozen vector's hex encodes the §6.1 table, in shortest form, and
/// carries nothing else.
///
/// # What this replaced, and why the old version could not fail
///
/// This was `the_frozen_literals_are_self_consistent`, and it passed over a
/// **stale** vector for as long as one existed. Its lengths (`86`, `66`, `169`,
/// `225`), its `0xa5` map head and its `100`/`90` offsets were genuine
/// specification facts rather than restatements of the hex — but nothing forced
/// them to move when the format did, so when the payload gained a typed subject
/// **both sides went stale together** and agreement survived. **A second party
/// that can go stale in lockstep with what it judges is not a second party.**
///
/// So there is no bare number here describing the hex. Every length is derived —
/// from a §6.1 table constant declared separately above, or from an RFC 8949
/// shortest head computed from the value — and each structure is walked with a
/// cursor that must land **exactly** on `len()`, so a field dropped, added,
/// re-typed or re-headed fails rather than going unexamined. Nothing in this
/// test calls `lys-core`.
///
/// Verified rather than argued: run against the pre-typed-subject vector it fails
/// at the content type. The old version passed.
///
/// # One residual hole, documented rather than hidden
///
/// [`expect_bstr`] derives the `bstr` head from the length of the value it is
/// given, so a truncated `HEX_ROOT_PUBLIC_KEY` is caught only because
/// `HEX_PROTECTED` is a *separate* literal still carrying `5820`. If both were
/// truncated identically this would pass. That is two coordinated paste errors
/// rather than one, and `the_seeds_derive_the_public_keys_the_vectors_name`
/// covers the key values independently.
#[test]
fn the_frozen_hex_encodes_the_specification_table_in_shortest_form() {
    let mut checked = 0;
    for v in all_vectors() {
        let name = v.name;
        let protected = from_hex(v.hex_protected);
        let payload = from_hex(v.hex_payload);
        let preimage = from_hex(v.hex_preimage);
        let signature = from_hex(v.hex_signature);
        let artifact = from_hex(v.hex_artifact);
        let root_key = from_hex(HEX_ROOT_PUBLIC_KEY);
        let delegated_key = from_hex(HEX_DELEGATED_PUBLIC_KEY);

        assert_eq!(
            root_key.len(),
            32,
            "{name}: spec §1.1, kid is a bstr .size 32"
        );
        assert_eq!(
            delegated_key.len(),
            32,
            "{name}: spec §1.2, the delegated key is a bstr .size 32"
        );
        assert_eq!(
            signature.len(),
            64,
            "{name}: RFC 8032, an Ed25519 signature is 64 bytes"
        );
        assert_ne!(
            v.subject_kind_wire, v.role_wire,
            "{name}: spec §0.5 consequence 4 — the two vocabularies must not \
             overlap, or a transposition of payload labels 1 and 4 is invisible \
             in these very bytes"
        );

        // ---- protected: {1: -8, 3: <content type>, 4: <kid>}   (spec §1.1)
        let mut at = 0;
        take(
            &protected,
            &mut at,
            &shortest_head(5, 3),
            &format!("{name}: protected map head — three entries"),
        );
        expect_uint(
            &protected,
            &mut at,
            1,
            &format!("{name}: protected label 1 (alg)"),
        );
        take(
            &protected,
            &mut at,
            &shortest_head(1, 7),
            &format!("{name}: alg = -8 (EdDSA): RFC 8949 major 1, argument -1-(-8) = 7"),
        );
        expect_uint(
            &protected,
            &mut at,
            3,
            &format!("{name}: protected label 3 (content type)"),
        );
        expect_text(
            &protected,
            &mut at,
            CONTENT_TYPE,
            &format!("{name}: the frozen content type"),
        );
        expect_uint(
            &protected,
            &mut at,
            4,
            &format!("{name}: protected label 4 (kid)"),
        );
        expect_bstr(
            &protected,
            &mut at,
            &root_key,
            &format!("{name}: kid — a bstr, not a tstr (RFC 9052 §3.1)"),
        );
        assert_eq!(
            at,
            protected.len(),
            "{name}: the protected bucket carries nothing else"
        );

        // ---- payload   (spec §1.2). The kind precedes the value it types.
        let mut at = 0;
        take(
            &payload,
            &mut at,
            &shortest_head(5, 6),
            &format!("{name}: payload map head — six entries"),
        );
        expect_uint(&payload, &mut at, 1, &format!("{name}: payload label 1"));
        expect_uint(
            &payload,
            &mut at,
            v.subject_kind_wire,
            &format!("{name}: subject_kind"),
        );
        expect_uint(&payload, &mut at, 2, &format!("{name}: payload label 2"));
        expect_text(
            &payload,
            &mut at,
            &v.subject_value,
            &format!("{name}: subject_value"),
        );
        expect_uint(&payload, &mut at, 3, &format!("{name}: payload label 3"));
        expect_bstr(
            &payload,
            &mut at,
            &delegated_key,
            &format!("{name}: the delegated public key"),
        );
        expect_uint(&payload, &mut at, 4, &format!("{name}: payload label 4"));
        expect_uint(&payload, &mut at, v.role_wire, &format!("{name}: role"));
        expect_uint(&payload, &mut at, 5, &format!("{name}: payload label 5"));
        expect_uint(
            &payload,
            &mut at,
            v.not_before_unix_ms,
            &format!("{name}: not_before_unix_ms"),
        );
        expect_uint(&payload, &mut at, 6, &format!("{name}: payload label 6"));
        expect_uint(&payload, &mut at, v.sequence, &format!("{name}: sequence"));
        assert_eq!(
            at,
            payload.len(),
            "{name}: the payload carries nothing else"
        );

        // ---- Sig_structure   (RFC 9052 §4.4, spec §1.0)
        let mut at = 0;
        take(
            &preimage,
            &mut at,
            &shortest_head(4, 4),
            &format!("{name}: Sig_structure is a four-element array"),
        );
        expect_text(
            &preimage,
            &mut at,
            "Signature1",
            &format!("{name}: the RFC 9052 §4.4 context string"),
        );
        expect_bstr(
            &preimage,
            &mut at,
            &protected,
            &format!("{name}: the protected bucket, bstr-wrapped"),
        );
        expect_bstr(
            &preimage,
            &mut at,
            &[],
            &format!("{name}: external_aad must be h'' — spec §1.0"),
        );
        expect_bstr(
            &preimage,
            &mut at,
            &payload,
            &format!("{name}: the embedded payload, bstr-wrapped"),
        );
        assert_eq!(
            at,
            preimage.len(),
            "{name}: the Sig_structure carries nothing else"
        );

        // ---- the tagged artifact   (spec §1)
        let mut at = 0;
        take(
            &artifact,
            &mut at,
            &shortest_head(6, 18),
            &format!("{name}: tag 18 is mandatory — spec §1.0"),
        );
        take(
            &artifact,
            &mut at,
            &shortest_head(4, 4),
            &format!("{name}: COSE_Sign1 is a four-element array"),
        );
        expect_bstr(
            &artifact,
            &mut at,
            &protected,
            &format!("{name}: the protected bucket"),
        );
        take(
            &artifact,
            &mut at,
            &shortest_head(5, 0),
            &format!("{name}: the unprotected bucket is the empty map — spec §1.3"),
        );
        expect_bstr(
            &artifact,
            &mut at,
            &payload,
            &format!("{name}: the embedded payload"),
        );
        expect_bstr(
            &artifact,
            &mut at,
            &signature,
            &format!("{name}: the signature"),
        );
        assert_eq!(
            at,
            artifact.len(),
            "{name}: the artifact carries nothing else, and no trailing bytes"
        );

        checked += 1;
    }
    assert_eq!(checked, 3, "every frozen vector must have been walked");
}

/// The two `bstr` heads inside each `Sig_structure` are distinguishable.
///
/// ⚠️ **This is why vector B's `subject_value` is 32 bytes and not 31.** At 31 the
/// payload is 79 bytes — exactly the protected bucket's length — so both heads
/// read `584f`, and an implementation that derived one length from the other, or
/// wrapped the wrong bucket, would produce identical bytes. The specification's
/// "no value collides with another value" rule was written about payload *fields*
/// and did not reach the envelope; this is that rule applied one level up.
///
/// Vector A satisfies it incidentally (79 against 68). B now satisfies it by
/// construction, and this test is what stops a future edit to `B_SUBJECT_VALUE`
/// from quietly reintroducing the tie.
#[test]
fn no_vector_ties_the_protected_and_payload_bstr_heads() {
    let mut checked = 0;
    for v in all_vectors() {
        let protected = from_hex(v.hex_protected);
        let payload = from_hex(v.hex_payload);
        assert_ne!(
            shortest_head(2, u64::try_from(protected.len()).unwrap()),
            shortest_head(2, u64::try_from(payload.len()).unwrap()),
            "{}: the protected and payload bstr heads are identical ({} bytes \
             each), so a bucket swap or a length derived from the wrong bucket \
             would be invisible in this vector",
            v.name,
            protected.len()
        );
        checked += 1;
    }
    assert_eq!(checked, 3, "every frozen vector must have been checked");
}

/// Both key derivations reach the public keys every vector names.
///
/// If this fails, nothing below means anything — the two sides are not talking
/// about the same keys — so it is a test of its own rather than a preamble
/// inside another.
#[test]
fn the_seeds_derive_the_public_keys_the_vectors_name() {
    let (_root_dir, root) = identity(&ROOT_SEED);
    let (_delegated_dir, delegated) = identity(&DELEGATED_SEED);
    assert_eq!(to_hex(&root.public_key_bytes()), HEX_ROOT_PUBLIC_KEY);
    assert_eq!(
        to_hex(&delegated.public_key_bytes()),
        HEX_DELEGATED_PUBLIC_KEY
    );
}

/// The `Sig_structure` this crate builds equals the one derived from the
/// specification independently, for every vector.
#[test]
fn delegation_preimage_matches_the_frozen_sig_structure() {
    let (_root_dir, root) = identity(&ROOT_SEED);
    let (_delegated_dir, delegated) = identity(&DELEGATED_SEED);

    let mut checked = 0;
    for v in all_vectors() {
        let name = v.name;
        let claim = v.claim(delegated.public_key_bytes());

        assert_eq!(
            claim.role.wire_value(),
            v.role_wire,
            "{name}: the role's wire encoding has moved from the frozen value"
        );
        assert_eq!(
            claim.subject_kind.wire_value(),
            v.subject_kind_wire,
            "{name}: the subject kind's wire encoding has moved from the frozen value"
        );

        let preimage = delegation_preimage(&root.public_key_bytes(), &claim);
        assert_eq!(
            to_hex(&preimage),
            v.hex_preimage,
            "{name}: the signing preimage no longer matches the frozen vector — \
             every signature ever produced under this format is over these bytes, \
             so a change here breaks all historical verification"
        );
        checked += 1;
    }
    assert_eq!(checked, 3, "every frozen vector must have been reproduced");
}

/// Every whole artifact, and the signature inside it, equals the frozen vector.
///
/// Ed25519 is deterministic, so `sign_delegation` under the vector's seed has
/// exactly one correct output; this is byte-identity, not mutual acceptance.
#[test]
fn sign_delegation_reproduces_the_frozen_artifacts() {
    let (_root_dir, root) = identity(&ROOT_SEED);
    let (_delegated_dir, delegated) = identity(&DELEGATED_SEED);

    let mut checked = 0;
    for v in all_vectors() {
        let name = v.name;
        let claim = v.claim(delegated.public_key_bytes());

        let artifact = sign_delegation(&root, &claim).unwrap();
        assert_eq!(to_hex(&artifact), v.hex_artifact, "{name}: artifact");

        // The signature on its own, so a failure says which half moved rather
        // than only that the artifact differs.
        assert_eq!(
            to_hex(&artifact[artifact.len() - 64..]),
            v.hex_signature,
            "{name}: signature"
        );

        // And the two-phase path must reach the same bytes as the convenience
        // wrapper: an operator signing on an air-gapped machine gets the artifact
        // a single-machine signer would have produced, or the split is a second
        // format.
        let signature: [u8; 64] = from_hex(v.hex_signature).try_into().unwrap();
        let assembled = assemble_delegation(&root.public_key_bytes(), &claim, &signature).unwrap();
        assert_eq!(
            to_hex(&assembled),
            v.hex_artifact,
            "{name}: assemble_delegation and sign_delegation disagree about the envelope"
        );
        checked += 1;
    }
    assert_eq!(checked, 3, "every frozen vector must have been signed");
}

/// The positive control: every frozen artifact verifies, and parses back to the
/// claim it was built from.
///
/// Without this, every assertion above could be satisfied by an implementation
/// broken in the same direction as the literals — the literals would then be
/// pinning a defect rather than a format.
///
/// Each vector is verified under **its own** subject kind, which is also the
/// cross-kind control: B is a seat delegation and A and C are domain ones, so a
/// verifier that ignored the kind would be the only way all three could pass
/// under one.
#[test]
fn the_frozen_artifacts_verify_and_parse_back() {
    let (_root_dir, root) = identity(&ROOT_SEED);
    let (_delegated_dir, delegated) = identity(&DELEGATED_SEED);
    let root_public_key: [u8; 32] = from_hex(HEX_ROOT_PUBLIC_KEY).try_into().unwrap();

    let mut checked = 0;
    let mut kinds = std::collections::BTreeSet::new();
    for v in all_vectors() {
        let name = v.name;
        let artifact = from_hex(v.hex_artifact);

        let verified = verify_delegation(
            &artifact,
            &root_public_key,
            v.subject_kind,
            &v.subject_value,
        )
        .unwrap_or_else(|err| panic!("{name}: the frozen artifact must verify, got {err:?}"));

        assert_eq!(verified.root_public_key, root_public_key, "{name}");
        assert_eq!(
            verified.claim,
            v.claim(delegated.public_key_bytes()),
            "{name}"
        );
        assert_eq!(to_hex(&verified.signature), v.hex_signature, "{name}");

        // Re-encoding the parsed delegation returns the same bytes, which is what
        // makes "canonical" mean the frozen encoding rather than merely a
        // self-consistent one.
        assert_eq!(to_hex(&verified.to_cose_bytes()), v.hex_artifact, "{name}");

        // The parse-only route reaches the same value. Named for parsing because
        // it is NOT verification: it takes no expected key and no expected
        // subject, so the delegation it returns vouches for nothing.
        let parsed = AnchorDelegation::from_cose_bytes(&artifact).unwrap();
        assert_eq!(parsed, verified, "{name}");

        kinds.insert(v.subject_kind_wire);
        checked += 1;
    }
    assert_eq!(checked, 3, "every frozen vector must have been verified");
    assert_eq!(
        kinds.len(),
        2,
        "both subject kinds must appear among the frozen vectors, or the seat \
         arm has no golden coverage at all"
    );

    // And the identity the vector's seed produces is the one the artifacts name,
    // so the frozen `kid` is not merely 32 plausible bytes.
    assert_eq!(root.public_key_bytes(), root_public_key);
}
