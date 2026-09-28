#![cfg(test)]
//! The inputs each vector is built from: the seeds, and every field of the
//! specification table for vectors A, B, C, D and F.

// ---------------------------------------------------------------------------
// Shared inputs — the seeds and the content type, from spec §1.1 and §6.1.
// ---------------------------------------------------------------------------

/// Root Ed25519 seed: bytes `0x00..=0x1f`. Shared by all three vectors, so every
/// protected header below is byte-identical and each vector isolates its payload.
pub const ROOT_SEED: [u8; 32] = [
    0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f,
    0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f,
];

/// Delegated Ed25519 seed: bytes `0x20..=0x3f`. Shared by all three vectors.
pub const DELEGATED_SEED: [u8; 32] = [
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
pub const CONTENT_TYPE: &str = "application/vnd.lys.delegation.v1+cbor";

// ---------------------------------------------------------------------------
// Vector A — the domain arm. Inputs from the specification's §6.1 table.
// ---------------------------------------------------------------------------

/// Vector A's `subject_value`, for `subject_kind = domain`. `example.test` is
/// a reserved name and deliberately not a production origin: a committed
/// constant naming a real one is precisely what DP15 forbids.
pub const ORIGIN: &str = "example.test";

/// Vector A's `not_before_unix_ms`. Written as a number, not imported.
pub const NOT_BEFORE_UNIX_MS: u64 = 1_700_000_000_000;

/// Vector A's role, as the **wire integer**. Compared against
/// `DelegationRole::Operational.wire_value()` so the enum cannot silently
/// re-number itself.
///
/// **`2`, not `1`, and the gap below it is deliberate.** `1` is
/// `DelegationSubjectKind::Domain`'s wire value; the role vocabulary starts
/// above the kind vocabulary so that no valid `(kind, role)` pair has equal
/// halves. See the module docs. Anyone "tidying" this to `1` reintroduces a
/// defect no vector can detect.
pub const ROLE_WIRE_VALUE: u64 = 2;

/// Vector A's `subject_kind`, as the **wire integer**. Written out for the
/// same reason as the role: the pair is the thing that must not drift, and
/// pinning one half would leave the other free to move.
pub const SUBJECT_KIND_WIRE_VALUE: u64 = 1;

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
pub const SEQUENCE: u64 = 300;

// ---------------------------------------------------------------------------
// Vector B — the seat arm. Inputs from the specification's §6.1.1 table.
// ---------------------------------------------------------------------------

/// Vector B's `subject_value`: a seat identifier, **32 bytes**.
///
/// The length is load-bearing twice over, and neither reason is aesthetic. See
/// the module docs: 31 would tie the protected bucket's length, and 32 puts
/// `7820` beside `5820` — one argument byte under two different major types.
pub const B_SUBJECT_VALUE: &str = "lys-seat-00000000000000000000001";

/// Vector B's `subject_kind`: `2`, seat. The arm no other vector reaches.
pub const B_SUBJECT_KIND_WIRE_VALUE: u64 = 2;

/// Vector B's role: `3`, speaks-for. Pair `(2, 3)`.
pub const B_ROLE_WIRE_VALUE: u64 = 3;

/// Vector B's `not_before_unix_ms`: **`7`**, and every digit of that choice is
/// deliberate.
///
/// It must be **inline**, because vector A's value needs the eight-byte head
/// anyway and so cannot test the shortest-form rule at all. It must not be `1`,
/// because `1` is payload label 1's value *and* label 1's own number, and a
/// positional decoder reading a shifted label-value stream is exactly the bug
/// class this vector exists to expose. `7` is inline, distinct from `2`, `3` and
/// `24`, and collides with no label in the map.
pub const B_NOT_BEFORE_UNIX_MS: u64 = 7;

/// Vector B's `sequence`: `24`, encoded `1818`.
///
/// The exact value at which CBOR stops inlining an argument. Off-by-one head
/// logic passes at `23` and at `300` and fails only here, which is why the
/// boundary is worth a vector rather than a comment.
pub const B_SEQUENCE: u64 = 24;

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
pub const C_SUBJECT_VALUE_LEN: usize = 300;

/// Vector C's `sequence`: `70000`, encoded `1a00011170` — the **four**-argument-
/// byte width that A and B jump straight over.
pub const C_SEQUENCE: u64 = 70_000;

// ---------------------------------------------------------------------------
// Vector D — the `tstr` inline boundary, isolated. Spec §6.1.1.
//
// A `tstr` length of 23 is the largest CBOR encodes inline (`0x77`); 24 is the
// first needing an argument byte. An encoder that switches one value early
// emits `7817` here, and A (12 bytes), B (`7820`) and C (`79012c`) all jump
// clear over the boundary and cannot see it.
//
// ⚠️ D isolates the LENGTH path only. Its companion F carries `not_before = 23`
// and isolates the INTEGER path, because 23 is the only value at the top of the
// inline range and one artifact holding it in both fields would have been a
// coincidence rather than a control — measured: deliberately breaking only the
// length path moves D and not F, and only the integer path moves F and not D.
//
// ⚠️ `not_before` is deliberately A's value and NOT 23. An earlier draft used
// 23 for both, which put the same number in two fields of the one artifact
// written to pin that number.
// ---------------------------------------------------------------------------

/// Vector D's `subject_value` — **exactly 23 bytes**, the top of the CBOR
/// inline `tstr` range, with `tstr` head `0x77`.
///
/// The text states its own length on purpose. The specification originally
/// said only "a 23-byte value" and the dispatch acting on it supplied a
/// 20-byte one; both independent parties caught it, and neither adjusted it
/// silently. **A specified length is not a specified value** — where the point
/// of a vector *is* a length, the length must be checkable by counting the
/// literal rather than by trusting the adjective beside it.
///
/// It is also a plain domain. The 23-byte replacement first chosen was
/// `lys-seat-…` under `subject_kind = 1 (domain)` — a seat-shaped string in a
/// domain slot, which is the confusable pairing §3.3 exists to prevent, in the
/// vector most likely to be copied as a starting point.
pub const D_SUBJECT_VALUE: &str = "twenty-three-bytes.test";

/// Vector D's `sequence`: `0`, encoded inline as `0x00`.
///
/// §6.1 refused a zero sequence for its own vector, because a zero field is
/// indistinguishable from one an implementation forgot to write. That was
/// correct with one vector and is spent with four: A, B and C all carry nonzero
/// sequences, so a defaulting bug already fails three times. D therefore pins
/// the genesis *shape* — `(kind, role, sequence) = (1, 2, 0)`.
///
/// ⚠️ It does **not** pin genesis's payload. §5 fixes three of the six fields;
/// `subject_value`, `not_before` and the delegated key are the operator's, and
/// DP15 forbids committing a real origin.
pub const D_SEQUENCE: u64 = 0;

// ---------------------------------------------------------------------------
// Vector F — the `uint` inline boundary, isolated. Spec §6.1.1.
// ---------------------------------------------------------------------------

/// Vector F's `not_before_unix_ms`: **23**, encoded inline as `0x17` — the
/// largest `uint` CBOR encodes without an argument byte.
///
/// B's `sequence = 24` (`1818`) pins this boundary from *above*; nothing pinned
/// it from below, and an encoder that switches to an argument byte one value
/// early emits `1817` here and passes A, B and C.
pub const F_NOT_BEFORE_UNIX_MS: u64 = 23;

/// Vector F's `sequence`: **8**.
///
/// ⛔ **It was `1`, and `subject_kind` is also `1`.** Both encode the byte
/// `0x01`, so transposing the values at labels 1 and 6 left F **byte-identical**
/// — measured by both independent parties, not argued — as did the derived-field
/// bugs `sequence := subject_kind` and `subject_kind := sequence`. The ruling
/// that chose `1` claimed in terms that it had removed every such collision,
/// having counted only the three *varying* numbers and omitted the two the
/// format pins.
///
/// ⭐ **A distinctness claim must count the fields that are FIXED, not just the
/// ones you chose.** The omitted values were the two present in every artifact,
/// which is precisely why they did not feel like variables.
///
/// `8` is inline, so F's boundary isolation is untouched; it equals no other
/// field value, no label number `1`–`6`, and no length anywhere in the artifact.
/// The transposed payload now carries `subject_kind = 8`, which `v1` does not
/// define, so it is **refused at decode** rather than merely differing.
pub const F_SEQUENCE: u64 = 8;
