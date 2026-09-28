#![cfg(test)]
//! Byte-shape and structural-rejection tests for `lys/delegation/v1`.
//!
//! Expected bytes here are **hand-assembled from the specification tables**,
//! not captured from the encoder's own output. A golden file produced by the
//! code under test only proves the code is self-consistent.
//!
//! One honest limitation, stated rather than left implicit: these literals were
//! written by the same party that wrote the encoder, so they are independent of
//! the *implementation* but not of the *author*. The independent encoder and
//! the `go-cose` gate are what supply independence on the encoding and envelope
//! axes; these tests supply the byte-level pin that makes any drift loud.

use super::*;
use crate::delegation::artifact::{DelegationRole, DelegationSubjectKind};

#[path = "encoding_tests/decode_envelope.rs"]
mod decode_envelope;
#[path = "encoding_tests/decode_payload.rs"]
mod decode_payload;
#[path = "encoding_tests/encodable.rs"]
mod encodable;
#[path = "encoding_tests/layout.rs"]
mod layout;

/// A fixed root key. It occupies `kid`, which is length-pinned but not
/// point-validated — a `kid` that is not a real key fails the signature check,
/// which is where it belongs — so an arbitrary 32 bytes is the right fixture.
const ROOT_KEY: [u8; KEY_LEN] = [0xa1; KEY_LEN];

/// A fixed signature.
const SIGNATURE: [u8; 64] = [0x5e; 64];

/// Encoded length of the fixture's `sequence` entry: label `06` plus the
/// two-byte head `19 01 2c`.
const SEQUENCE_ENTRY_LEN: usize = 4;

/// The fixture's `sequence`. `300` needs a two-byte head, which is a third head
/// width alongside `not_before`'s eight-byte and `role`'s inline one, so a
/// head-width bug has nowhere to hide in a single payload.
const SEQUENCE: u64 = 300;

/// A **real** Ed25519 public key for the delegated slot, derived from a fixed
/// seed rather than being an arbitrary byte pattern.
///
/// It has to be real: the payload's delegated key is now validated as a point
/// strict verification could accept, so `[0xd2; 32]` — the previous fixture —
/// is exactly the kind of value this format refuses. Derived through the public
/// `load` route so the test cannot disagree with how the crate makes keys.
fn delegated_key() -> [u8; KEY_LEN] {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("delegated.key");
    std::fs::write(&path, [0x42u8; 32]).unwrap();
    crate::Ed25519Identity::load(&path)
        .unwrap()
        .public_key_bytes()
}

fn claim() -> DelegationClaim {
    DelegationClaim {
        subject_kind: DelegationSubjectKind::Domain,
        subject_value: "example.test".to_string(),
        delegated_public_key: delegated_key(),
        role: DelegationRole::Operational,
        not_before_unix_ms: 1_700_000_000_000,
        sequence: SEQUENCE,
    }
}

/// The `(seat, speaks-for)` claim — the other valid pair, over the same key
/// material, so the two differ in exactly the two fields under test.
fn seat_claim() -> DelegationClaim {
    DelegationClaim {
        subject_kind: DelegationSubjectKind::Seat,
        subject_value: SEAT.to_string(),
        delegated_public_key: delegated_key(),
        role: DelegationRole::SpeaksFor,
        not_before_unix_ms: 1_700_000_000_000,
        sequence: SEQUENCE,
    }
}

/// A seat identifier. **Deliberately equal to the domain fixture's value**, so
/// the cases below turn on the subject KIND alone and cannot be satisfied by a
/// verifier that only compares strings. A seat identifier is arbitrary text
/// minted elsewhere, so this collision is an attacker's choice rather than a
/// contrivance.
const SEAT: &str = "example.test";

/// The protected bucket, written out byte by byte from the spec §1.1 table.
fn expected_protected(key: &[u8; KEY_LEN]) -> Vec<u8> {
    let mut out = vec![
        0xa3, // map(3)
        0x01, 0x27, // 1 (alg) => -8 (EdDSA)
        0x03, 0x78, 0x26, // 3 (content type) => text(38)
    ];
    out.extend_from_slice(b"application/vnd.lys.delegation.v1+cbor");
    out.extend_from_slice(&[0x04, 0x58, 0x20]); // 4 (kid) => bstr(32)
    out.extend_from_slice(key);
    out
}

/// The payload map for [`claim`], written out byte by byte from the spec §1.2
/// table with every head width spelled as a literal.
///
/// Deliberately takes no parameters. A parameterised version would need its own
/// shortest-head logic, which is the encoder under test reimplemented — and two
/// copies of one algorithm agree with each other right up until both are wrong.
fn expected_payload(delegated: &[u8; KEY_LEN]) -> Vec<u8> {
    let mut out = vec![
        0xa6, // map(6)
        0x01, 0x01, // 1 (subject_kind) => 1 (domain), inline
        0x02, 0x6c, // 2 (subject_value) => text(12)
    ];
    out.extend_from_slice(b"example.test");
    out.extend_from_slice(&[0x03, 0x58, 0x20]); // 3 (delegated key) => bstr(32)
    out.extend_from_slice(delegated);
    // 4 (role) => 2 (operational), inline. NOT 1: the role vocabulary is offset
    // from the subject-kind vocabulary so that no valid pair has kind == role,
    // which is what makes a transposition of the two fields visible in the bytes.
    out.extend_from_slice(&[0x04, 0x02]);
    // 5 (not_before) => 1_700_000_000_000. Above u32::MAX, so the shortest head
    // that fits is the eight-byte one.
    out.extend_from_slice(&[0x05, 0x1b]);
    out.extend_from_slice(&1_700_000_000_000u64.to_be_bytes());
    // 6 (sequence) => 300 = 0x012c. Above 255, so the shortest head that fits is
    // the two-byte one.
    out.extend_from_slice(&[0x06, 0x19, 0x01, 0x2c]);
    out
}

/// Public-key byte patterns that strict Ed25519 verification can never accept.
///
/// - all-zeros: decompresses to the identity, which is small-order.
/// - all-`0xff`: `y >= p`, so it is not a canonical encoding of any point.
/// - the identity point encoded canonically (`y = 1`): small-order, and the
///   one that a naive "is it all zeros?" check would miss.
fn unusable_keys() -> [[u8; KEY_LEN]; 3] {
    let mut identity_point = [0u8; KEY_LEN];
    identity_point[0] = 1;
    [[0u8; KEY_LEN], [0xffu8; KEY_LEN], identity_point]
}

/// The offset of the `04` role label inside a canonical payload, located by its
/// **neighbours** rather than by a byte search.
///
/// A window search for `[0x04, 0x02]` would be ambiguous: the same two bytes can
/// occur inside the 32-byte delegated key or inside the subject value, and a
/// fixture where they happen not to is a fixture, not a property. The tail of
/// every canonical payload is structurally fixed — `04 <role> 05 <not_before
/// head> 06 <sequence head>` — so the role entry is located from the end, and the
/// locating assumption is asserted.
fn role_offset(payload: &[u8]) -> usize {
    // The fixture's `not_before` needs the 8-byte head, so its entry is
    // `05 1b <8 bytes>` = 10 bytes; the role entry is `04 <role>` = 2.
    let not_before_entry_len = 10;
    let at = payload.len() - SEQUENCE_ENTRY_LEN - not_before_entry_len - 2;
    assert_eq!(payload[at], 0x04, "label 4 must be where the tail puts it");
    assert_eq!(
        payload[at + 2],
        0x05,
        "label 5 must immediately follow the role value"
    );
    at
}
