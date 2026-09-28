#![cfg(test)]
//! The bytes the encoder writes: the content type, the protected bucket,
//! the payload, the shortest-form integer heads, the envelope, and the frozen
//! wire values of the role and the subject kind.

use super::*;

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
