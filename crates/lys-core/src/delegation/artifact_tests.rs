//! Round-trip and canonical-encoding-strictness tests for
//! [`AnchorDelegation`].
//!
//! The mutants here all have or could have **cryptographically valid
//! signatures** — several are re-encodings of a genuine delegation rather than
//! forgeries, and a vanilla COSE verifier accepts them. lys rejects them because
//! a delegation with two valid encodings is a delegation whose bytes are not its
//! identity, and every downstream comparison (log dedup, a cache key, "is this
//! the same statement I saw yesterday") then depends on which encoding you
//! happened to receive.
//!
//! Signature-bearing versions of the security rules live in `sign_tests`, where
//! they are signed for real; this file is about the parser.
//!
//! # The malleability sweep, and what it does and does not establish
//!
//! [`the_envelope_malleability_sweep_is_refused`] walks every non-canonical
//! **envelope** spelling the suite knows how to build. That is the byte-compare's
//! own territory, because no signature covers the envelope under any verifier's
//! strategy — and at this parse-only entry point the byte-compare is the *only*
//! canonicality guard, since no signature is checked here at all.
//!
//! **The sweep is a lower bound on what the check guards, not an inventory of
//! it.** CBOR admits more non-canonical spellings than any suite enumerates,
//! which is exactly why the rule is "byte-identical to the canonical
//! re-encoding" rather than a list of rejected forms. An earlier note claimed
//! "exactly four artifacts flip to accepted" when the check is removed; that
//! was the set the suite happened to cover, stated as though it were the set
//! that exists.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use crate::delegation::encoding::{CONTENT_TYPE, MAX_ARTIFACT_LEN, MAX_SEQUENCE, PROTECTED_LEN};

const ROOT_KEY: [u8; KEY_LEN] = [0xa1; KEY_LEN];
const SIGNATURE: [u8; 64] = [0x5e; 64];

/// Offset of the protected bucket's first byte inside the artifact: the four
/// bytes `d2 84 58 56` precede it.
const PROTECTED_AT: usize = 4;
/// Offset of the unprotected bucket byte.
const UNPROTECTED_AT: usize = PROTECTED_AT + PROTECTED_LEN;

/// A real Ed25519 public key for the delegated slot. It must be real: the
/// decoder now validates it as a point strict verification could accept.
fn delegated_key() -> [u8; KEY_LEN] {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("delegated.key");
    std::fs::write(&path, [0x42u8; 32]).unwrap();
    crate::Ed25519Identity::load(&path)
        .unwrap()
        .public_key_bytes()
}

fn claim_for(origin: &str, not_before_unix_ms: u64, sequence: u64) -> DelegationClaim {
    DelegationClaim {
        origin: origin.to_string(),
        delegated_public_key: delegated_key(),
        role: DelegationRole::Operational,
        not_before_unix_ms,
        sequence,
    }
}

fn sample() -> AnchorDelegation {
    AnchorDelegation {
        root_public_key: ROOT_KEY,
        claim: claim_for("example.test", 1_700_000_000_000, 300),
        signature: SIGNATURE,
    }
}

/// Rebuild the artifact around a deliberately altered payload, so the payload
/// bstr head stays correct and the *canonicality* rule is what rejects the
/// mutant rather than a malformed document.
fn artifact_with_payload(payload: &[u8]) -> Vec<u8> {
    encoding::envelope_bytes(
        &encoding::protected_bytes(CONTENT_TYPE, &ROOT_KEY),
        payload,
        &SIGNATURE,
    )
}

#[test]
fn round_trip_is_identity_on_the_value() {
    let delegation = sample();
    let parsed = AnchorDelegation::from_cose_bytes(&delegation.to_cose_bytes()).unwrap();
    assert_eq!(parsed, delegation);
}

#[test]
fn round_trip_is_identity_on_the_bytes() {
    let bytes = sample().to_cose_bytes();
    let reencoded = AnchorDelegation::from_cose_bytes(&bytes)
        .unwrap()
        .to_cose_bytes();
    assert_eq!(reencoded, bytes);
}

#[test]
fn round_trip_holds_across_the_interesting_shapes() {
    // The origin lengths straddle every CBOR text head width the artifact cap
    // permits, and both integers straddle every unsigned head width. The empty
    // origin is absent deliberately — it is refused now, and
    // `encoding_tests::decode_refuses_an_empty_origin_*` is where that lives.
    let cases: Vec<(String, u64, u64)> = vec![
        ("a".to_string(), 0, 0),
        ("b".repeat(23), 23, 23),
        ("example.test".to_string(), 24, 24),
        ("d".repeat(255), 65_535, 255),
        ("e".repeat(256), 1_700_000_000_000, 300),
        ("f".repeat(3000), u64::MAX, MAX_SEQUENCE),
    ];
    let mut checked = 0;
    for (origin, not_before_unix_ms, sequence) in cases {
        let delegation = AnchorDelegation {
            root_public_key: ROOT_KEY,
            claim: claim_for(&origin, not_before_unix_ms, sequence),
            signature: SIGNATURE,
        };
        let bytes = delegation.to_cose_bytes();
        assert!(bytes.len() <= MAX_ARTIFACT_LEN, "fixture within the cap");
        assert_eq!(
            AnchorDelegation::from_cose_bytes(&bytes).unwrap(),
            delegation
        );
        checked += 1;
    }
    assert_eq!(checked, 6, "every shape must have been exercised");
}

// ---------------------------------------------------------------------------
// Envelope canonicality — the byte-compare's own territory.
// ---------------------------------------------------------------------------

#[test]
fn the_envelope_malleability_sweep_is_refused() {
    // Every one of these is well-formed CBOR carrying the identical statement,
    // and every one is a second spelling of an artifact that must have exactly
    // one. Each is asserted to parse as CBOR first, so a mutant that merely
    // corrupted the document would be caught here rather than passing as a
    // canonicality rejection.
    let good = sample().to_cose_bytes();
    let protected = encoding::protected_bytes(CONTENT_TYPE, &ROOT_KEY);
    let payload = encoding::payload_bytes(&sample().claim);

    // The head widths the canonical form uses, so the mutants below are known to
    // be *widenings* rather than corrections.
    assert_eq!(&good[..4], &[0xd2, 0x84, 0x58, 0x56]);

    let mut mutants: Vec<(&str, Vec<u8>)> = Vec::new();

    // 1. Non-minimal tag head: `d8 12` instead of `d2`.
    let mut m = vec![0xd8, 0x12];
    m.extend_from_slice(&good[1..]);
    mutants.push(("non-minimal tag head", m));

    // 2. Non-minimal outer-array head: `98 04` instead of `84`.
    let mut m = vec![0xd2, 0x98, 0x04];
    m.extend_from_slice(&good[2..]);
    mutants.push(("non-minimal outer-array head", m));

    // 3. Indefinite-length outer array: `9f ... ff`. This is the case spec §6.2
    //    designates as the isolating injection for the canonicality rule.
    let mut m = vec![0xd2, 0x9f];
    m.extend_from_slice(&good[2..]);
    m.push(0xff);
    mutants.push(("indefinite outer array", m));

    // 4. Indefinite-length protected bstr: `5f 58 56 <86 bytes> ff`.
    let mut m = vec![0xd2, 0x84, 0x5f, 0x58, 0x56];
    m.extend_from_slice(&protected);
    m.push(0xff);
    m.extend_from_slice(&good[PROTECTED_AT + PROTECTED_LEN..]);
    mutants.push(("indefinite protected bstr", m));

    // 5. Non-minimal protected bstr length head: `59 00 56` instead of `58 56`.
    let mut m = vec![0xd2, 0x84, 0x59, 0x00, 0x56];
    m.extend_from_slice(&protected);
    m.extend_from_slice(&good[PROTECTED_AT + PROTECTED_LEN..]);
    mutants.push(("non-minimal protected bstr head", m));

    // 6. Indefinite-length unprotected map: `bf ff` instead of `a0`.
    let mut m = good[..UNPROTECTED_AT].to_vec();
    m.extend_from_slice(&[0xbf, 0xff]);
    m.extend_from_slice(&good[UNPROTECTED_AT + 1..]);
    mutants.push(("indefinite unprotected map", m));

    // 7. Indefinite-length payload bstr.
    let mut m = good[..=UNPROTECTED_AT].to_vec();
    m.push(0x5f);
    m.push(0x58);
    m.push(u8::try_from(payload.len()).unwrap());
    m.extend_from_slice(&payload);
    m.push(0xff);
    m.extend_from_slice(&[0x58, 0x40]);
    m.extend_from_slice(&SIGNATURE);
    mutants.push(("indefinite payload bstr", m));

    // 8. Non-minimal payload bstr length head: `59 00 xx`.
    let mut m = good[..=UNPROTECTED_AT].to_vec();
    m.extend_from_slice(&[0x59, 0x00, u8::try_from(payload.len()).unwrap()]);
    m.extend_from_slice(&payload);
    m.extend_from_slice(&[0x58, 0x40]);
    m.extend_from_slice(&SIGNATURE);
    mutants.push(("non-minimal payload bstr head", m));

    // 9. Non-minimal signature bstr length head: `59 00 40` instead of `58 40`.
    let mut m = good[..good.len() - 66].to_vec();
    m.extend_from_slice(&[0x59, 0x00, 0x40]);
    m.extend_from_slice(&SIGNATURE);
    mutants.push(("non-minimal signature bstr head", m));

    // 10. Indefinite-length signature bstr.
    let mut m = good[..good.len() - 66].to_vec();
    m.extend_from_slice(&[0x5f, 0x58, 0x40]);
    m.extend_from_slice(&SIGNATURE);
    m.push(0xff);
    mutants.push(("indefinite signature bstr", m));

    // 11. Trailing garbage. ciborium stops at the end of the first value, so
    //     nothing but the byte-compare sees this.
    let mut m = good.clone();
    m.push(0x00);
    mutants.push(("trailing garbage", m));

    // 12. Sixteen bytes of trailing garbage, so the case is not special to one.
    let mut m = good.clone();
    m.extend_from_slice(&[0xff; 16]);
    mutants.push(("trailing garbage, 16 bytes", m));

    let mut refused = 0;
    for (name, mutant) in &mutants {
        assert_ne!(mutant, &good, "{name} did not change the artifact");
        assert!(
            ciborium::de::from_reader::<ciborium::value::Value, _>(mutant.as_slice()).is_ok(),
            "{name} is not well-formed CBOR, so its rejection would prove nothing"
        );
        assert!(
            matches!(
                AnchorDelegation::from_cose_bytes(mutant),
                Err(TrustError::DelegationVerification)
            ),
            "{name} was accepted"
        );
        refused += 1;
    }
    assert_eq!(
        refused, 12,
        "every malleability mutant must have been tried"
    );

    // The positive control. Without it a parser that refused everything would
    // satisfy all twelve assertions above.
    AnchorDelegation::from_cose_bytes(&good).unwrap();
}

#[test]
fn an_oversized_integer_head_in_the_payload_is_refused() {
    // Encode the role `1` as `19 00 01` (two-byte head) rather than `01`. The
    // value is identical; the bytes are not shortest-form, so the same statement
    // would have two encodings.
    let payload = encoding::payload_bytes(&sample().claim);
    let matches = payload.windows(2).filter(|w| *w == [0x03, 0x01]).count();
    assert_eq!(matches, 1, "the role entry must be locatable unambiguously");
    let role_at = payload
        .windows(2)
        .position(|w| w == [0x03, 0x01])
        .expect("role entry present");

    let mut fat = payload[..role_at].to_vec();
    fat.extend_from_slice(&[0x03, 0x19, 0x00, 0x01]);
    fat.extend_from_slice(&payload[role_at + 2..]);

    let mutant = artifact_with_payload(&fat);
    assert!(
        ciborium::de::from_reader::<ciborium::value::Value, _>(mutant.as_slice()).is_ok(),
        "the mutant must be well-formed CBOR for this test to mean anything"
    );
    assert!(AnchorDelegation::from_cose_bytes(&mutant).is_err());
}

#[test]
fn a_reordered_protected_map_is_refused() {
    // Swap labels 3 and 4 in the protected bucket. Every value is unchanged and
    // a permissive verifier reads the same headers; the bytes differ, so the
    // signature over them would differ too.
    //
    // Note what refuses this: the positional decode pins, NOT the byte-compare.
    // `encoding_tests::decode_refuses_a_permuted_payload_map_*` is the isolating
    // case for that rule.
    let good = sample().to_cose_bytes();
    let protected = &good[PROTECTED_AT..PROTECTED_AT + PROTECTED_LEN];

    let mut reordered = vec![0xa3, 0x01, 0x27];
    reordered.extend_from_slice(&[0x04, 0x58, 0x20]);
    reordered.extend_from_slice(&ROOT_KEY);
    reordered.extend_from_slice(&protected[3..3 + 3 + CONTENT_TYPE.len()]);
    assert_eq!(reordered.len(), PROTECTED_LEN);
    assert_ne!(reordered.as_slice(), protected);

    let mut mutant = good[..PROTECTED_AT].to_vec();
    mutant.extend_from_slice(&reordered);
    mutant.extend_from_slice(&good[PROTECTED_AT + PROTECTED_LEN..]);
    assert_eq!(mutant.len(), good.len());
    assert!(AnchorDelegation::from_cose_bytes(&mutant).is_err());
}

#[test]
fn a_detached_nil_payload_is_not_a_delegation() {
    // The shape a receipt uses. A delegation's assertion cannot be recomputed by
    // anyone, so an artifact that does not carry its payload asserts nothing.
    let protected = encoding::protected_bytes(CONTENT_TYPE, &ROOT_KEY);
    let mut mutant = vec![0xd2, 0x84, 0x58, 0x56];
    mutant.extend_from_slice(&protected);
    mutant.push(0xa0);
    mutant.push(0xf6);
    mutant.extend_from_slice(&[0x58, 0x40]);
    mutant.extend_from_slice(&SIGNATURE);
    assert!(
        ciborium::de::from_reader::<ciborium::value::Value, _>(mutant.as_slice()).is_ok(),
        "the mutant must be well-formed CBOR for this test to mean anything"
    );
    assert!(AnchorDelegation::from_cose_bytes(&mutant).is_err());
}

#[test]
fn stripping_the_tag_is_refused() {
    let good = sample().to_cose_bytes();
    assert_eq!(good[0], 0xd2);
    assert!(AnchorDelegation::from_cose_bytes(&good[1..]).is_err());
}

#[test]
fn a_wrong_tag_number_is_refused() {
    // Tag 98 is `COSE_Sign` (multi-signer): the same outer array shape carrying
    // a different meaning.
    let mut mutant = sample().to_cose_bytes();
    mutant[0] = 0xd8;
    mutant.insert(1, 98);
    assert!(AnchorDelegation::from_cose_bytes(&mutant).is_err());
}

#[test]
fn a_receipt_content_type_in_the_protected_bucket_is_refused_by_the_parser() {
    // The parser pins its own constant, so an otherwise perfectly shaped
    // artifact bearing another lys artifact's discriminator never parses.
    // `sign_tests` proves the same thing for an artifact with a *valid*
    // signature, which is the case that matters.
    let confused = encoding::envelope_bytes(
        &encoding::protected_bytes("application/vnd.lys.receipt.v1+cbor", &ROOT_KEY),
        &encoding::payload_bytes(&sample().claim),
        &SIGNATURE,
    );
    assert!(AnchorDelegation::from_cose_bytes(&confused).is_err());
}

#[test]
fn an_oversize_artifact_is_refused() {
    let delegation = AnchorDelegation {
        root_public_key: ROOT_KEY,
        claim: claim_for(&"o".repeat(MAX_ARTIFACT_LEN), 0, 0),
        signature: SIGNATURE,
    };
    let bytes = delegation.to_cose_bytes();
    assert!(bytes.len() > MAX_ARTIFACT_LEN);
    assert!(AnchorDelegation::from_cose_bytes(&bytes).is_err());
}

#[test]
fn to_cose_bytes_stays_infallible_which_is_why_the_round_trip_claim_is_qualified() {
    // The documented invariant says round-trip identity holds for values whose
    // fields satisfy the format's constraints, and the qualification was added
    // after it was violated. `to_cose_bytes` is infallible by design, so a
    // hand-built value outside the constraints still encodes and then fails to
    // decode. This test pins that gap as *known*, so a future reader does not
    // rediscover it as a bug — and so that anyone who closes it has to delete a
    // test that says why it was open.
    //
    // Every issuing entry point in `sign` refuses these up front; the only way
    // here is to build the struct literal yourself.
    let over_cap = AnchorDelegation {
        root_public_key: ROOT_KEY,
        claim: claim_for(&"o".repeat(MAX_ARTIFACT_LEN), 0, 0),
        signature: SIGNATURE,
    };
    assert!(AnchorDelegation::from_cose_bytes(&over_cap.to_cose_bytes()).is_err());

    let empty_origin = AnchorDelegation {
        root_public_key: ROOT_KEY,
        claim: claim_for("", 0, 0),
        signature: SIGNATURE,
    };
    assert!(AnchorDelegation::from_cose_bytes(&empty_origin.to_cose_bytes()).is_err());

    // Whereas a value that does satisfy them round-trips, which is the invariant
    // as it is actually stated.
    let ok = sample();
    assert_eq!(
        AnchorDelegation::from_cose_bytes(&ok.to_cose_bytes()).unwrap(),
        ok
    );
}

#[test]
fn every_parse_failure_is_the_same_error() {
    // Non-oracle discipline, with a positive control: the unmodified artifact
    // must parse, or a parser that refused everything would satisfy every
    // assertion below without doing any work.
    let good = sample().to_cose_bytes();
    AnchorDelegation::from_cose_bytes(&good).unwrap();

    let mut trailing = good.clone();
    trailing.push(0x00);

    let mut truncated = good.clone();
    truncated.truncate(good.len() / 2);

    let mut untagged = good.clone();
    untagged.remove(0);

    // A well-formed one-entry unprotected bucket `{1: 1}`, spliced in place of
    // the empty map. The document stays valid CBOR; only the empty-bucket rule
    // rejects it.
    let mut non_empty_unprotected = good[..UNPROTECTED_AT].to_vec();
    non_empty_unprotected.extend_from_slice(&[0xa1, 0x01, 0x01]);
    non_empty_unprotected.extend_from_slice(&good[UNPROTECTED_AT + 1..]);
    assert!(
        ciborium::de::from_reader::<ciborium::value::Value, _>(non_empty_unprotected.as_slice())
            .is_ok(),
        "the non-empty-bucket mutant must be well-formed CBOR"
    );

    let cases = [
        ("trailing garbage", trailing),
        ("truncated", truncated),
        ("untagged", untagged),
        ("non-empty unprotected", non_empty_unprotected),
        ("empty input", vec![]),
        ("noise", vec![0xff; 10]),
    ];

    let mut refused = 0;
    for (name, mutant) in cases {
        let err = AnchorDelegation::from_cose_bytes(&mutant).unwrap_err();
        assert!(
            matches!(err, TrustError::DelegationVerification),
            "{name} produced a distinguishable error: {err:?}"
        );
        assert_eq!(
            format!("{err}"),
            format!("{}", TrustError::DelegationVerification),
            "{name} produced a distinguishable message"
        );
        refused += 1;
    }
    assert_eq!(refused, 6, "every mutant must have been rejected");
}
