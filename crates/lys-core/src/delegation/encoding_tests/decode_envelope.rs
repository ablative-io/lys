#![cfg(test)]
//! Decoder refusals in the envelope and the protected bucket, each reached
//! with no help from the byte compare: the tag, the content type, the
//! unprotected bucket, the payload map order and the `kid`.

use super::*;

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
