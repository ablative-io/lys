//! Byte-level tests for the shared canonical CBOR primitives.
//!
//! Every expectation here is a literal from RFC 8949 §4.2 (core deterministic
//! encoding) or RFC 9052 §4.4 (`Sig_structure`), written out rather than
//! computed, so a change to the writers fails against the specification and
//! not against itself.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;

/// Helper: encode one head and return it.
fn head(major: u8, value: u64) -> Vec<u8> {
    let mut out = Vec::new();
    write_head(&mut out, major, value);
    out
}

#[test]
fn heads_use_the_shortest_form_at_every_boundary() {
    // RFC 8949 §4.2.1 rule 2: the argument is encoded in the fewest bytes.
    // The boundaries are where a naive encoder picks the wider form.
    assert_eq!(head(MAJOR_UNSIGNED, 0), vec![0x00]);
    assert_eq!(head(MAJOR_UNSIGNED, 23), vec![0x17]);
    assert_eq!(head(MAJOR_UNSIGNED, 24), vec![0x18, 24]);
    assert_eq!(head(MAJOR_UNSIGNED, 255), vec![0x18, 0xff]);
    assert_eq!(head(MAJOR_UNSIGNED, 256), vec![0x19, 0x01, 0x00]);
    assert_eq!(head(MAJOR_UNSIGNED, 65535), vec![0x19, 0xff, 0xff]);
    assert_eq!(
        head(MAJOR_UNSIGNED, 65536),
        vec![0x1a, 0x00, 0x01, 0x00, 0x00]
    );
    assert_eq!(
        head(MAJOR_UNSIGNED, u64::from(u32::MAX)),
        vec![0x1a, 0xff, 0xff, 0xff, 0xff]
    );
    assert_eq!(
        head(MAJOR_UNSIGNED, u64::from(u32::MAX) + 1),
        vec![0x1b, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00]
    );
    assert_eq!(
        head(MAJOR_UNSIGNED, u64::MAX),
        vec![0x1b; 1]
            .into_iter()
            .chain([0xff; 8])
            .collect::<Vec<u8>>()
    );
}

#[test]
fn every_major_type_lands_in_the_top_three_bits() {
    for major in [
        MAJOR_UNSIGNED,
        MAJOR_NEGATIVE,
        MAJOR_BYTES,
        MAJOR_TEXT,
        MAJOR_ARRAY,
        MAJOR_MAP,
        MAJOR_TAG,
    ] {
        assert_eq!(head(major, 1)[0] >> 5, major);
        assert_eq!(head(major, 1000)[0] >> 5, major);
    }
}

#[test]
fn signed_integers_use_major_one_for_negatives() {
    // RFC 8949 §3.1: major type 1 encodes -1-n, so -1 is argument 0.
    let mut out = Vec::new();
    write_i64(&mut out, -1);
    assert_eq!(out, vec![0x20]);

    // The COSE header values lys actually uses.
    let mut alg = Vec::new();
    write_i64(&mut alg, -8);
    assert_eq!(alg, vec![0x27], "EdDSA alg -8 is one byte");

    let mut inclusion = Vec::new();
    write_i64(&mut inclusion, -1);
    assert_eq!(inclusion, vec![0x20], "inclusion proof type -1 is one byte");

    let mut zero = Vec::new();
    write_i64(&mut zero, 0);
    assert_eq!(zero, vec![0x00], "zero is major type 0, not 1");
}

#[test]
fn the_extremes_of_i64_do_not_overflow() {
    // `write_i64` computes |value| - 1 for negatives; i64::MIN is the case
    // where a naive `-value` would panic in debug and wrap in release.
    let mut min = Vec::new();
    write_i64(&mut min, i64::MIN);
    assert_eq!(
        min,
        vec![0x3b, 0x7f, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]
    );

    let mut max = Vec::new();
    write_i64(&mut max, i64::MAX);
    assert_eq!(
        max,
        vec![0x1b, 0x7f, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]
    );
}

#[test]
fn text_and_bytes_carry_a_length_prefixed_definite_head() {
    let mut text = Vec::new();
    write_text(&mut text, "a");
    assert_eq!(text, vec![0x61, b'a']);

    let mut empty_text = Vec::new();
    write_text(&mut empty_text, "");
    assert_eq!(empty_text, vec![0x60]);

    let mut bytes = Vec::new();
    write_bytes(&mut bytes, &[0xde, 0xad]);
    assert_eq!(bytes, vec![0x42, 0xde, 0xad]);

    let mut empty_bytes = Vec::new();
    write_bytes(&mut empty_bytes, &[]);
    assert_eq!(empty_bytes, vec![0x40]);

    // A 32-byte digest — the shape both artifacts use most.
    let mut digest = Vec::new();
    write_bytes(&mut digest, &[0x11; 32]);
    assert_eq!(digest[0..2], [0x58, 0x20]);
    assert_eq!(digest.len(), 34);
}

#[test]
fn null_is_the_canonical_detached_payload_marker() {
    // RFC 9052 requires a detached payload to be encoded as CBOR nil, and
    // RFC 8949 §3.3 fixes that as major type 7 simple value 22.
    assert_eq!(NULL, 0xf6);
    assert_eq!(NULL >> 5, 7);
    assert_eq!(NULL & 0x1f, 22);
}

#[test]
fn sig_structure_matches_the_rfc_9052_layout() {
    let protected = [0xa1, 0x01, 0x27];
    let payload = [0x07; 4];
    let built = sig_structure_bytes(&protected, &payload);

    let mut expected = Vec::new();
    expected.push(0x84); // array(4)
    expected.push(0x6a); // text(10)
    expected.extend_from_slice(b"Signature1");
    expected.push(0x43); // bstr(3) — protected
    expected.extend_from_slice(&protected);
    expected.push(0x40); // bstr(0) — empty external_aad
    expected.push(0x44); // bstr(4) — payload
    expected.extend_from_slice(&payload);

    assert_eq!(built, expected);
}

#[test]
fn sig_structure_always_starts_with_the_domain_separating_prefix() {
    // The byte-0 disjointness argument in `attestation::sign` and
    // `receipt::sign` rests on this prefix being unconditional.
    for protected_len in [0usize, 1, 80, 300] {
        for payload_len in [0usize, 32, 1000] {
            let built = sig_structure_bytes(&vec![0xaa; protected_len], &vec![0xbb; payload_len]);
            assert_eq!(&built[0..12], b"\x84\x6aSignature1");
        }
    }
}

#[test]
fn empty_external_aad_is_present_rather_than_omitted() {
    // Dropping the empty bstr would produce a 3-element structure that some
    // permissive verifiers accept, and would change every signature lys makes.
    let built = sig_structure_bytes(&[], &[]);
    assert_eq!(
        built,
        vec![
            0x84, 0x6a, b'S', b'i', b'g', b'n', b'a', b't', b'u', b'r', b'e', b'1', 0x40, 0x40,
            0x40
        ]
    );
}
