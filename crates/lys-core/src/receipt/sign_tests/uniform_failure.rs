#![cfg(test)]
//! Every verification failure is one error, and malformed bytes are
//! indistinguishable from bad signatures.

use super::*;

/// Non-oracle discipline: every rejection must be the same value, so a
/// network-exposed verifier cannot be used to learn which check failed.
#[test]
fn every_verification_failure_is_the_same_error() {
    let key = primary();
    let size = 8;
    let index = 3;
    let leaf = leaf_bytes(index);
    let expected = key.public_key_bytes();
    let good = sign_receipt(&leaf, index, size, &path_of(size, index), &key).unwrap();

    let mut wrong_anchor = good.clone();
    wrong_anchor.anchor_public_key = other().public_key_bytes();

    let mut bad_path = good.clone();
    bad_path.inclusion_path[0][0] ^= 0x01;

    let mut short_path = good.clone();
    short_path.inclusion_path.pop();

    let mut bad_index = good.clone();
    bad_index.leaf_index = 99;

    let mut bad_sig = good.clone();
    bad_sig.signature[0] ^= 0x01;

    let cases = [
        ("wrong anchor", wrong_anchor, leaf.clone()),
        ("tampered path", bad_path, leaf.clone()),
        ("inconsistent path length", short_path, leaf.clone()),
        ("index outside the tree", bad_index, leaf.clone()),
        ("forged signature", bad_sig, leaf),
        ("wrong leaf", good, leaf_bytes(4)),
    ];

    for (name, receipt, candidate_leaf) in cases {
        let err = verify_receipt(&receipt, &candidate_leaf, &expected).unwrap_err();
        assert!(
            matches!(err, TrustError::ReceiptVerification),
            "{name} produced a distinguishable error: {err:?}"
        );
        assert_eq!(
            format!("{err}"),
            format!("{}", TrustError::ReceiptVerification),
            "{name} produced a distinguishable message"
        );
    }
}

#[test]
fn malformed_bytes_and_bad_signatures_are_indistinguishable_through_the_bytes_api() {
    let key = primary();
    let size = 8;
    let index = 3;
    let leaf = leaf_bytes(index);
    let expected = key.public_key_bytes();
    let good = sign_receipt(&leaf, index, size, &path_of(size, index), &key)
        .unwrap()
        .to_cose_bytes();

    let mut truncated = good.clone();
    truncated.truncate(good.len() / 2);

    let mut trailing = good.clone();
    trailing.push(0x00);

    let mut forged = good;
    let last = forged.len() - 1;
    forged[last] ^= 0x01;

    for mutant in [truncated, trailing, forged, vec![], vec![0xff; 10]] {
        let err = verify_receipt_bytes(&mutant, &leaf, &expected).unwrap_err();
        assert!(matches!(err, TrustError::ReceiptVerification));
    }
}
