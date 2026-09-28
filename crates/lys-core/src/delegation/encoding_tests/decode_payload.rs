#![cfg(test)]
//! Decoder refusals in the payload fields, each reached with no help from
//! the byte compare: the role, the subject kind, the pair table, the subject
//! value and the delegated key.

use super::*;

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
