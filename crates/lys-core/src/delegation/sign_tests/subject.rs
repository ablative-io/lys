#![cfg(test)]
//! The subject: origin binding, the two subject kinds, the kind and role
//! table, and the empty subject value and unusable delegated key at
//! verification.

use super::*;

// ---------------------------------------------------------------------------
// §3.3 — cross-origin replay.
// ---------------------------------------------------------------------------

#[test]
fn a_delegation_for_another_origin_is_refused_though_it_is_internally_perfect() {
    // The injection from §6.2: payload origin changed. Same root key, same
    // delegated key, same role — only the origin differs, and the signature over
    // it is genuine.
    let key = root();
    let elsewhere = sign_delegation(&key, &claim_for(OTHER_ORIGIN)).unwrap();

    // It is a perfectly good delegation for the origin it names.
    verify_delegation(&elsewhere, &key.public_key_bytes(), DOMAIN, OTHER_ORIGIN).unwrap();

    // And is not one for ours, even though the root key is the one we trust —
    // the case an operator sharing a root key across anchors actually hits.
    assert!(matches!(
        verify_delegation(&elsewhere, &key.public_key_bytes(), DOMAIN, ORIGIN),
        Err(TrustError::DelegationVerification)
    ));

    // Origin comparison is exact: no prefix, suffix or case relaxation.
    let mut refused = 0;
    for near_miss in [
        "example.tes",
        "example.test.",
        "Example.Test",
        "sub.example.test",
        " example.test",
    ] {
        let cose = sign_delegation(&key, &claim_for(near_miss)).unwrap();
        assert!(
            verify_delegation(&cose, &key.public_key_bytes(), DOMAIN, ORIGIN).is_err(),
            "origin {near_miss:?} must not satisfy {ORIGIN:?}"
        );
        refused += 1;
    }
    assert_eq!(refused, 5, "every near-miss origin must have been tried");
}

#[test]
fn origin_comparison_is_raw_utf8_byte_equality() {
    // The test vector's origin is ASCII and so cannot expose this; an
    // internationalised one can. `café.test` in NFC (U+00E9) and in NFD
    // (`e` + U+0301) render identically and are different byte strings — a
    // Unicode-normalising comparison would treat them as one origin, and an
    // attacker holding a delegation for either would then hold one for both.
    let key = root();
    let nfc = "caf\u{e9}.test";
    let nfd = "cafe\u{301}.test";
    assert_ne!(nfc.as_bytes(), nfd.as_bytes());

    let cose = sign_delegation(&key, &claim_for(nfc)).unwrap();
    // Positive control: it verifies against its own byte string.
    verify_delegation(&cose, &key.public_key_bytes(), DOMAIN, nfc).unwrap();
    assert!(verify_delegation(&cose, &key.public_key_bytes(), DOMAIN, nfd).is_err());

    // And the reverse direction, so neither form is privileged.
    let cose_nfd = sign_delegation(&key, &claim_for(nfd)).unwrap();
    verify_delegation(&cose_nfd, &key.public_key_bytes(), DOMAIN, nfd).unwrap();
    assert!(verify_delegation(&cose_nfd, &key.public_key_bytes(), DOMAIN, nfc).is_err());
}

// ---------------------------------------------------------------------------
// §3.3 — cross-KIND replay, which is the point of typing the subject.
// ---------------------------------------------------------------------------

/// ⛔ **The collision case.** A seat delegation and a domain delegation naming
/// the **same subject string** must not be interchangeable.
///
/// A verifier that compared only the value would accept the seat delegation as
/// authority over the origin. This is not a contrived overlap: a seat identifier
/// is arbitrary text minted by a registry elsewhere, so an attacker who can
/// choose one chooses the target's origin string. The kind argument is what makes
/// the two namespaces disjoint by construction instead of by hope.
#[test]
fn the_two_subject_kinds_do_not_interchange_even_at_the_same_subject_value() {
    let key = root();
    let expected = key.public_key_bytes();

    let domain = sign_delegation(&key, &claim_for(ORIGIN)).unwrap();
    let seat = sign_delegation(&key, &seat_claim_for(ORIGIN)).unwrap();

    // The premise, asserted rather than assumed: one string, two artifacts.
    assert_eq!(
        Delegation::from_cose_bytes(&domain)
            .unwrap()
            .claim
            .subject_value,
        Delegation::from_cose_bytes(&seat)
            .unwrap()
            .claim
            .subject_value,
        "the two artifacts must name the same subject VALUE for this test to be \
         about the KIND"
    );
    assert_ne!(domain, seat, "and they must still be different artifacts");

    // Positive controls: each verifies under its own kind. Without these, a
    // verifier that refused both would satisfy the two refusals below.
    let ok_domain = verify_delegation(&domain, &expected, DOMAIN, ORIGIN).unwrap();
    assert_eq!(ok_domain.claim.subject_kind, DelegationSubjectKind::Domain);
    assert_eq!(ok_domain.claim.role, DelegationRole::Operational);
    let ok_seat = verify_delegation(&seat, &expected, SEAT, ORIGIN).unwrap();
    assert_eq!(ok_seat.claim.subject_kind, DelegationSubjectKind::Seat);
    assert_eq!(ok_seat.claim.role, DelegationRole::SpeaksFor);

    // And neither is accepted by the other's verifier, though both are signed by
    // the root key that verifier trusts and both name the string it expects.
    let mut refused = 0;
    for (name, artifact, kind) in [
        ("a seat delegation at a domain verifier", &seat, DOMAIN),
        ("a domain delegation at a seat verifier", &domain, SEAT),
    ] {
        assert!(
            matches!(
                verify_delegation(artifact, &expected, kind, ORIGIN),
                Err(TrustError::DelegationVerification)
            ),
            "{name} was accepted"
        );
        refused += 1;
    }
    assert_eq!(refused, 2, "both directions must have been tried");
}

// ---------------------------------------------------------------------------
// §1.2 / §3.3 — the (subject_kind, role) pair.
// ---------------------------------------------------------------------------

/// Spec §2.3 clause 3, with a genuine signature: the two pairs outside the table
/// whose halves are each individually recognised.
///
/// Only those two — the transposition cases fail *role decode* and belong to
/// `encoding_tests::transposing_the_subject_kind_and_role_labels_fails_role_decode_not_the_pair_check`,
/// which asserts that attribution. Bundling them here would make this test pass
/// for the wrong reason and would leave clause 3 unproven by any single case.
#[test]
fn a_pair_outside_the_table_is_refused_though_its_signature_is_valid() {
    let key = root();
    let expected = key.public_key_bytes();
    let canonical = encoding::payload_bytes(&claim());
    assert_eq!(&canonical[..3], &[0xa6, 0x01, 0x01], "kind at index 2");
    let role_at = role_offset(&canonical);

    // (kind byte, role byte, why it must be refused)
    let cases = [
        (
            0x01u8,
            0x03u8,
            "domain + speaks-for: both values valid, pair is not",
        ),
        (
            0x02,
            0x02,
            "seat + operational: both values valid, pair is not",
        ),
    ];

    let mut refused = 0;
    for (kind, role, why) in cases {
        // The premise of clause 3: each half decodes on its own.
        let kind_enum = DelegationSubjectKind::from_wire(u64::from(kind)).expect("kind decodes");
        let role_enum = DelegationRole::from_wire(u64::from(role)).expect("role decodes");
        assert!(!kind_enum.permits(role_enum), "{why}");
        let mut payload = canonical.clone();
        payload[2] = kind;
        payload[role_at + 1] = role;
        assert_eq!(payload.len(), canonical.len(), "a repair, not a resize");
        let mutant = signed_envelope(&protected_for(&key), &payload, &key);

        // The mutant is cryptographically perfect: only the pair rule can refuse
        // it. Proven rather than asserted.
        Ed25519Identity::verify(
            &expected,
            &cbor::sig_structure_bytes(&protected_for(&key), &payload),
            &mutant[mutant.len() - 64..],
        )
        .unwrap();

        // Refused under BOTH kinds a caller could name, so no configuration
        // accepts it.
        for kind_expected in [DOMAIN, SEAT] {
            assert!(
                matches!(
                    verify_delegation(&mutant, &expected, kind_expected, ORIGIN),
                    Err(TrustError::DelegationVerification)
                ),
                "({kind}, {role}) was accepted — {why}"
            );
        }
        refused += 1;
    }
    assert_eq!(refused, 2, "both clause-3 pairs must have been tried");

    // Positive controls through the identical construction: the two VALID pairs
    // verify. Without them these refusals would also be satisfied by a decoder
    // that had stopped parsing payloads at all.
    let mut accepted = 0;
    for (kind, role, kind_expected) in [(0x01u8, 0x02u8, DOMAIN), (0x02, 0x03, SEAT)] {
        let mut payload = canonical.clone();
        payload[2] = kind;
        payload[role_at + 1] = role;
        let ok = signed_envelope(&protected_for(&key), &payload, &key);
        verify_delegation(&ok, &expected, kind_expected, ORIGIN).unwrap();
        accepted += 1;
    }
    assert_eq!(
        accepted, 2,
        "both valid pairs must have been controlled for"
    );
}

#[test]
fn an_unusable_delegated_key_is_refused_at_verification_too() {
    // Hand-assembled and signed for real, because the encoder now refuses to
    // build one. A delegation naming a key that can never verify anything is a
    // signed statement with no possible meaning — the same failure the closed
    // role enum refuses, in a different field.
    let key = root();
    let canonical = encoding::payload_bytes(&claim());
    let delegated = operational().public_key_bytes();
    let at = canonical
        .windows(32)
        .position(|w| w == delegated)
        .expect("the delegated key is in the payload");

    let mut identity_point = [0u8; 32];
    identity_point[0] = 1;
    let mut refused = 0;
    for bad_key in [[0u8; 32], [0xffu8; 32], identity_point] {
        let mut payload = canonical.clone();
        payload[at..at + 32].copy_from_slice(&bad_key);
        let mutant = signed_envelope(&protected_for(&key), &payload, &key);
        assert!(
            matches!(
                verify_delegation(&mutant, &key.public_key_bytes(), DOMAIN, ORIGIN),
                Err(TrustError::DelegationVerification)
            ),
            "a delegation naming an unusable key was accepted"
        );
        refused += 1;
    }
    assert_eq!(refused, 3, "every unusable key shape must have been tried");

    // Positive control: the same hand-assembly with the real key verifies.
    let ok = signed_envelope(&protected_for(&key), &canonical, &key);
    verify_delegation(&ok, &key.public_key_bytes(), DOMAIN, ORIGIN).unwrap();
}

#[test]
fn an_empty_subject_value_is_refused_at_verification_too() {
    // The rule is about the VERIFIER, not the string: acceptance is a comparison
    // against the caller's configured subject, so a verifier whose subject is
    // unset would match an empty-subject delegation and accept it. Refusing at
    // decode makes that misconfiguration fail closed.
    let key = root();
    let mut payload = vec![0xa6, 0x01, 0x01, 0x02, 0x60, 0x03, 0x58, 0x20];
    payload.extend_from_slice(&operational().public_key_bytes());
    payload.extend_from_slice(&[0x04, 0x02, 0x05, 0x1b]);
    payload.extend_from_slice(&1_700_000_000_000u64.to_be_bytes());
    payload.extend_from_slice(&[0x06, 0x19, 0x01, 0x2c]);

    let mutant = signed_envelope(&protected_for(&key), &payload, &key);

    // The case that motivates the rule: a verifier that passes "" as its
    // expected subject value. Without the decode rule this would SUCCEED.
    assert!(matches!(
        verify_delegation(&mutant, &key.public_key_bytes(), DOMAIN, ""),
        Err(TrustError::DelegationVerification)
    ));
    // And with a real origin configured, naturally.
    assert!(verify_delegation(&mutant, &key.public_key_bytes(), DOMAIN, ORIGIN).is_err());
}
