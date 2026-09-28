#![cfg(test)]
//! Issuance: the size walk equivalence, size-one trees, inconsistent proofs,
//! determinism, and checkpoint leaves.

use super::*;

/// The documented limitation, proven rather than left to be discovered.
///
/// `tree_size` rides in the unprotected header and is authenticated only as far
/// as it changes the reconstruction — and for `leaf_index = 0` a size-3 and a
/// size-4 tree produce the same sequence of combinations. So one valid receipt
/// can be re-presented with the other size.
#[test]
fn tree_size_is_malleable_within_its_walk_equivalence_class() {
    let key = primary();
    let leaf = leaf_bytes(0);
    let path = path_of(4, 0);
    let receipt = sign_receipt(&leaf, 0, 4, &path, &key).unwrap();
    verify_receipt(&receipt, &leaf, &key.public_key_bytes()).unwrap();

    let mut relabelled = receipt.clone();
    relabelled.tree_size = 3;
    assert!(
        verify_receipt(&relabelled, &leaf, &key.public_key_bytes()).is_ok(),
        "sizes 3 and 4 at index 0 share a walk, so this receipt is re-labellable — \
         if this now fails the limitation has been fixed and the docs must change"
    );

    // What is *not* malleable: the leaf, the index, and the root. The relabelled
    // receipt still proves the same leaf at the same index under the same root,
    // so nothing false is asserted about inclusion.
    assert_eq!(
        relabelled.reconstructed_root(&leaf).unwrap(),
        receipt.reconstructed_root(&leaf).unwrap()
    );

    // And a size outside the class is refused, so this is a narrow equivalence
    // and not an absence of checking.
    for size in [2u64, 5, 8, 100] {
        let mut out_of_class = receipt.clone();
        out_of_class.tree_size = size;
        assert!(
            verify_receipt(&out_of_class, &leaf, &key.public_key_bytes()).is_err(),
            "size {size} should not be in the walk class of size 4 at index 0"
        );
    }
}

#[test]
fn a_size_one_tree_is_refused_at_issuance_with_an_actionable_reason() {
    // RFC 9942 types the inclusion path as one-or-more nodes, and a one-leaf
    // tree's path is empty, so a conforming receipt cannot exist. The remedy is
    // a genesis leaf.
    let key = primary();
    let err = sign_receipt(&leaf_bytes(0), 0, 1, &[], &key).unwrap_err();
    let TrustError::MerkleTree { reason } = &err else {
        panic!("expected a Merkle error, got {err:?}");
    };
    assert!(
        reason.contains("genesis"),
        "the error must name the remedy, got: {reason}"
    );
}

#[test]
fn an_inconsistent_proof_is_refused_at_issuance() {
    // The anchor cannot sign a root it cannot derive. Each case is a proof whose
    // shape contradicts its claimed tree.
    let key = primary();
    let leaf = leaf_bytes(0);
    let node = [0x11u8; 32];

    assert!(sign_receipt(&leaf, 0, 0, &[], &key).is_err(), "empty tree");
    assert!(
        sign_receipt(&leaf, 4, 4, &[node; 2], &key).is_err(),
        "index == size"
    );
    assert!(
        sign_receipt(&leaf, 9, 4, &[node; 2], &key).is_err(),
        "index > size"
    );
    assert!(
        sign_receipt(&leaf, 0, 4, &[node], &key).is_err(),
        "path too short"
    );
    assert!(
        sign_receipt(&leaf, 0, 4, &[node; 3], &key).is_err(),
        "path too long"
    );
    assert!(
        sign_receipt(&leaf, 0, u64::MAX, &vec![node; 65], &key).is_err(),
        "path beyond the 64-node structural maximum"
    );
}

#[test]
fn issuance_is_deterministic() {
    // Ed25519 signing is deterministic and the encoder is fixed-shape, so the
    // same inputs must produce byte-identical receipts. A receipt that varied
    // run to run could not be deduplicated or compared by bytes.
    let key = primary();
    let size = 8;
    let index = 3;
    let leaf = leaf_bytes(index);
    let first = sign_receipt(&leaf, index, size, &path_of(size, index), &key).unwrap();
    let second = sign_receipt(&leaf, index, size, &path_of(size, index), &key).unwrap();
    assert_eq!(first, second);
    assert_eq!(first.to_cose_bytes(), second.to_cose_bytes());
}

#[test]
fn a_checkpoint_leaf_carries_its_own_origin_so_no_extra_binding_is_needed() {
    // The anchor notarizes checkpoint bytes, which are self-describing. This is
    // why verifying the receipt simultaneously establishes *which* child root
    // was notarized, with no separate receipt-to-child binding.
    let key = primary();
    let child = crate::checkpoint::CheckpointBody::new("child.example", 7, [0x42; 32]).unwrap();
    let leaf = child.encode().into_bytes();

    let mut tree = AppendOnlyTree::<RawLeaf>::new();
    tree.append_raw(b"genesis");
    tree.append_raw(&leaf);
    let proof = tree.prove_inclusion(1).unwrap();
    let path: Vec<[u8; 32]> = proof
        .as_bytes()
        .chunks_exact(32)
        .map(|c| <[u8; 32]>::try_from(c).unwrap())
        .collect();

    let receipt = sign_receipt(&leaf, 1, 2, &path, &key).unwrap();
    verify_receipt(&receipt, &leaf, &key.public_key_bytes()).unwrap();

    // The verified leaf parses back to the child's own claim.
    let leaf_text = String::from_utf8(leaf.clone()).unwrap();
    let parsed = crate::checkpoint::CheckpointBody::parse(&leaf_text).unwrap();
    assert_eq!(parsed.origin(), "child.example");
    assert_eq!(parsed.tree_size(), 7);
}
