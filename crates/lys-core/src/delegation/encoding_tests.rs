//! Byte-shape and structural-rejection tests for `lys/anchor-delegation/v1`.
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

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;

/// A fixed root key. It occupies `kid`, which is length-pinned but not
/// point-validated — a `kid` that is not a real key fails the signature check,
/// which is where it belongs — so an arbitrary 32 bytes is the right fixture.
const ROOT_KEY: [u8; KEY_LEN] = [0xa1; KEY_LEN];

/// A fixed signature.
const SIGNATURE: [u8; 64] = [0x5e; 64];

/// Encoded length of the fixture's `sequence` entry: label `05` plus the
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
        origin: "example.test".to_string(),
        delegated_public_key: delegated_key(),
        role: DelegationRole::Operational,
        not_before_unix_ms: 1_700_000_000_000,
        sequence: SEQUENCE,
    }
}

/// The protected bucket, written out byte by byte from the spec §1.1 table.
fn expected_protected(key: &[u8; KEY_LEN]) -> Vec<u8> {
    let mut out = vec![
        0xa3, // map(3)
        0x01, 0x27, // 1 (alg) => -8 (EdDSA)
        0x03, 0x78, 0x2d, // 3 (content type) => text(45)
    ];
    out.extend_from_slice(b"application/vnd.lys.anchor-delegation.v1+cbor");
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
        0xa5, // map(5)
        0x01, 0x6c, // 1 (origin) => text(12)
    ];
    out.extend_from_slice(b"example.test");
    out.extend_from_slice(&[0x02, 0x58, 0x20]); // 2 (delegated key) => bstr(32)
    out.extend_from_slice(delegated);
    out.extend_from_slice(&[0x03, 0x01]); // 3 (role) => 1, inline
    // 4 (not_before) => 1_700_000_000_000. Above u32::MAX, so the shortest head
    // that fits is the eight-byte one.
    out.extend_from_slice(&[0x04, 0x1b]);
    out.extend_from_slice(&1_700_000_000_000u64.to_be_bytes());
    // 5 (sequence) => 300 = 0x012c. Above 255, so the shortest head that fits is
    // the two-byte one.
    out.extend_from_slice(&[0x05, 0x19, 0x01, 0x2c]);
    out
}

#[test]
fn the_content_type_is_the_frozen_string() {
    assert_eq!(
        CONTENT_TYPE,
        "application/vnd.lys.anchor-delegation.v1+cbor"
    );
    assert_eq!(CONTENT_TYPE.len(), 45);
    assert!(
        CONTENT_TYPE.starts_with("application/vnd.lys."),
        "the media type uses the dotted vnd. form, not the context-tag form"
    );
    assert_ne!(
        CONTENT_TYPE, "application/vnd.lys.receipt.v1+cbor",
        "the delegation and receipt discriminators must differ"
    );
}

#[test]
fn protected_bucket_matches_the_specification_bytes() {
    let built = protected_bytes(CONTENT_TYPE, &ROOT_KEY);
    assert_eq!(built, expected_protected(&ROOT_KEY));
    assert_eq!(built.len(), PROTECTED_LEN);
    assert_eq!(built.len(), 86, "the protected bucket is always 86 bytes");
}

#[test]
fn the_protected_bucket_carries_no_vds_label() {
    // 395 encodes as `19 01 8b`. A delegation proves nothing about a tree, so
    // the label that declares a verifiable data structure must be absent.
    let built = protected_bytes(CONTENT_TYPE, &ROOT_KEY);
    assert!(
        !built.windows(3).any(|w| w == [0x19, 0x01, 0x8b]),
        "no vds label may appear in a delegation's protected bucket"
    );
    assert_eq!(built[0], 0xa3, "a 3-entry map, not a receipt's 4-entry one");
}

#[test]
fn payload_matches_the_specification_bytes() {
    let delegated = delegated_key();
    let built = payload_bytes(&claim());
    assert_eq!(built, expected_payload(&delegated));
    assert_eq!(built[0], 0xa5, "five entries: sequence is label 5");
}

#[test]
fn the_sequence_field_is_a_u64_with_a_shortest_form_head() {
    // Both halves matter, and neither is exposed by the fixed vector alone. The
    // `u64` bound stops an `i64` model finding values >= 2^63 wire-legal and
    // undecodable; the shortest-form rule stops one number having two encodings.
    let cases: [(u64, &[u8]); 9] = [
        (0, &[0x00]),
        (23, &[0x17]),
        (24, &[0x18, 0x18]),
        (255, &[0x18, 0xff]),
        (256, &[0x19, 0x01, 0x00]),
        (65_535, &[0x19, 0xff, 0xff]),
        (65_536, &[0x1a, 0x00, 0x01, 0x00, 0x00]),
        (4_294_967_295, &[0x1a, 0xff, 0xff, 0xff, 0xff]),
        // `u64::MAX` is refused (a sequence must have a successor), so the
        // eight-byte head is exercised at the largest value that IS issuable.
        (
            MAX_SEQUENCE,
            &[0x1b, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xfe],
        ),
    ];

    let mut checked = 0;
    for (sequence, expected_head) in cases {
        let mut c = claim();
        c.sequence = sequence;
        let payload = payload_bytes(&c);
        // The sequence entry is the last thing in the map, so its encoding is
        // the tail after label 5.
        let tail_at = payload.len() - expected_head.len();
        assert_eq!(
            payload[tail_at - 1],
            0x05,
            "label 5 must immediately precede the sequence value for {sequence}"
        );
        assert_eq!(
            &payload[tail_at..],
            expected_head,
            "sequence {sequence} used a non-shortest head"
        );
        // And it survives the round trip, which is what the u64 bound buys.
        let artifact = artifact_bytes(&ROOT_KEY, &c, &SIGNATURE);
        assert_eq!(decode_fields(&artifact).unwrap().claim.sequence, sequence);
        checked += 1;
    }
    assert_eq!(checked, 9, "every head-width boundary must have been tried");
}

#[test]
fn not_before_also_uses_a_shortest_form_head_across_every_boundary() {
    // The same discipline for label 4. Its head sits between label 4 and label
    // 5, so it is located by its neighbours rather than by an offset.
    let cases: [(u64, &[u8]); 6] = [
        (0, &[0x00]),
        (23, &[0x17]),
        (24, &[0x18, 0x18]),
        (65_536, &[0x1a, 0x00, 0x01, 0x00, 0x00]),
        (4_294_967_296, &[0x1b, 0, 0, 0, 1, 0, 0, 0, 0]),
        (
            u64::MAX,
            &[0x1b, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff],
        ),
    ];

    let mut checked = 0;
    for (not_before, expected_head) in cases {
        let mut c = claim();
        c.not_before_unix_ms = not_before;
        let payload = payload_bytes(&c);
        // The tail is `04 <not_before head> 05 19 01 2c`: the fixture's
        // sequence of 300 always occupies the last four bytes.
        let at = payload.len() - SEQUENCE_ENTRY_LEN - expected_head.len();
        assert_eq!(
            &payload[payload.len() - SEQUENCE_ENTRY_LEN..],
            &[0x05, 0x19, 0x01, 0x2c],
            "the fixture's sequence entry must be where this offset assumes"
        );
        assert_eq!(payload[at - 1], 0x04, "label 4 precedes not_before");
        assert_eq!(
            &payload[at..at + expected_head.len()],
            expected_head,
            "not_before {not_before} used a non-shortest head"
        );
        let artifact = artifact_bytes(&ROOT_KEY, &c, &SIGNATURE);
        assert_eq!(
            decode_fields(&artifact).unwrap().claim.not_before_unix_ms,
            not_before
        );
        checked += 1;
    }
    assert_eq!(checked, 6, "every head-width boundary must have been tried");
}

#[test]
fn the_delegated_key_and_the_root_key_occupy_different_slots() {
    // A swap between kid and the payload key would be invisible if both slots
    // held the same bytes, so the fixture uses distinct keys and this test says
    // why. The root key must appear only in the protected bucket, the delegated
    // key only in the payload.
    let delegated = delegated_key();
    assert_ne!(delegated, ROOT_KEY);
    let protected = protected_bytes(CONTENT_TYPE, &ROOT_KEY);
    let payload = payload_bytes(&claim());
    assert!(protected.windows(KEY_LEN).any(|w| w == ROOT_KEY));
    assert!(!protected.windows(KEY_LEN).any(|w| w == delegated));
    assert!(payload.windows(KEY_LEN).any(|w| w == delegated));
    assert!(!payload.windows(KEY_LEN).any(|w| w == ROOT_KEY));
}

#[test]
fn the_envelope_is_tagged_with_an_empty_unprotected_bucket_and_an_embedded_payload() {
    let protected = protected_bytes(CONTENT_TYPE, &ROOT_KEY);
    let payload = payload_bytes(&claim());
    let artifact = artifact_bytes(&ROOT_KEY, &claim(), &SIGNATURE);
    assert_eq!(artifact, envelope_bytes(&protected, &payload, &SIGNATURE));

    let mut expected = vec![0xd2, 0x84, 0x58, 0x56]; // 18([ ... ]), bstr(86)
    expected.extend_from_slice(&protected);
    expected.push(0xa0); // the empty unprotected map
    assert!(
        (24..=255).contains(&payload.len()),
        "this literal assumes the one-extra-byte bstr head"
    );
    expected.push(0x58); // bstr(n)
    expected.push(u8::try_from(payload.len()).unwrap());
    expected.extend_from_slice(&payload);
    expected.extend_from_slice(&[0x58, 0x40]); // bstr(64)
    expected.extend_from_slice(&SIGNATURE);
    assert_eq!(artifact, expected);

    // The payload is embedded, not detached: `nil` (0xf6) never appears where a
    // receipt would carry it.
    assert_ne!(artifact[4 + protected.len() + 1], 0xf6);
}

#[test]
fn the_role_wire_values_are_the_frozen_mapping() {
    assert_eq!(DelegationRole::Operational.wire_value(), 1);
    assert_eq!(
        DelegationRole::from_wire(1),
        Some(DelegationRole::Operational)
    );
    // A closed enum: every other value is a decode failure, not a carried one.
    let mut refused = 0;
    for value in [0u64, 2, 3, 7, 255, u64::MAX] {
        assert!(DelegationRole::from_wire(value).is_none(), "role {value}");
        refused += 1;
    }
    assert_eq!(refused, 6, "every listed unknown role must have been tried");
}

#[test]
fn decoding_a_canonical_artifact_recovers_every_field() {
    // The positive control for this file's rejection tests: if the decoder were
    // broken in the direction of refusing everything, this is what would fail.
    let fields = decode_fields(&artifact_bytes(&ROOT_KEY, &claim(), &SIGNATURE)).unwrap();
    assert_eq!(fields.root_public_key, ROOT_KEY);
    assert_eq!(fields.claim, claim());
    assert_eq!(fields.claim.sequence, SEQUENCE);
    assert_eq!(fields.signature, SIGNATURE);
}

#[test]
fn an_oversize_input_is_refused_before_parsing() {
    let mut bytes = artifact_bytes(&ROOT_KEY, &claim(), &SIGNATURE);
    bytes.resize(MAX_ARTIFACT_LEN + 1, 0x00);
    assert!(matches!(
        decode_fields(&bytes),
        Err(TrustError::DelegationVerification)
    ));
}

// ---------------------------------------------------------------------------
// The encode side refuses everything the decode side refuses.
//
// The two were allowed to disagree once: the size cap was enforced at decode
// only, so a 3885-byte origin signed successfully and then failed every
// verification afterwards. These tests key on the *derived* boundary rather
// than on a literal, because the boundary moved when `sequence` was added.
// ---------------------------------------------------------------------------

#[test]
fn check_encodable_agrees_with_the_decoder_at_the_size_boundary() {
    // Find the longest origin that fits, by construction rather than by a
    // hardcoded number. A literal here would have been correct before `sequence`
    // landed and silently wrong after it.
    let mut longest_ok = None;
    let mut shortest_refused = None;
    for len in 3800..3900usize {
        let mut c = claim();
        c.origin = "o".repeat(len);
        let encoded_len = artifact_bytes(&ROOT_KEY, &c, &SIGNATURE).len();
        let accepted = check_encodable(&ROOT_KEY, &c).is_ok();
        assert_eq!(
            accepted,
            encoded_len <= MAX_ARTIFACT_LEN,
            "encode-side acceptance disagrees with the cap at origin length {len}"
        );
        // The decoder must reach the same verdict on the same claim.
        let decoded_ok = decode_fields(&artifact_bytes(&ROOT_KEY, &c, &SIGNATURE)).is_ok();
        assert_eq!(
            accepted, decoded_ok,
            "encode and decode disagree at origin length {len} — this is the \
             defect class the encode-side check exists to close"
        );
        if accepted {
            longest_ok = Some(len);
        } else if shortest_refused.is_none() {
            shortest_refused = Some(len);
        }
    }
    let longest_ok = longest_ok.expect("some origin length must fit");
    let shortest_refused = shortest_refused.expect("some origin length must not fit");
    assert_eq!(
        shortest_refused,
        longest_ok + 1,
        "the boundary must be a single step, not a region"
    );
}

#[test]
fn a_sequence_of_u64_max_is_refused_at_both_ends_so_a_successor_always_exists() {
    // Strictly-increasing has no successor at the maximum, so issuing there
    // would permanently disable rotation for that origin. Nothing
    // attacker-reachable leads here — issuing needs the offline root key — so
    // this is a foot-gun rather than a vulnerability, forbidden because the
    // check costs one comparison now and is impossible to add after the freeze.
    let mut at_max = claim();
    at_max.sequence = u64::MAX;

    // Encode side: an actionable, operator-facing reason.
    let err = check_encodable(&ROOT_KEY, &at_max).unwrap_err();
    let TrustError::DelegationEncoding { reason } = &err else {
        panic!("expected an encoding error naming the constraint, got {err:?}");
    };
    assert!(
        reason.contains("successor"),
        "the reason must name the property being preserved, got: {reason}"
    );

    // Decode side, so an artifact minted by some other implementation is refused
    // too — the encode check alone would only bind this crate.
    let mutant = artifact_bytes(&ROOT_KEY, &at_max, &SIGNATURE);
    assert!(matches!(
        decode_fields(&mutant),
        Err(TrustError::DelegationVerification)
    ));

    // The boundary is a single step, not a region: MAX_SEQUENCE is issuable and
    // decodes, and it is exactly one below the refused value.
    assert_eq!(MAX_SEQUENCE, u64::MAX - 1);
    let mut at_limit = claim();
    at_limit.sequence = MAX_SEQUENCE;
    check_encodable(&ROOT_KEY, &at_limit).unwrap();
    assert_eq!(
        decode_fields(&artifact_bytes(&ROOT_KEY, &at_limit, &SIGNATURE))
            .unwrap()
            .claim
            .sequence,
        MAX_SEQUENCE
    );
}

#[test]
fn check_encodable_refuses_an_empty_origin_and_an_unusable_delegated_key() {
    let mut empty_origin = claim();
    empty_origin.origin = String::new();
    assert!(matches!(
        check_encodable(&ROOT_KEY, &empty_origin),
        Err(TrustError::DelegationEncoding { .. })
    ));

    let mut refused = 0;
    for bad_key in unusable_keys() {
        let mut c = claim();
        c.delegated_public_key = bad_key;
        assert!(
            matches!(
                check_encodable(&ROOT_KEY, &c),
                Err(TrustError::DelegationEncoding { .. })
            ),
            "an unusable delegated key was accepted at encode"
        );
        refused += 1;
    }
    assert_eq!(refused, 3, "every unusable key shape must have been tried");

    // Positive control: the untouched claim encodes.
    check_encodable(&ROOT_KEY, &claim()).unwrap();
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

// ---------------------------------------------------------------------------
// Isolating cases for the rules the re-encode gate would otherwise mask.
//
// Several of this format's rules — the content-type pin, the empty unprotected
// bucket, the closed role enum, the mandatory tag, the non-empty origin, the
// usable delegated key — are each ALSO caught by the byte-compare in
// `AnchorDelegation::from_cose_bytes`, because the canonical re-encoding always
// emits the right content type, an empty bucket, a known role and the tag. A
// violation therefore fails twice at the artifact level, which means **neither
// check is proven by an artifact-level test**: disable one and the other still
// refuses, so the test stays green over a missing rule.
//
// The cases below call `decode_fields` directly, below the byte-compare, so
// each rule is the only thing that can reject its input. If a rule is deleted
// these fail and the artifact-level tests do not.
// ---------------------------------------------------------------------------

#[test]
fn decode_refuses_an_untagged_message_with_no_help_from_the_byte_compare() {
    // Stripping `0xd2` leaves exactly the untagged `COSE_Sign1` form RFC 9052
    // §4.2 permits in other contexts — well-formed CBOR that this one refuses.
    let good = artifact_bytes(&ROOT_KEY, &claim(), &SIGNATURE);
    assert_eq!(good[0], 0xd2);
    assert_eq!(good[1], 0x84);
    assert!(matches!(
        decode_fields(&good[1..]),
        Err(TrustError::DelegationVerification)
    ));
}

#[test]
fn decode_refuses_a_foreign_content_type_with_no_help_from_the_byte_compare() {
    for foreign in [
        "application/vnd.lys.receipt.v1+cbor",
        "application/vnd.lys.consistency-receipt.v1+cbor",
        "application/vnd.lys.attestation.v2+cbor",
        "application/vnd.lys.anchor-delegation.v2+cbor",
    ] {
        let mutant = envelope_bytes(
            &protected_bytes(foreign, &ROOT_KEY),
            &payload_bytes(&claim()),
            &SIGNATURE,
        );
        assert!(
            matches!(
                decode_fields(&mutant),
                Err(TrustError::DelegationVerification)
            ),
            "content type {foreign:?} was accepted"
        );
    }
    // Positive control: the same construction with our own type decodes.
    let ours = envelope_bytes(
        &protected_bytes(CONTENT_TYPE, &ROOT_KEY),
        &payload_bytes(&claim()),
        &SIGNATURE,
    );
    decode_fields(&ours).unwrap();
}

#[test]
fn decode_refuses_a_non_empty_unprotected_bucket_with_no_help_from_the_byte_compare() {
    let good = artifact_bytes(&ROOT_KEY, &claim(), &SIGNATURE);
    let unprotected_at = 4 + PROTECTED_LEN;
    assert_eq!(good[unprotected_at], 0xa0);

    let mut mutant = good[..unprotected_at].to_vec();
    mutant.extend_from_slice(&[0xa1, 0x01, 0x01]); // {1: 1}
    mutant.extend_from_slice(&good[unprotected_at + 1..]);
    assert!(matches!(
        decode_fields(&mutant),
        Err(TrustError::DelegationVerification)
    ));
}

#[test]
fn decode_refuses_an_unknown_role_with_no_help_from_the_byte_compare() {
    let canonical = payload_bytes(&claim());
    let role_at = role_offset(&canonical);

    let mut refused = 0;
    for role in [0u8, 2, 7, 23] {
        let mut payload = canonical.clone();
        payload[role_at + 1] = role;
        let mutant = envelope_bytes(
            &protected_bytes(CONTENT_TYPE, &ROOT_KEY),
            &payload,
            &SIGNATURE,
        );
        assert!(
            matches!(
                decode_fields(&mutant),
                Err(TrustError::DelegationVerification)
            ),
            "role {role} was carried through the decoder"
        );
        refused += 1;
    }
    assert_eq!(refused, 4, "every unknown role must have been tried");
}

#[test]
fn decode_refuses_a_permuted_payload_map_with_no_help_from_the_byte_compare() {
    // The positional slice pattern with pinned labels is what refuses this — NOT
    // the byte-compare, which an adversarial review proved by deleting the
    // byte-compare and watching both permutation tests still pass. This case is
    // below the byte-compare so the decode pins are the only thing that can fire.
    let delegated = delegated_key();
    let mut permuted = vec![0xa5, 0x02, 0x58, 0x20];
    permuted.extend_from_slice(&delegated);
    permuted.extend_from_slice(&[0x01, 0x6c]);
    permuted.extend_from_slice(b"example.test");
    permuted.extend_from_slice(&[0x03, 0x01, 0x04, 0x1b]);
    permuted.extend_from_slice(&1_700_000_000_000u64.to_be_bytes());
    permuted.extend_from_slice(&[0x05, 0x19, 0x01, 0x2c]);
    assert_eq!(
        permuted.len(),
        payload_bytes(&claim()).len(),
        "a permutation, not a different payload"
    );

    let mutant = envelope_bytes(
        &protected_bytes(CONTENT_TYPE, &ROOT_KEY),
        &permuted,
        &SIGNATURE,
    );
    assert!(matches!(
        decode_fields(&mutant),
        Err(TrustError::DelegationVerification)
    ));
}

#[test]
fn decode_refuses_an_empty_origin_with_no_help_from_the_byte_compare() {
    // A verifier whose configured origin is unset would match this and accept
    // it, which is why the rule is about the verifier rather than about the
    // string. Hand-assembled, because the encoder now refuses to build it.
    let mut payload = vec![0xa5, 0x01, 0x60, 0x02, 0x58, 0x20]; // origin => text(0)
    payload.extend_from_slice(&delegated_key());
    payload.extend_from_slice(&[0x03, 0x01, 0x04, 0x1b]);
    payload.extend_from_slice(&1_700_000_000_000u64.to_be_bytes());
    payload.extend_from_slice(&[0x05, 0x19, 0x01, 0x2c]);

    let mutant = envelope_bytes(
        &protected_bytes(CONTENT_TYPE, &ROOT_KEY),
        &payload,
        &SIGNATURE,
    );
    assert!(matches!(
        decode_fields(&mutant),
        Err(TrustError::DelegationVerification)
    ));

    // Positive control: a one-character origin, same construction, decodes — so
    // the refusal is about emptiness and not about the hand assembly.
    let mut ok_payload = vec![0xa5, 0x01, 0x61, b'x', 0x02, 0x58, 0x20];
    ok_payload.extend_from_slice(&delegated_key());
    ok_payload.extend_from_slice(&[0x03, 0x01, 0x04, 0x1b]);
    ok_payload.extend_from_slice(&1_700_000_000_000u64.to_be_bytes());
    ok_payload.extend_from_slice(&[0x05, 0x19, 0x01, 0x2c]);
    let ok = envelope_bytes(
        &protected_bytes(CONTENT_TYPE, &ROOT_KEY),
        &ok_payload,
        &SIGNATURE,
    );
    assert_eq!(decode_fields(&ok).unwrap().claim.origin, "x");
}

#[test]
fn decode_refuses_an_unusable_delegated_key_with_no_help_from_the_byte_compare() {
    // All-zeros, all-0xff and the canonically-encoded identity point. Each is a
    // key `verify_strict` can never accept, so a delegation naming one is a
    // signed statement that could never authorise anything — the same failure
    // §2.3 refuses for unknown roles, in a different field.
    let canonical = payload_bytes(&claim());
    let delegated = delegated_key();
    let at = canonical
        .windows(KEY_LEN)
        .position(|w| w == delegated)
        .expect("the delegated key is in the payload");

    let mut refused = 0;
    for bad_key in unusable_keys() {
        let mut payload = canonical.clone();
        payload[at..at + KEY_LEN].copy_from_slice(&bad_key);
        let mutant = envelope_bytes(
            &protected_bytes(CONTENT_TYPE, &ROOT_KEY),
            &payload,
            &SIGNATURE,
        );
        assert!(
            matches!(
                decode_fields(&mutant),
                Err(TrustError::DelegationVerification)
            ),
            "an unusable delegated key was carried through the decoder"
        );
        refused += 1;
    }
    assert_eq!(refused, 3, "every unusable key shape must have been tried");

    // Positive control: the real key in the same slot decodes.
    decode_fields(&envelope_bytes(
        &protected_bytes(CONTENT_TYPE, &ROOT_KEY),
        &canonical,
        &SIGNATURE,
    ))
    .unwrap();
}

#[test]
fn decode_refuses_a_kid_of_the_wrong_length_with_no_help_from_the_byte_compare() {
    for len in [0u8, 31, 33] {
        let mut protected = vec![0xa3, 0x01, 0x27, 0x03, 0x78, 0x2d];
        protected.extend_from_slice(CONTENT_TYPE.as_bytes());
        protected.extend_from_slice(&[0x04, 0x58, len]);
        protected.extend(std::iter::repeat_n(0x11u8, usize::from(len)));
        let mutant = envelope_bytes(&protected, &payload_bytes(&claim()), &SIGNATURE);
        assert!(
            matches!(
                decode_fields(&mutant),
                Err(TrustError::DelegationVerification)
            ),
            "a {len}-byte kid was accepted"
        );
    }
}

/// The offset of the `03` role label inside a canonical payload, located rather
/// than assumed — the preceding fields are variable-length.
fn role_offset(payload: &[u8]) -> usize {
    let matches = payload.windows(2).filter(|w| *w == [0x03, 0x01]).count();
    assert_eq!(matches, 1, "the role entry must be locatable unambiguously");
    payload
        .windows(2)
        .position(|w| w == [0x03, 0x01])
        .expect("role entry present")
}
