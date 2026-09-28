#![cfg(test)]
//! The envelope and its key: artifact-family confusion, the `kid`, the COSE
//! tag, the payload map order, the unprotected bucket, the role, and the
//! signature bytes, each refused though every mutant is validly signed.

use super::*;

// ---------------------------------------------------------------------------
// §3.1 — cross-format confusion.
// ---------------------------------------------------------------------------

#[test]
fn a_delegation_relabelled_as_a_receipt_is_refused_though_its_signature_is_valid() {
    // The injection from §6.2: content type changed to the receipt's. The root
    // key signs the relabelled bucket for real, so the artifact is
    // cryptographically perfect and only the content-type pin can refuse it.
    let key = root();
    let relabelled = encoding::protected_bytes(
        "application/vnd.lys.receipt.v1+cbor",
        &key.public_key_bytes(),
    );
    let mutant = signed_envelope(&relabelled, &encoding::payload_bytes(&claim()), &key);

    // The mutant's own signature checks out — this is not a forgery.
    Ed25519Identity::verify(
        &key.public_key_bytes(),
        &cbor::sig_structure_bytes(&relabelled, &encoding::payload_bytes(&claim())),
        &mutant[mutant.len() - 64..],
    )
    .unwrap();

    assert!(matches!(
        verify_delegation(&mutant, &key.public_key_bytes(), DOMAIN, ORIGIN),
        Err(TrustError::DelegationVerification)
    ));
}

#[test]
fn a_real_receipt_is_not_a_delegation_and_a_delegation_is_not_a_receipt() {
    // Both directions, on genuine artifacts issued by the same key — the
    // strongest form of the cross-format claim, since key separation is doing
    // none of the work.
    let key = root();

    let mut tree = AppendOnlyTree::<RawLeaf>::new();
    tree.append_raw(b"genesis");
    tree.append_raw(b"leaf");
    let path: Vec<[u8; 32]> = tree
        .prove_inclusion(1)
        .unwrap()
        .as_bytes()
        .chunks_exact(32)
        .map(|c| <[u8; 32]>::try_from(c).unwrap())
        .collect();
    let receipt_bytes = receipt::sign_receipt(b"leaf", 1, 2, &path, &key)
        .unwrap()
        .to_cose_bytes();
    let delegation_bytes = sign_delegation(&key, &claim()).unwrap();

    // Each verifies as itself: the positive control for this test.
    receipt::verify_receipt_bytes(&receipt_bytes, b"leaf", &key.public_key_bytes()).unwrap();
    verify_delegation(&delegation_bytes, &key.public_key_bytes(), DOMAIN, ORIGIN).unwrap();

    // Neither verifies as the other.
    assert!(verify_delegation(&receipt_bytes, &key.public_key_bytes(), DOMAIN, ORIGIN).is_err());
    assert!(
        receipt::verify_receipt_bytes(&delegation_bytes, b"leaf", &key.public_key_bytes()).is_err()
    );

    // And the preimages share their first twelve bytes, which is exactly why the
    // separation has to live inside the signed bytes rather than in a prefix.
    let delegation_preimage_bytes =
        cbor::sig_structure_bytes(&protected_for(&key), &encoding::payload_bytes(&claim()));
    assert_eq!(&delegation_preimage_bytes[..12], b"\x84\x6aSignature1");
}

// ---------------------------------------------------------------------------
// §3.2 — THE CENTRAL TRAP. `kid` is a claim, not an authority.
// ---------------------------------------------------------------------------

#[test]
fn a_delegation_signed_by_an_attacker_naming_their_own_key_is_refused() {
    // The injection from §6.2: `kid` replaced with the attacker's root key.
    //
    // The attacker signs a delegation for OUR origin with THEIR root key and
    // puts THEIR key in `kid`. The artifact is internally perfect — it parses,
    // it is canonical, its content type is right, its origin is right, and its
    // signature verifies against the key it carries. It vouches for nothing.
    let evil = attacker();
    let forged = sign_delegation(&evil, &claim()).unwrap();

    // Internally perfect, proven rather than asserted: verified against the key
    // it names, it passes.
    let self_consistent =
        verify_delegation(&forged, &evil.public_key_bytes(), DOMAIN, ORIGIN).unwrap();
    assert_eq!(self_consistent.root_public_key, evil.public_key_bytes());
    assert_eq!(self_consistent.claim.subject_value, ORIGIN);

    // And refused the moment the caller names the root key they actually trust.
    assert!(matches!(
        verify_delegation(&forged, &root().public_key_bytes(), DOMAIN, ORIGIN),
        Err(TrustError::DelegationVerification)
    ));
}

#[test]
fn substituting_the_kid_in_a_genuine_delegation_breaks_the_signature_too() {
    // `kid` rides in the signature-covered protected bucket, so even a caller
    // fooled into expecting the substituted key gets a rejection. This is
    // defence in depth behind the expected-key check, not a replacement for it —
    // the test above is the one that matters, because there the signature is
    // genuine.
    let key = root();
    let evil = attacker();
    let mutant = encoding::envelope_bytes(
        &encoding::protected_bytes(encoding::CONTENT_TYPE, &evil.public_key_bytes()),
        &encoding::payload_bytes(&claim()),
        &key.sign(&delegation_preimage(&key.public_key_bytes(), &claim())),
    );
    assert!(verify_delegation(&mutant, &evil.public_key_bytes(), DOMAIN, ORIGIN).is_err());
}

#[test]
fn a_kid_that_is_not_exactly_thirty_two_bytes_is_refused() {
    // RFC 9052 §3.1 types `kid` as a `bstr`; this format pins its length, so a
    // slot that could not hold an Ed25519 key never parses. The bucket is
    // hand-assembled and signed for real, so only the length rule can refuse it.
    let key = root();
    let mut refused = 0;
    for len in [0u8, 1, 31, 33, 64] {
        let protected = protected_with_kid_len(len);
        let mutant = signed_envelope(&protected, &encoding::payload_bytes(&claim()), &key);
        assert!(
            verify_delegation(&mutant, &key.public_key_bytes(), DOMAIN, ORIGIN).is_err(),
            "a {len}-byte kid was accepted"
        );
        refused += 1;
    }
    assert_eq!(refused, 5, "every kid length must have been tried");

    // Positive control: the same hand-assembled construction at 32 bytes is a
    // well-formed bucket, so the refusals above are about the length alone.
    let genuine = protected_with_kid_len(32);
    assert_eq!(
        genuine,
        encoding::protected_bytes(encoding::CONTENT_TYPE, &[0x11; 32])
    );
}

// ---------------------------------------------------------------------------
// §1.0 — the tag is mandatory.
// ---------------------------------------------------------------------------

#[test]
fn an_untagged_cose_sign1_is_refused_though_its_signature_is_valid() {
    // RFC 9052 §4.2 permits an untagged `COSE_Sign1` "depending on the
    // context", so accepting both forms would give one statement two valid
    // encodings — the defect the canonicality rule exists to prevent. The
    // untagged form here is byte-for-byte the tagged artifact minus its first
    // byte, so the signature over the buckets is untouched and genuine.
    let key = root();
    let good = sign_delegation(&key, &claim()).unwrap();
    verify_delegation(&good, &key.public_key_bytes(), DOMAIN, ORIGIN).unwrap();

    assert_eq!(good[0], 0xd2, "tag 18");
    let untagged = &good[1..];
    assert_eq!(untagged[0], 0x84, "a bare 4-array, which is valid COSE");
    assert!(
        ciborium::de::from_reader::<ciborium::value::Value, _>(untagged).is_ok(),
        "the untagged form must be well-formed CBOR for this test to mean anything"
    );

    assert!(matches!(
        verify_delegation(untagged, &key.public_key_bytes(), DOMAIN, ORIGIN),
        Err(TrustError::DelegationVerification)
    ));
}

// ---------------------------------------------------------------------------
// §3.4 — canonical-encoding strictness, with a valid signature.
// ---------------------------------------------------------------------------

#[test]
fn a_permuted_payload_map_is_refused_though_its_signature_is_valid() {
    // Payload map keys emitted in permuted order (2, 1, 3, 4, 5 instead of
    // 1, 2, 3, 4, 5). Every value is identical and a permissive CBOR reader
    // recovers the same statement; the root key signs these exact bytes, so the
    // signature is genuine.
    //
    // **What refuses this is the positional decode pins, not the byte-compare.**
    // §6.2 originally designated the permutation as the isolating injection for
    // the canonicality rule, and an adversarial review disproved that by
    // deleting the byte-compare and watching this test still pass. The
    // designated case is now `the_envelope_malleability_sweep_is_refused`'s
    // indefinite outer array. This test is kept because the rule it *does* guard
    // is real: without it a permuted map would be a second valid artifact for
    // one statement, and to a conforming stranger — RFC 9052 §9 does not
    // constrain map key ordering — it would be a perfectly good one.
    let key = root();
    let claim = claim();

    let mut permuted = vec![0xa6, 0x03, 0x58, 0x20];
    permuted.extend_from_slice(&claim.delegated_public_key);
    permuted.extend_from_slice(&[0x01, 0x01, 0x02]);
    permuted.push(0x60 | u8::try_from(ORIGIN.len()).unwrap());
    permuted.extend_from_slice(ORIGIN.as_bytes());
    permuted.extend_from_slice(&[0x04, 0x02, 0x05, 0x1b]);
    permuted.extend_from_slice(&claim.not_before_unix_ms.to_be_bytes());
    permuted.extend_from_slice(&[0x06, 0x19, 0x01, 0x2c]);

    let canonical = encoding::payload_bytes(&claim);
    assert_eq!(
        permuted.len(),
        canonical.len(),
        "a permutation, not a different payload"
    );
    assert_ne!(permuted, canonical);

    let mutant = signed_envelope(&protected_for(&key), &permuted, &key);
    assert!(
        ciborium::de::from_reader::<ciborium::value::Value, _>(mutant.as_slice()).is_ok(),
        "the mutant must be well-formed CBOR for this test to mean anything"
    );
    assert!(matches!(
        verify_delegation(&mutant, &key.public_key_bytes(), DOMAIN, ORIGIN),
        Err(TrustError::DelegationVerification)
    ));
}

// ---------------------------------------------------------------------------
// §1.3 — the unprotected header must be empty.
// ---------------------------------------------------------------------------

#[test]
fn a_non_empty_unprotected_header_is_refused_though_the_signature_still_verifies() {
    // The injection from §6.2: the unprotected header carries one entry. This is
    // the sharpest of the seven, because the unprotected bucket is *not*
    // signature-covered: the entry can be added to a genuine artifact after the
    // fact and the root key's signature over the rest still verifies perfectly.
    // Only the empty-bucket rule stands between an attacker and unsigned data
    // riding inside a trusted artifact.
    let key = root();
    let good = sign_delegation(&key, &claim()).unwrap();
    verify_delegation(&good, &key.public_key_bytes(), DOMAIN, ORIGIN).unwrap();

    let unprotected_at = 4 + encoding::PROTECTED_LEN;
    assert_eq!(good[unprotected_at], 0xa0, "the empty map");

    let mut mutant = good[..unprotected_at].to_vec();
    mutant.extend_from_slice(&[0xa1, 0x01, 0x01]); // {1: 1}
    mutant.extend_from_slice(&good[unprotected_at + 1..]);

    assert!(
        ciborium::de::from_reader::<ciborium::value::Value, _>(mutant.as_slice()).is_ok(),
        "the mutant must be well-formed CBOR for this test to mean anything"
    );
    // The signature is untouched and still covers the protected bucket and the
    // payload, both of which are byte-identical to the genuine artifact's.
    assert_eq!(&mutant[mutant.len() - 64..], &good[good.len() - 64..]);

    assert!(matches!(
        verify_delegation(&mutant, &key.public_key_bytes(), DOMAIN, ORIGIN),
        Err(TrustError::DelegationVerification)
    ));
}

// ---------------------------------------------------------------------------
// §2.3 — unknown roles are rejected, not carried.
// ---------------------------------------------------------------------------

#[test]
fn an_unknown_role_is_refused_though_its_signature_is_valid() {
    // The injection from §6.2: `role` set to 7. The root key signs it for real,
    // so a tolerant implementation would hand a consumer a cryptographically
    // perfect delegation whose meaning nobody defines — a signed unchecked value
    // that looks checked.
    let key = root();
    let canonical = encoding::payload_bytes(&claim());
    let role_at = role_offset(&canonical);

    // `1` is in the list on purpose: it is the DOMAIN kind's wire value, so an
    // artifact carrying it in the role slot is what a transposed implementation
    // would produce. `3` is a valid role but not for a domain, so it belongs to
    // the pair test rather than here.
    let mut refused = 0;
    for role in [0u8, 1, 4, 7, 23] {
        let mut payload = canonical.clone();
        payload[role_at + 1] = role;
        assert_eq!(payload.len(), canonical.len());
        let mutant = signed_envelope(&protected_for(&key), &payload, &key);
        assert!(
            matches!(
                verify_delegation(&mutant, &key.public_key_bytes(), DOMAIN, ORIGIN),
                Err(TrustError::DelegationVerification)
            ),
            "role {role} was accepted"
        );
        refused += 1;
    }
    assert_eq!(refused, 5, "every unknown role must have been tried");

    // Positive control: the same construction with the known role is accepted,
    // so the refusals above are about the role and not about the construction.
    let ok = signed_envelope(&protected_for(&key), &canonical, &key);
    verify_delegation(&ok, &key.public_key_bytes(), DOMAIN, ORIGIN).unwrap();
}

// ---------------------------------------------------------------------------
// §3.6 — strict Ed25519.
// ---------------------------------------------------------------------------

#[test]
fn a_flipped_signature_bit_is_refused() {
    // The injection from §6.2: a byte flipped in the signature.
    let key = root();
    let good = sign_delegation(&key, &claim()).unwrap();
    let at = good.len() - 64;

    let mut refused = 0;
    for byte in [0usize, 1, 31, 32, 62, 63] {
        for bit in [0x01u8, 0x80] {
            let mut mutant = good.clone();
            mutant[at + byte] ^= bit;
            assert!(
                verify_delegation(&mutant, &key.public_key_bytes(), DOMAIN, ORIGIN).is_err(),
                "a flip of bit {bit:#04x} in signature byte {byte} was accepted"
            );
            refused += 1;
        }
    }
    assert_eq!(refused, 12, "every flip must have been tried");
}

#[test]
fn a_non_canonical_signature_scalar_is_refused() {
    // Strict verification rejects a malleable `s`; non-repudiation requires a
    // unique valid signature per message. Setting the high bits of `s` puts it
    // outside the canonical range, which plain (non-strict) verification would
    // accept.
    let key = root();
    let mut mutant = sign_delegation(&key, &claim()).unwrap();
    let last = mutant.len() - 1;
    mutant[last] |= 0xe0;
    assert!(verify_delegation(&mutant, &key.public_key_bytes(), DOMAIN, ORIGIN).is_err());
}
