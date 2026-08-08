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

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;

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

#[test]
fn the_content_type_is_the_frozen_string() {
    assert_eq!(CONTENT_TYPE, "application/vnd.lys.delegation.v1+cbor");
    assert_eq!(CONTENT_TYPE.len(), 38);
    assert!(
        !CONTENT_TYPE.contains("anchor"),
        "the format serves seats as well as anchors: the v1 type was renamed \
         before publication and a signature-covered misnomer is permanent"
    );
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
    assert_eq!(built.len(), 79, "the protected bucket is always 79 bytes");
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
    assert_eq!(built[0], 0xa6, "six entries: sequence is label 6");
}

#[test]
fn the_subject_kind_and_role_wire_values_cannot_be_transposed_undetectably() {
    // ⛔ THE numbering test. A draft numbered domain = 1 / operational = 1 and
    // seat = 2 / speaks-for = 2, so `subject_kind == role` held for EVERY valid
    // v1 artifact — both fields are uints in one map, so an implementation that
    // wired label 1 into its role and label 4 into its kind would have emitted
    // byte-identical output for every valid delegation. No vector and no
    // round-trip could see that.
    //
    // This test fails if the numbering is reverted, which is the only reason it
    // exists: the defect is in the enum values, not in any encoded case.
    let mut pairs = 0;
    for (kind, role) in [
        (DelegationSubjectKind::Domain, DelegationRole::Operational),
        (DelegationSubjectKind::Seat, DelegationRole::SpeaksFor),
    ] {
        assert!(kind.permits(role), "this must be a valid pair");
        assert_ne!(
            kind.wire_value(),
            role.wire_value(),
            "a valid pair whose two fields share a wire value makes a \
             transposition of subject_kind and role invisible in the bytes"
        );
        pairs += 1;
    }
    assert_eq!(pairs, 2, "every valid pair must have been checked");

    // And the stronger property: a transposition is not merely different, it is
    // REFUSED — because the swapped pair is outside the table. `(1, 2)` swapped
    // is `(2, 1)`; `(2, 3)` swapped is `(3, 2)`.
    let mut swaps = 0;
    for (kind_value, role_value) in [(1u64, 2u64), (2, 3)] {
        // The swap, read back through the closed vocabularies: either it names
        // nothing this version defines, or it names a pair `permits` refuses.
        let swapped_ok = match (
            DelegationSubjectKind::from_wire(role_value),
            DelegationRole::from_wire(kind_value),
        ) {
            (Some(kind), Some(role)) => kind.permits(role),
            _ => false,
        };
        assert!(
            !swapped_ok,
            "transposing subject_kind {kind_value} and role {role_value} \
             produced a pair this version accepts"
        );
        swaps += 1;
    }
    assert_eq!(swaps, 2, "every valid pair must have been swapped");
}

#[test]
fn both_valid_pairs_encode_and_decode_and_carry_their_own_kind() {
    // The positive control for every pair rejection below: a verifier that
    // refused both kinds would satisfy all of them.
    let mut checked = 0;
    for (claim, kind_byte, role_byte) in [(claim(), 0x01u8, 0x02u8), (seat_claim(), 0x02, 0x03)] {
        let payload = payload_bytes(&claim);
        assert_eq!(payload[0], 0xa6);
        assert_eq!(
            &payload[1..3],
            &[0x01, kind_byte],
            "label 1 must carry this claim's subject kind"
        );
        // The role entry is located by its neighbours: label 4, then label 5.
        let role_at = role_offset(&payload);
        assert_eq!(
            &payload[role_at..role_at + 2],
            &[0x04, role_byte],
            "label 4 must carry this claim's role"
        );

        check_encodable(&ROOT_KEY, &claim).unwrap();
        let decoded = decode_fields(&artifact_bytes(&ROOT_KEY, &claim, &SIGNATURE))
            .unwrap()
            .claim;
        assert_eq!(decoded, claim);
        checked += 1;
    }
    assert_eq!(checked, 2, "both valid pairs must have been exercised");
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
            0x06,
            "label 6 must immediately precede the sequence value for {sequence}"
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
        // The tail is `05 <not_before head> 06 19 01 2c`: the fixture's
        // sequence of 300 always occupies the last four bytes.
        let at = payload.len() - SEQUENCE_ENTRY_LEN - expected_head.len();
        assert_eq!(
            &payload[payload.len() - SEQUENCE_ENTRY_LEN..],
            &[0x06, 0x19, 0x01, 0x2c],
            "the fixture's sequence entry must be where this offset assumes"
        );
        assert_eq!(payload[at - 1], 0x05, "label 5 precedes not_before");
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

    let mut expected = vec![0xd2, 0x84, 0x58, 0x4f]; // 18([ ... ]), bstr(79)
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
    assert_eq!(DelegationRole::Operational.wire_value(), 2);
    assert_eq!(DelegationRole::SpeaksFor.wire_value(), 3);
    assert_eq!(
        DelegationRole::from_wire(2),
        Some(DelegationRole::Operational)
    );
    assert_eq!(
        DelegationRole::from_wire(3),
        Some(DelegationRole::SpeaksFor)
    );
    // A closed enum: every other value is a decode failure, not a carried one.
    // `1` is among them ON PURPOSE — it is the domain kind's wire value, and the
    // role vocabulary is offset past it so that a transposition of the two fields
    // cannot produce a valid artifact.
    let mut refused = 0;
    for value in [0u64, 1, 4, 7, 255, u64::MAX] {
        assert!(DelegationRole::from_wire(value).is_none(), "role {value}");
        refused += 1;
    }
    assert_eq!(refused, 6, "every listed unknown role must have been tried");
}

#[test]
fn the_subject_kind_wire_values_are_the_frozen_mapping() {
    assert_eq!(DelegationSubjectKind::Domain.wire_value(), 1);
    assert_eq!(DelegationSubjectKind::Seat.wire_value(), 2);
    assert_eq!(
        DelegationSubjectKind::from_wire(1),
        Some(DelegationSubjectKind::Domain)
    );
    assert_eq!(
        DelegationSubjectKind::from_wire(2),
        Some(DelegationSubjectKind::Seat)
    );
    // Closed, on exactly the reasoning that closes `role`: an unrecognised kind
    // names a namespace nothing in this version can interpret. `3` is the
    // specification's own example of a v3 kind.
    let mut refused = 0;
    for value in [0u64, 3, 4, 7, 255, u64::MAX] {
        assert!(
            DelegationSubjectKind::from_wire(value).is_none(),
            "subject kind {value}"
        );
        refused += 1;
    }
    assert_eq!(refused, 6, "every listed unknown kind must have been tried");
}

#[test]
fn every_invalid_kind_role_pair_is_refused_and_only_the_table_is_accepted() {
    // The rule, exercised over the FULL cross product rather than over the two
    // cases someone would think to write. `(domain, speaks-for)` and
    // `(seat, operational)` are the ones made of individually valid values, and
    // they are the whole point: no field check can catch them.
    let mut accepted = 0;
    let mut refused = 0;
    for kind in [DelegationSubjectKind::Domain, DelegationSubjectKind::Seat] {
        for role in [DelegationRole::Operational, DelegationRole::SpeaksFor] {
            let expected = matches!(
                (kind, role),
                (DelegationSubjectKind::Domain, DelegationRole::Operational)
                    | (DelegationSubjectKind::Seat, DelegationRole::SpeaksFor)
            );
            assert_eq!(
                kind.permits(role),
                expected,
                "the pair table disagrees about ({kind:?}, {role:?})"
            );
            if expected {
                accepted += 1;
            } else {
                refused += 1;
            }
        }
    }
    assert_eq!(accepted, 2, "exactly two pairs are valid in v1");
    assert_eq!(refused, 2, "exactly two pairs are invalid in v1");
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
    // Find the longest subject value that fits, by construction rather than by a
    // hardcoded number. A literal here would have been correct before `sequence`
    // landed and silently wrong after it, and the typed subject moved it again.
    let mut longest_ok = None;
    let mut shortest_refused = None;
    for len in 3800..3900usize {
        let mut c = claim();
        c.subject_value = "o".repeat(len);
        let encoded_len = artifact_bytes(&ROOT_KEY, &c, &SIGNATURE).len();
        let accepted = check_encodable(&ROOT_KEY, &c).is_ok();
        assert_eq!(
            accepted,
            encoded_len <= MAX_ARTIFACT_LEN,
            "encode-side acceptance disagrees with the cap at subject length {len}"
        );
        // The decoder must reach the same verdict on the same claim.
        let decoded_ok = decode_fields(&artifact_bytes(&ROOT_KEY, &c, &SIGNATURE)).is_ok();
        assert_eq!(
            accepted, decoded_ok,
            "encode and decode disagree at subject length {len} — this is the \
             defect class the encode-side check exists to close"
        );
        if accepted {
            longest_ok = Some(len);
        } else if shortest_refused.is_none() {
            shortest_refused = Some(len);
        }
    }
    let longest_ok = longest_ok.expect("some subject length must fit");
    let shortest_refused = shortest_refused.expect("some subject length must not fit");
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
fn check_encodable_refuses_an_empty_subject_value_and_an_unusable_delegated_key() {
    let mut empty_subject = claim();
    empty_subject.subject_value = String::new();
    assert!(matches!(
        check_encodable(&ROOT_KEY, &empty_subject),
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

#[test]
fn check_encodable_refuses_an_invalid_kind_role_pair_with_an_actionable_reason() {
    // The encode side must refuse everything the decode side refuses, or an
    // issuing path can mint an artifact that fails every verification afterwards
    // — the defect class this function exists to close, reached through the
    // newest rule. Both invalid pairs are made of individually valid values.
    let mut refused = 0;
    for (kind, role) in [
        (DelegationSubjectKind::Domain, DelegationRole::SpeaksFor),
        (DelegationSubjectKind::Seat, DelegationRole::Operational),
    ] {
        let mut c = claim();
        c.subject_kind = kind;
        c.role = role;
        let err = check_encodable(&ROOT_KEY, &c).unwrap_err();
        let TrustError::DelegationEncoding { reason } = &err else {
            panic!("expected an encoding error naming the constraint, got {err:?}");
        };
        assert!(
            reason.contains("pair"),
            "the reason must name the rule being enforced, got: {reason}"
        );
        refused += 1;
    }
    assert_eq!(refused, 2, "both invalid pairs must have been tried");

    // Positive controls: both valid pairs encode, so the refusals are about the
    // pairing and not about either value on its own.
    check_encodable(&ROOT_KEY, &claim()).unwrap();
    check_encodable(&ROOT_KEY, &seat_claim()).unwrap();
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
// `Delegation::from_cose_bytes`, because the canonical re-encoding always
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
        "application/vnd.lys.delegation.v2+cbor",
        // The name this format carried before the typed subject landed. A
        // verifier that still accepted it would accept a payload shaped to the
        // old five-label numbering.
        "application/vnd.lys.anchor-delegation.v1+cbor",
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

    // `1` is included deliberately: it is the DOMAIN kind's wire value, so a v1
    // artifact whose role field carries it is exactly what a transposed
    // implementation would emit.
    let mut refused = 0;
    for role in [0u8, 1, 4, 7, 23] {
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
    assert_eq!(refused, 5, "every unknown role must have been tried");
}

#[test]
fn decode_refuses_an_unknown_subject_kind_with_no_help_from_the_byte_compare() {
    // Label 1 is the payload's first entry, so its value byte is at index 2 —
    // asserted rather than assumed.
    let canonical = payload_bytes(&claim());
    assert_eq!(&canonical[..3], &[0xa6, 0x01, 0x01]);

    let mut refused = 0;
    for kind in [0u8, 3, 4, 7, 23] {
        let mut payload = canonical.clone();
        payload[2] = kind;
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
            "subject kind {kind} was carried through the decoder"
        );
        refused += 1;
    }
    assert_eq!(
        refused, 5,
        "every unknown subject kind must have been tried"
    );
}

/// Spec §2.3 **clause 3** — a pair outside the table, both halves of which are
/// individually recognised.
///
/// ⛔ **This clause needs its own case because neither single-field check can
/// reach it**, and the two cases below are the only two such pairs `v1` admits:
/// `(1 domain, 3 speaks-for)` and `(2 seat, 2 operational)`. Each half is asserted
/// to decode on its own *first*, so a failure here is provably the pair check and
/// not a mistyped byte.
///
/// A second property this test carries alone: the caller's
/// re-encode-and-byte-compare cannot mask this rule, because the canonical
/// re-encoding of an invalid pair **is** the invalid pair. Delete the check and
/// `Delegation::from_cose_bytes` accepts these artifacts too — unlike the
/// content-type pin, the unprotected-bucket rule and the tag rule, all of which
/// the byte-compare covers for.
#[test]
fn decode_refuses_a_pair_outside_the_table_though_both_halves_decode() {
    let canonical = payload_bytes(&claim());
    assert_eq!(
        &canonical[..3],
        &[0xa6, 0x01, 0x01],
        "kind value at index 2"
    );
    let role_at = role_offset(&canonical);

    let mut refused = 0;
    for (kind, role) in [(1u64, 3u64), (2, 2)] {
        // Each half individually recognised — the premise, asserted rather than
        // assumed, so this test cannot silently degenerate into a single-field
        // rejection wearing a pair's name.
        let kind_enum = DelegationSubjectKind::from_wire(kind)
            .expect("clause 3 requires a RECOGNISED subject kind");
        let role_enum =
            DelegationRole::from_wire(role).expect("clause 3 requires a RECOGNISED role");
        assert!(
            !kind_enum.permits(role_enum),
            "({kind}, {role}) must be outside the pair table"
        );

        let mut payload = canonical.clone();
        payload[2] = u8::try_from(kind).unwrap();
        payload[role_at + 1] = u8::try_from(role).unwrap();
        assert_eq!(payload.len(), canonical.len(), "a repair, not a resize");
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
            "the pair (kind {kind}, role {role}) was carried through the decoder"
        );
        refused += 1;
    }
    assert_eq!(refused, 2, "both invalid pairs must have been tried");

    // Positive controls, built the same way: both VALID pairs decode through this
    // exact construction, so the refusals above are about the pairing rather than
    // about the hand-patched payload.
    let mut accepted = 0;
    for (kind, role, expected) in [(0x01u8, 0x02u8, claim()), (0x02, 0x03, seat_claim())] {
        let mut payload = canonical.clone();
        payload[2] = kind;
        payload[role_at + 1] = role;
        let ok = envelope_bytes(
            &protected_bytes(CONTENT_TYPE, &ROOT_KEY),
            &payload,
            &SIGNATURE,
        );
        assert_eq!(decode_fields(&ok).unwrap().claim, expected);
        accepted += 1;
    }
    assert_eq!(
        accepted, 2,
        "both valid pairs must have been controlled for"
    );
}

/// Spec §2.3 **clause 2**, reached by transposing labels 1 and 4 — and this test
/// exists to record *which* clause catches it.
///
/// ⭐ **The value of this case is that it can be written at all.** Under a
/// numbering where `domain = 1` and `operational = 1`, transposing the two labels
/// produced bytes identical to a valid artifact, so there was no injection to
/// write and no test could have existed. Offsetting the role vocabulary is what
/// makes the transposition *reachable*, and then refusable.
///
/// It fails **role decode**, not the pair check: `1` is not a role, so no
/// [`DelegationRole`] value is ever produced and no pair is ever formed. That is
/// asserted below rather than assumed, because attributing this to clause 3 would
/// credit the pair rule with a rejection it never performed.
#[test]
fn transposing_the_subject_kind_and_role_labels_fails_role_decode_not_the_pair_check() {
    let canonical = payload_bytes(&claim());
    let role_at = role_offset(&canonical);
    assert_eq!(canonical[2], 0x01, "domain is wire value 1");
    assert_eq!(canonical[role_at + 1], 0x02, "operational is wire value 2");

    // The transposition of the anchor's own pair: kind 2, role 1.
    let mut payload = canonical.clone();
    payload[2] = 0x02;
    payload[role_at + 1] = 0x01;
    assert_eq!(payload.len(), canonical.len());

    // WHICH clause fires: `1` is not a role at all, so decoding stops at the role
    // and the pair check is never reached. The kind half, by contrast, decodes.
    assert!(
        DelegationRole::from_wire(1).is_none(),
        "1 must not be a role — this is what makes the vocabularies disjoint"
    );
    assert!(
        DelegationSubjectKind::from_wire(2).is_some(),
        "the transposed kind half is still a recognised kind, so the rejection is \
         attributable to the role alone"
    );

    let mutant = envelope_bytes(
        &protected_bytes(CONTENT_TYPE, &ROOT_KEY),
        &payload,
        &SIGNATURE,
    );
    assert!(matches!(
        decode_fields(&mutant),
        Err(TrustError::DelegationVerification)
    ));

    // And the other valid pair's transposition, `(3, 2)`: an unrecognised KIND
    // this time, so the mirror case fires on clause 1.
    let seat = payload_bytes(&seat_claim());
    let seat_role_at = role_offset(&seat);
    let mut swapped = seat.clone();
    swapped[2] = 0x03;
    swapped[seat_role_at + 1] = 0x02;
    assert!(
        DelegationSubjectKind::from_wire(3).is_none(),
        "3 must not be a subject kind"
    );
    assert!(matches!(
        decode_fields(&envelope_bytes(
            &protected_bytes(CONTENT_TYPE, &ROOT_KEY),
            &swapped,
            &SIGNATURE,
        )),
        Err(TrustError::DelegationVerification)
    ));

    // Positive control: untransposed, both decode.
    decode_fields(&envelope_bytes(
        &protected_bytes(CONTENT_TYPE, &ROOT_KEY),
        &canonical,
        &SIGNATURE,
    ))
    .unwrap();
    decode_fields(&envelope_bytes(
        &protected_bytes(CONTENT_TYPE, &ROOT_KEY),
        &seat,
        &SIGNATURE,
    ))
    .unwrap();
}

/// ⚠️ Attribution, not coverage: spec §6.2's *"`subject_kind` changed to `2`, value
/// left alone"* row is **not** caught by §3.3's cross-kind arm. It is caught at
/// decode, by the pair rule.
///
/// Changing only the kind byte of a domain delegation yields `(2 seat,
/// 2 operational)` — a pair outside the table — so the artifact never reaches a
/// subject comparison at all. The isolating case for §3.3's kind arm has to be a
/// **genuinely valid** seat delegation, which is
/// `sign_tests::the_two_subject_kinds_do_not_interchange_even_at_the_same_subject_value`.
///
/// This test exists so that nobody reads the §6.2 row and credits the verifier's
/// kind check with a rejection the decoder performed — the same
/// wrong-argument-for-a-real-rule failure the byte-compare's history records.
#[test]
fn changing_only_the_subject_kind_byte_is_caught_by_the_pair_rule_not_the_kind_check() {
    let canonical = payload_bytes(&claim());
    let role_at = role_offset(&canonical);
    let mut payload = canonical.clone();
    payload[2] = 0x02; // seat, with the role left at operational

    let kind = DelegationSubjectKind::from_wire(2).expect("seat is a recognised kind");
    let role = DelegationRole::from_wire(u64::from(canonical[role_at + 1]))
        .expect("operational is a recognised role");
    assert!(
        !kind.permits(role),
        "the mutant's pair must be the thing that is wrong with it"
    );

    assert!(matches!(
        decode_fields(&envelope_bytes(
            &protected_bytes(CONTENT_TYPE, &ROOT_KEY),
            &payload,
            &SIGNATURE,
        )),
        Err(TrustError::DelegationVerification)
    ));
}

#[test]
fn decode_refuses_a_permuted_payload_map_with_no_help_from_the_byte_compare() {
    // The positional slice pattern with pinned labels is what refuses this — NOT
    // the byte-compare, which an adversarial review proved by deleting the
    // byte-compare and watching both permutation tests still pass. This case is
    // below the byte-compare so the decode pins are the only thing that can fire.
    let delegated = delegated_key();
    let mut permuted = vec![0xa6, 0x02, 0x6c];
    permuted.extend_from_slice(b"example.test");
    permuted.extend_from_slice(&[0x01, 0x01, 0x03, 0x58, 0x20]);
    permuted.extend_from_slice(&delegated);
    permuted.extend_from_slice(&[0x04, 0x02, 0x05, 0x1b]);
    permuted.extend_from_slice(&1_700_000_000_000u64.to_be_bytes());
    permuted.extend_from_slice(&[0x06, 0x19, 0x01, 0x2c]);
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
fn decode_refuses_an_empty_subject_value_with_no_help_from_the_byte_compare() {
    // A verifier whose configured subject is unset would match this and accept
    // it, which is why the rule is about the verifier rather than about the
    // string. Hand-assembled, because the encoder now refuses to build it.
    let mut payload = vec![0xa6, 0x01, 0x01, 0x02, 0x60, 0x03, 0x58, 0x20]; // value => text(0)
    payload.extend_from_slice(&delegated_key());
    payload.extend_from_slice(&[0x04, 0x02, 0x05, 0x1b]);
    payload.extend_from_slice(&1_700_000_000_000u64.to_be_bytes());
    payload.extend_from_slice(&[0x06, 0x19, 0x01, 0x2c]);

    let mutant = envelope_bytes(
        &protected_bytes(CONTENT_TYPE, &ROOT_KEY),
        &payload,
        &SIGNATURE,
    );
    assert!(matches!(
        decode_fields(&mutant),
        Err(TrustError::DelegationVerification)
    ));

    // Positive control: a one-character subject value, same construction, decodes
    // — so the refusal is about emptiness and not about the hand assembly.
    let mut ok_payload = vec![0xa6, 0x01, 0x01, 0x02, 0x61, b'x', 0x03, 0x58, 0x20];
    ok_payload.extend_from_slice(&delegated_key());
    ok_payload.extend_from_slice(&[0x04, 0x02, 0x05, 0x1b]);
    ok_payload.extend_from_slice(&1_700_000_000_000u64.to_be_bytes());
    ok_payload.extend_from_slice(&[0x06, 0x19, 0x01, 0x2c]);
    let ok = envelope_bytes(
        &protected_bytes(CONTENT_TYPE, &ROOT_KEY),
        &ok_payload,
        &SIGNATURE,
    );
    assert_eq!(decode_fields(&ok).unwrap().claim.subject_value, "x");
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
fn decode_refuses_an_unusable_kid_with_no_help_from_the_byte_compare() {
    // ⛔ The slot the module docs claimed length alone protected. It does not:
    // every key below is exactly 32 bytes and none is a point strict Ed25519
    // verification could accept, so before this rule existed each parsed into a
    // `kid` and became `Delegation::root_public_key` — a field named for a
    // key, holding something that is not one.
    //
    // Distinct from the delegated-key case below, and each must be able to fail
    // alone: this one patches the PROTECTED bucket and leaves the payload
    // canonical; that one patches the payload and leaves `kid` canonical.
    let canonical_payload = payload_bytes(&claim());

    let mut refused = 0;
    for bad_key in unusable_keys() {
        let mutant = envelope_bytes(
            &protected_bytes(CONTENT_TYPE, &bad_key),
            &canonical_payload,
            &SIGNATURE,
        );
        assert!(
            matches!(
                decode_fields(&mutant),
                Err(TrustError::DelegationVerification)
            ),
            "an unusable kid was carried through the decoder"
        );
        refused += 1;
    }
    assert_eq!(refused, 3, "every unusable key shape must have been tried");

    // Positive control: the same construction with a usable key decodes, so the
    // refusals are about the point and not about the hand assembly. ROOT_KEY is
    // asserted usable rather than assumed — the fixture predates this rule.
    assert!(
        crate::keys::identity::is_usable_ed25519_public_key(&ROOT_KEY),
        "the fixture root key must be a usable point, or this control proves nothing"
    );
    decode_fields(&envelope_bytes(
        &protected_bytes(CONTENT_TYPE, &ROOT_KEY),
        &canonical_payload,
        &SIGNATURE,
    ))
    .unwrap();
}

#[test]
fn check_encodable_refuses_an_unusable_root_key_with_an_actionable_reason() {
    // The encode-side mirror. Without it, adding the decode rule would have made
    // the crate's *other* invariant false — "every constraint the decoder
    // enforces is refused at encode too" — which is how repairing one false
    // invariant creates the next one.
    let mut refused = 0;
    for bad_key in unusable_keys() {
        let err = check_encodable(&bad_key, &claim()).unwrap_err();
        let TrustError::DelegationEncoding { reason } = &err else {
            panic!("expected an encoding error naming the constraint, got {err:?}");
        };
        assert!(
            reason.contains("root public key"),
            "the reason must name the slot at fault, got: {reason}"
        );
        refused += 1;
    }
    assert_eq!(refused, 3, "every unusable key shape must have been tried");

    // Positive control.
    check_encodable(&ROOT_KEY, &claim()).unwrap();
}

#[test]
fn decode_refuses_a_kid_of_the_wrong_length_with_no_help_from_the_byte_compare() {
    for len in [0u8, 31, 33] {
        let mut protected = vec![0xa3, 0x01, 0x27, 0x03, 0x78, 0x26];
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
