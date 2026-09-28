#![cfg(test)]
//! The `lys/delegation/v1` fixed vectors, pinned as literal hex.
//!
//! Five vectors are frozen here — **A** (domain), **B** (seat), **C** (wide
//! heads), **D** (the `tstr` inline boundary) and **F** (the `uint` inline
//! boundary). What each is *for* is recorded
//! below, because a vector whose purpose is not written down is a vector
//! somebody later "simplifies".
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
//! - **D — the `tstr` inline boundary, from BELOW.** A 23-byte `subject_value`
//!   (`0x77`) is the largest CBOR encodes inline; 24 is the first needing an
//!   argument byte. A (12), B (`7820`) and C (`79012c`) all jump clear over it,
//!   so an encoder switching one value early is invisible to every one of them.
//!   D also carries `sequence = 0`, pinning the genesis *shape* — `(1, 2, 0)`.
//!
//!   ⚠️ D isolates the **length** path only. Its companion **F** carries
//!   `not_before = 23` and isolates the **integer** path, and the two are
//!   separate artifacts because 23 is the only value at the top of the inline
//!   range: one artifact holding it in both fields puts the same number in two
//!   fields of the vector written to pin that number. Measured, not assumed —
//!   breaking only the length path moves D and not F, and only the integer path
//!   moves F and not D, while A and B miss both.
//!
//! # What the set does NOT cover, stated so it cannot read as complete
//!
//! Head width is a property of a *range*; shortest-form is a property of a
//! *boundary*. The format has **eight** reachable ones — four each in
//! `not_before_unix_ms` and `sequence` at 23|24, 255|256, 65535|65536 and
//! 2³²−1|2³², plus the `subject_value` length at 23|24 and at 255|256.
//!
//! A+B+C place a value adjacent to exactly one, from above only (`sequence = 24`).
//! D and F add the low side of the 23|24 boundary in both paths. **Everything at
//! 255|256 and above remains untouched.**
//!
//! ⚠️ Two cases that look like gaps and are not, so they stop being counted:
//! there is **no inline representation of 24** (`0x18` in the low bits *is* the
//! one-argument-byte marker), so an encoder cannot emit one and the only
//! expressible error at that boundary is the non-minimal form for values ≤ 23 —
//! which is the side D and F cover. And the `bstr` inline boundary is
//! unreachable in this format at all: every `bstr` here is 0, 32, 64, 79, or a
//! payload whose minimum is far above 23.
//!
//! ⚠️ **D and F add no new `(major type, argument width)` pair over A and B.**
//! Their entire value is the boundary, not the width. Recorded because a reader
//! counting vectors would otherwise credit them with coverage they do not add.
//!
//! # Vector E is specified and NOT generated
//!
//! **E — `not_before_unix_ms = 2⁶³`**, the value §1.2 argues the `u64` bound
//! over. Its bytes were computed and agreed by both parties, and it is not
//! frozen here because **it cannot measure the claim it exists for**: an `i64`
//! implementation cannot represent 2⁶³ at all, so it can never be asked to
//! encode E, and every available encoder emits the same eight bytes by
//! construction. The discriminating test is a *decoder* property — decode E with
//! label 5 as `uint64` and assert equality, and as `int64` and assert it fails —
//! and no golden vector can contain it.
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

use lys_core::Ed25519Identity;
use lys_core::delegation::{
    Delegation, DelegationClaim, DelegationRole, DelegationSubjectKind, assemble_delegation,
    delegation_preimage, sign_delegation, verify_delegation,
};

mod fields;
mod frozen_hex;
mod shortest_form;
mod signing;

use fields::*;
use frozen_hex::*;

// ---------------------------------------------------------------------------
// Helpers. None of these calls into `lys-core`.
// ---------------------------------------------------------------------------

fn to_hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    bytes.iter().fold(String::new(), |mut acc, byte| {
        write!(acc, "{byte:02x}").expect("writing to a String cannot fail");
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

fn vector_d() -> FrozenVector {
    FrozenVector {
        name: "D (tstr inline boundary)",
        subject_kind: DelegationSubjectKind::Domain,
        subject_kind_wire: SUBJECT_KIND_WIRE_VALUE,
        subject_value: D_SUBJECT_VALUE.to_string(),
        role: DelegationRole::Operational,
        role_wire: ROLE_WIRE_VALUE,
        not_before_unix_ms: NOT_BEFORE_UNIX_MS,
        sequence: D_SEQUENCE,
        hex_protected: D_HEX_PROTECTED,
        hex_payload: D_HEX_PAYLOAD,
        hex_preimage: D_HEX_PREIMAGE,
        hex_signature: D_HEX_SIGNATURE,
        hex_artifact: D_HEX_ARTIFACT,
    }
}

fn vector_f() -> FrozenVector {
    FrozenVector {
        name: "F (uint inline boundary)",
        subject_kind: DelegationSubjectKind::Domain,
        subject_kind_wire: SUBJECT_KIND_WIRE_VALUE,
        subject_value: ORIGIN.to_string(),
        role: DelegationRole::Operational,
        role_wire: ROLE_WIRE_VALUE,
        not_before_unix_ms: F_NOT_BEFORE_UNIX_MS,
        sequence: F_SEQUENCE,
        hex_protected: F_HEX_PROTECTED,
        hex_payload: F_HEX_PAYLOAD,
        hex_preimage: F_HEX_PREIMAGE,
        hex_signature: F_HEX_SIGNATURE,
        hex_artifact: F_HEX_ARTIFACT,
    }
}

fn all_vectors() -> Vec<FrozenVector> {
    vec![vector_a(), vector_b(), vector_c(), vector_d(), vector_f()]
}
