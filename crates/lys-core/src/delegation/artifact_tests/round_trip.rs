#![cfg(test)]
//! Round trips on the value and the bytes, and the infallible encoder the
//! round-trip claim rests on.

use super::*;

#[test]
fn round_trip_is_identity_on_the_value() {
    let mut checked = 0;
    for delegation in [sample(), seat_sample()] {
        let parsed = Delegation::from_cose_bytes(&delegation.to_cose_bytes()).unwrap();
        assert_eq!(parsed, delegation);
        checked += 1;
    }
    assert_eq!(checked, 2, "both valid pairs must have round-tripped");
}

#[test]
fn round_trip_is_identity_on_the_bytes() {
    let bytes = sample().to_cose_bytes();
    let reencoded = Delegation::from_cose_bytes(&bytes).unwrap().to_cose_bytes();
    assert_eq!(reencoded, bytes);
}

#[test]
fn round_trip_holds_across_the_interesting_shapes() {
    // The subject-value lengths straddle every CBOR text head width up to the
    // four-byte one, and both integers straddle every unsigned head width. The empty
    // value is absent deliberately — it is refused now, and
    // `encoding_tests::decode_refuses_an_empty_subject_value_*` is where that
    // lives.
    let cases: Vec<(String, u64, u64)> = vec![
        ("a".to_string(), 0, 0),
        ("b".repeat(23), 23, 23),
        ("example.test".to_string(), 24, 24),
        ("d".repeat(255), 65_535, 255),
        ("e".repeat(256), 1_700_000_000_000, 300),
        ("f".repeat(3000), u64::MAX, MAX_SEQUENCE),
        ("g".repeat(10_000), 1, 1),
        ("h".repeat(70_000), 2, 2),
    ];
    let mut checked = 0;
    for (origin, not_before_unix_ms, sequence) in cases {
        let delegation = Delegation {
            root_public_key: ROOT_KEY,
            claim: claim_for(&origin, not_before_unix_ms, sequence),
            signature: SIGNATURE,
        };
        let bytes = delegation.to_cose_bytes();
        assert_eq!(Delegation::from_cose_bytes(&bytes).unwrap(), delegation);
        checked += 1;
    }
    assert_eq!(checked, 8, "every shape must have been exercised");
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
    let empty_subject = Delegation {
        root_public_key: ROOT_KEY,
        claim: claim_for("", 0, 0),
        signature: SIGNATURE,
    };
    assert!(Delegation::from_cose_bytes(&empty_subject.to_cose_bytes()).is_err());

    // Whereas a value that does satisfy them round-trips, which is the invariant
    // as it is actually stated.
    let ok = sample();
    assert_eq!(
        Delegation::from_cose_bytes(&ok.to_cose_bytes()).unwrap(),
        ok
    );
}
