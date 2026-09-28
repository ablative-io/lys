#![cfg(test)]
//! The pair table, the size boundary and the sequence ceiling, checked at
//! both ends: `check_encodable` on the issuing side and the decoder on the
//! verifying side.

use super::*;

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
