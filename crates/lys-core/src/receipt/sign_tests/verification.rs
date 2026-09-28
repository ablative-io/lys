#![cfg(test)]
//! Receipts that verify at every size and index, and the tampers and
//! substitutions verification refuses.

use super::*;

/// The end-to-end property, over every leaf of every tree size a genesis-seeded
/// log can reach: issue a receipt from a real proof, then verify it.
#[test]
fn every_issued_receipt_verifies_at_every_size_and_index() {
    let key = primary();
    let expected = key.public_key_bytes();

    for size in 2..=33u64 {
        for index in 0..size {
            let path = path_of(size, index);
            let leaf = leaf_bytes(index);
            let receipt = sign_receipt(&leaf, index, size, &path, &key)
                .unwrap_or_else(|e| panic!("issuance failed at size {size} index {index}: {e}"));

            verify_receipt(&receipt, &leaf, &expected)
                .unwrap_or_else(|e| panic!("verify failed at size {size} index {index}: {e}"));

            // And through the wire, which is the only form that matters.
            let bytes = receipt.to_cose_bytes();
            let parsed = verify_receipt_bytes(&bytes, &leaf, &expected).unwrap();
            assert_eq!(parsed, receipt);
        }
    }
}

/// The signed root is the tree's actual root — the anchor vouches for a value
/// it derived, not one it was handed.
#[test]
fn the_signed_root_is_the_trees_own_root() {
    let key = primary();
    let size = 8;
    let index = 3;
    let tree = tree_of(size);
    let receipt =
        sign_receipt(&leaf_bytes(index), index, size, &path_of(size, index), &key).unwrap();
    assert_eq!(
        receipt.reconstructed_root(&leaf_bytes(index)).unwrap(),
        tree.root().to_parts().0
    );
}

#[test]
fn a_receipt_for_one_leaf_does_not_verify_for_another() {
    let key = primary();
    let size = 8;
    let receipt = sign_receipt(&leaf_bytes(3), 3, size, &path_of(size, 3), &key).unwrap();
    assert!(matches!(
        verify_receipt(&receipt, &leaf_bytes(4), &key.public_key_bytes()),
        Err(TrustError::ReceiptVerification)
    ));
    // Including a leaf that differs by a single byte.
    let mut nearly = leaf_bytes(3);
    nearly[0] ^= 0x01;
    assert!(verify_receipt(&receipt, &nearly, &key.public_key_bytes()).is_err());
}

/// Misattribution: a valid receipt from an anchor nobody trusts must not pass.
/// This is the same trap as a self-signed certificate — the signature proves
/// someone holding a key spoke, never that the statement is true.
#[test]
fn a_receipt_from_an_unexpected_anchor_is_refused_though_it_is_internally_perfect() {
    let attacker = other();
    let size = 8;
    let index = 3;
    let leaf = leaf_bytes(index);
    let receipt = sign_receipt(&leaf, index, size, &path_of(size, index), &attacker).unwrap();

    // It verifies against its own key: the artifact is not forged.
    verify_receipt(&receipt, &leaf, &attacker.public_key_bytes()).unwrap();

    // And is refused when the caller names the anchor they actually trust.
    assert!(matches!(
        verify_receipt(&receipt, &leaf, &primary().public_key_bytes()),
        Err(TrustError::ReceiptVerification)
    ));
}

#[test]
fn swapping_the_embedded_anchor_key_breaks_the_signature_too() {
    // The key rides in the signature-covered protected header, inheriting the
    // attestation v1→v2 fix. Substituting it invalidates the signature even if
    // the caller is fooled into expecting the substituted key.
    let key = primary();
    let size = 8;
    let index = 3;
    let leaf = leaf_bytes(index);
    let mut receipt = sign_receipt(&leaf, index, size, &path_of(size, index), &key).unwrap();

    receipt.anchor_public_key = other().public_key_bytes();
    assert!(verify_receipt(&receipt, &leaf, &other().public_key_bytes()).is_err());
}

#[test]
fn a_tampered_path_is_refused() {
    let key = primary();
    let size = 9;
    let index = 4;
    let leaf = leaf_bytes(index);
    let expected = key.public_key_bytes();

    for node in 0..path_of(size, index).len() {
        for bit in [0x01u8, 0x80] {
            let mut receipt =
                sign_receipt(&leaf, index, size, &path_of(size, index), &key).unwrap();
            receipt.inclusion_path[node][0] ^= bit;
            assert!(
                verify_receipt(&receipt, &leaf, &expected).is_err(),
                "a flipped bit in path node {node} was accepted"
            );
        }
    }
}

#[test]
fn reordering_the_path_is_refused() {
    let key = primary();
    let size = 16;
    let index = 5;
    let leaf = leaf_bytes(index);
    let mut receipt = sign_receipt(&leaf, index, size, &path_of(size, index), &key).unwrap();
    receipt.inclusion_path.reverse();
    assert!(verify_receipt(&receipt, &leaf, &key.public_key_bytes()).is_err());
}

#[test]
fn a_tampered_leaf_index_is_refused() {
    // Index 5 and index 6 sit at the same depth in a size-16 tree, so the path
    // length still matches and only the walk direction differs — the case a
    // length-only check would miss.
    let key = primary();
    let size = 16;
    let leaf = leaf_bytes(5);
    let mut receipt = sign_receipt(&leaf, 5, size, &path_of(size, 5), &key).unwrap();
    receipt.leaf_index = 6;
    assert!(verify_receipt(&receipt, &leaf, &key.public_key_bytes()).is_err());
}

#[test]
fn a_tampered_signature_is_refused() {
    let key = primary();
    let size = 8;
    let index = 3;
    let leaf = leaf_bytes(index);
    for byte in [0usize, 31, 32, 63] {
        let mut receipt = sign_receipt(&leaf, index, size, &path_of(size, index), &key).unwrap();
        receipt.signature[byte] ^= 0x01;
        assert!(verify_receipt(&receipt, &leaf, &key.public_key_bytes()).is_err());
    }
}

#[test]
fn a_non_canonical_signature_scalar_is_refused() {
    // Strict verification rejects a malleable `s`; non-repudiation requires a
    // unique valid signature per message. Setting the high bits of `s` puts it
    // outside the canonical range.
    let key = primary();
    let size = 8;
    let index = 3;
    let leaf = leaf_bytes(index);
    let mut receipt = sign_receipt(&leaf, index, size, &path_of(size, index), &key).unwrap();
    receipt.signature[63] |= 0xe0;
    assert!(verify_receipt(&receipt, &leaf, &key.public_key_bytes()).is_err());
}

#[test]
fn a_receipt_cannot_be_replayed_against_a_different_tree() {
    // The same leaf at the same index, but in a tree that grew. The root
    // changed, so the old signature does not cover the new shape.
    let key = primary();
    let index = 3;
    let leaf = leaf_bytes(index);
    let receipt = sign_receipt(&leaf, index, 8, &path_of(8, index), &key).unwrap();

    let mut replayed = receipt;
    replayed.tree_size = 9;
    replayed.inclusion_path = path_of(9, index);
    assert!(verify_receipt(&replayed, &leaf, &key.public_key_bytes()).is_err());
}
