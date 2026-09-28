#![cfg(test)]
//! Issuance: the one-shot and two-phase paths agree, issuance is deterministic,
//! and issuance refuses every claim the verifier would reject.

use super::*;

// ---------------------------------------------------------------------------
// The positive path. Without this, every rejection test below would be
// satisfied by a verifier that refuses everything.
// ---------------------------------------------------------------------------

#[test]
fn a_signed_delegation_verifies_and_carries_back_every_field() {
    let key = root();
    let cose = sign_delegation(&key, &claim()).unwrap();
    let verified = verify_delegation(&cose, &key.public_key_bytes(), DOMAIN, ORIGIN).unwrap();

    assert_eq!(verified.root_public_key, key.public_key_bytes());
    assert_eq!(verified.claim, claim());
    assert_eq!(
        verified.claim.delegated_public_key,
        operational().public_key_bytes()
    );
    assert_eq!(verified.claim.role, DelegationRole::Operational);
    // The parsed value re-encodes to exactly the bytes that were verified.
    assert_eq!(verified.to_cose_bytes(), cose);
}

#[test]
fn the_two_phase_path_produces_the_same_artifact_as_the_one_shot_path() {
    // The offline-signing path and the convenience must agree byte for byte, or
    // an air-gapped operator would produce artifacts nobody else can reproduce.
    let key = root();
    let root_key = key.public_key_bytes();
    let preimage = delegation_preimage(&root_key, &claim());
    let signature = key.sign(&preimage);
    let assembled = assemble_delegation(&root_key, &claim(), &signature).unwrap();
    assert_eq!(assembled, sign_delegation(&key, &claim()).unwrap());

    // And the preimage really is the COSE Sig_structure over both buckets.
    assert_eq!(
        preimage,
        cbor::sig_structure_bytes(&protected_for(&key), &encoding::payload_bytes(&claim()))
    );
    assert_eq!(&preimage[..12], b"\x84\x6aSignature1");

    // `external_aad` is pinned to the empty byte string `h''` (`0x40`). It sits
    // inside the signed bytes, so an unpinned value would mean two conforming
    // implementations produce different signatures over the same claim and
    // neither is wrong. Located structurally: context, then the protected bstr.
    let external_aad_at = 12 + 2 + encoding::PROTECTED_LEN;
    assert_eq!(
        preimage[external_aad_at], 0x40,
        "external_aad must be the zero-length byte string"
    );
    assert_eq!(
        &preimage[12..14],
        &[0x58, 0x4f],
        "the protected bstr head precedes it — 79 bytes"
    );
}

#[test]
fn assembly_refuses_a_signature_that_does_not_match_its_own_bytes() {
    // Catching this here is the whole point: an artifact whose signature does
    // not match its bytes is indistinguishable from a forgery to everyone who
    // receives it later.
    let key = root();
    let root_key = key.public_key_bytes();
    let good = key.sign(&delegation_preimage(&root_key, &claim()));

    let mut flipped = good;
    flipped[0] ^= 0x01;
    assert!(matches!(
        assemble_delegation(&root_key, &claim(), &flipped),
        Err(TrustError::DelegationVerification)
    ));

    // A signature by the wrong key, and one over a different claim, are refused
    // for the same reason.
    let wrong_signer = attacker().sign(&delegation_preimage(&root_key, &claim()));
    assert!(assemble_delegation(&root_key, &claim(), &wrong_signer).is_err());

    let wrong_claim = key.sign(&delegation_preimage(&root_key, &claim_for(OTHER_ORIGIN)));
    assert!(assemble_delegation(&root_key, &claim(), &wrong_claim).is_err());

    // Positive control: the untouched signature assembles.
    assert!(assemble_delegation(&root_key, &claim(), &good).is_ok());
}

#[test]
fn issuance_is_deterministic() {
    // Ed25519 signing is deterministic and the encoder is fixed-shape, so the
    // same inputs must produce byte-identical artifacts. One that varied run to
    // run could not be deduplicated, compared, or logged by its bytes.
    let key = root();
    assert_eq!(
        sign_delegation(&key, &claim()).unwrap(),
        sign_delegation(&key, &claim()).unwrap()
    );
}

/// The issuing side refuses the same pairs, through both entry points.
///
/// Without this, `sign_delegation` would happily mint an artifact its own
/// `verify_delegation` refuses — the defect class `check_encodable` exists to
/// close, arriving through the newest rule.
#[test]
fn issuance_refuses_an_invalid_kind_role_pair_through_both_entry_points() {
    let key = root();
    let root_key = key.public_key_bytes();

    let mut refused = 0;
    for (kind, role) in [
        (DelegationSubjectKind::Domain, DelegationRole::SpeaksFor),
        (DelegationSubjectKind::Seat, DelegationRole::Operational),
    ] {
        let mut c = claim();
        c.subject_kind = kind;
        c.role = role;
        assert!(
            matches!(
                sign_delegation(&key, &c),
                Err(TrustError::DelegationEncoding { .. })
            ),
            "({kind:?}, {role:?}) was issuable through the convenience path"
        );
        // And the air-gapped two-phase route, which does not go through
        // `sign_delegation` at all.
        let signature = key.sign(&delegation_preimage(&root_key, &c));
        assert!(
            matches!(
                assemble_delegation(&root_key, &c, &signature),
                Err(TrustError::DelegationEncoding { .. })
            ),
            "({kind:?}, {role:?}) was issuable through the two-phase path"
        );
        refused += 1;
    }
    assert_eq!(refused, 2, "both invalid pairs must have been tried");

    // Positive controls: both valid pairs issue and verify end to end.
    let mut issued = 0;
    for (c, kind_expected) in [(claim(), DOMAIN), (seat_claim_for(ORIGIN), SEAT)] {
        let cose = sign_delegation(&key, &c).unwrap();
        assert_eq!(
            verify_delegation(&cose, &root_key, kind_expected, &c.subject_value)
                .unwrap()
                .claim,
            c
        );
        issued += 1;
    }
    assert_eq!(issued, 2, "both valid pairs must have been issued");
}

// ---------------------------------------------------------------------------
// The encode side refuses everything the decode side refuses.
// ---------------------------------------------------------------------------

#[test]
fn issuance_refuses_a_claim_the_verifier_would_reject() {
    // The defect: the artifact cap was enforced at decode only, so a subject
    // value of 3884 bytes signed and verified while 3885 signed SUCCESSFULLY and failed
    // every verification afterwards — a file that fails at a later, far less
    // debuggable moment, which is precisely what assembling-with-verification
    // exists to prevent, arriving through the one door that check did not cover.
    //
    // The boundary is DERIVED here, never hardcoded: it moved by nine bytes when
    // `sequence` was added, and a literal would have gone stale in silence.
    let key = root();
    let root_key = key.public_key_bytes();

    let mut longest_ok = None;
    for len in (1..4000usize).rev() {
        if sign_delegation(&key, &claim_at(&"o".repeat(len), 300)).is_ok() {
            longest_ok = Some(len);
            break;
        }
    }
    let longest_ok = longest_ok.expect("some subject length must be issuable");

    // At the boundary: issuable AND verifiable. The pair is the point — either
    // one alone is what the defect looked like.
    let at_limit = claim_at(&"o".repeat(longest_ok), 300);
    let artifact = sign_delegation(&key, &at_limit).unwrap();
    verify_delegation(&artifact, &root_key, DOMAIN, &at_limit.subject_value).unwrap();

    // One byte over: refused at ISSUANCE, with an actionable reason rather than
    // the non-oracle value. This is the operator's side; no stranger is being
    // kept in the dark.
    let over = claim_at(&"o".repeat(longest_ok + 1), 300);
    let err = sign_delegation(&key, &over).unwrap_err();
    let TrustError::DelegationEncoding { reason } = &err else {
        panic!("expected an encoding error naming the constraint, got {err:?}");
    };
    assert!(
        reason.contains("cap"),
        "the reason must name the constraint, got: {reason}"
    );

    // And the two-phase path refuses it too, so the air-gapped route cannot
    // produce what the convenience route refuses.
    let signature = key.sign(&delegation_preimage(&root_key, &over));
    assert!(matches!(
        assemble_delegation(&root_key, &over, &signature),
        Err(TrustError::DelegationEncoding { .. })
    ));
}

#[test]
fn issuance_refuses_an_empty_subject_value_and_an_unusable_delegated_key() {
    let key = root();
    let root_key = key.public_key_bytes();

    let empty = claim_at("", 300);
    assert!(matches!(
        sign_delegation(&key, &empty),
        Err(TrustError::DelegationEncoding { .. })
    ));

    // All-zeros, all-0xff and the canonically-encoded identity point: three keys
    // strict Ed25519 verification can never accept, so a delegation naming one
    // could never authorise anything. Each was accepted before this check.
    let mut identity_point = [0u8; 32];
    identity_point[0] = 1;
    let mut refused = 0;
    for bad_key in [[0u8; 32], [0xffu8; 32], identity_point] {
        let mut c = claim();
        c.delegated_public_key = bad_key;
        assert!(
            matches!(
                sign_delegation(&key, &c),
                Err(TrustError::DelegationEncoding { .. })
            ),
            "an unusable delegated key was issuable"
        );
        // The two-phase path too.
        let signature = key.sign(&delegation_preimage(&root_key, &c));
        assert!(matches!(
            assemble_delegation(&root_key, &c, &signature),
            Err(TrustError::DelegationEncoding { .. })
        ));
        refused += 1;
    }
    assert_eq!(refused, 3, "every unusable key shape must have been tried");

    // Positive control: the real operational key issues.
    sign_delegation(&key, &claim()).unwrap();
}
