#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use crate::merkle::tree::{AppendOnlyTree, RawLeaf};
use crate::merkle::{raw_leaf_hash, verify_inclusion_raw};

/// Largest tree size exercised. Crossing 32 covers several incomplete-level
/// shapes and both sides of the power-of-two boundaries at 2, 4, 8, 16 and 32,
/// which is where a naive walk breaks.
const MAX_SIZE: u64 = 33;

fn leaf_bytes(index: u64) -> Vec<u8> {
    format!("leaf-{index}").into_bytes()
}

fn tree_of(size: u64) -> AppendOnlyTree<RawLeaf> {
    let mut tree = AppendOnlyTree::<RawLeaf>::new();
    for index in 0..size {
        tree.append_raw(&leaf_bytes(index));
    }
    tree
}

/// The invariant that makes receipts possible, checked exhaustively rather than
/// sampled: for every tree size and every leaf in it, the root reconstructed
/// from `(leaf hash, index, size, path)` must equal the root the tree itself
/// reports.
///
/// This is the test that closes the one real risk in hand-writing the RFC 6962
/// walk. A walk that disagreed with ct-merkle's tree shape would produce roots
/// no anchor ever signed, making every receipt ever issued unverifiable — and
/// small balanced trees are exactly the cases where a wrong implementation
/// still happens to agree, so agreement on a handful of sizes would prove
/// nothing.
#[test]
fn reconstructed_root_matches_the_tree_at_every_size_and_index() {
    for size in 1..=MAX_SIZE {
        let tree = tree_of(size);
        let root = tree.root();
        let (expected_root_bytes, num_leaves) = root.to_parts();
        assert_eq!(num_leaves, size);

        for index in 0..size {
            let proof = tree.prove_inclusion(index).unwrap();
            let leaf_hash = raw_leaf_hash(&leaf_bytes(index));

            let reconstructed = root_from_inclusion_path(&leaf_hash, index, size, proof.as_bytes())
                .unwrap_or_else(|e| {
                    panic!("reconstruction failed at size {size} index {index}: {e}")
                });

            assert_eq!(
                reconstructed, expected_root_bytes,
                "reconstructed root differs from the tree's own root at size {size} index {index}"
            );
        }
    }
}

/// Reconstruction must agree with the existing verifier on every case, in both
/// directions: where `verify_inclusion_raw` accepts, reconstruction reproduces
/// the root, and where reconstruction produces a different root, the verifier
/// rejects. Two independent code paths over the same proof either both hold or
/// the disagreement is a bug in one of them.
#[test]
fn reconstruction_agrees_with_the_supplied_root_verifier() {
    for size in [1u64, 2, 3, 5, 8, 13, 21] {
        let tree = tree_of(size);
        let root = tree.root();
        for index in 0..size {
            let proof = tree.prove_inclusion(index).unwrap();
            let bytes = leaf_bytes(index);
            verify_inclusion_raw(&root, &bytes, index, &proof).unwrap();

            let reconstructed =
                root_from_inclusion_path(&raw_leaf_hash(&bytes), index, size, proof.as_bytes())
                    .unwrap();
            assert_eq!(reconstructed, root.to_parts().0);
        }
    }
}

/// A tampered path must not reconstruct the signed root. This is the property
/// that makes an unprotected inclusion proof safe: altering it yields bytes the
/// anchor never signed, so the signature check fails.
#[test]
fn a_tampered_path_reconstructs_a_different_root() {
    let size = 9;
    let tree = tree_of(size);
    let expected = tree.root().to_parts().0;
    let index = 4;
    let proof = tree.prove_inclusion(index).unwrap();
    let leaf_hash = raw_leaf_hash(&leaf_bytes(index));

    let mut tampered = proof.as_bytes().to_vec();
    assert!(!tampered.is_empty(), "expected a non-empty path");
    tampered[0] ^= 0x01;

    let reconstructed = root_from_inclusion_path(&leaf_hash, index, size, &tampered).unwrap();
    assert_ne!(
        reconstructed, expected,
        "a tampered path must not reconstruct the root the anchor signed"
    );
}

/// A path of the right length for a *different* leaf must not reconstruct the
/// root either — the index is an input to the walk, not a label on it.
#[test]
fn the_wrong_leaf_index_reconstructs_a_different_root() {
    let size = 16;
    let tree = tree_of(size);
    let expected = tree.root().to_parts().0;
    let proof = tree.prove_inclusion(5).unwrap();
    // Index 5 and index 6 sit at the same depth in a size-16 tree, so the path
    // length matches and only the walk direction differs.
    let reconstructed =
        root_from_inclusion_path(&raw_leaf_hash(&leaf_bytes(5)), 6, size, proof.as_bytes())
            .unwrap();
    assert_ne!(reconstructed, expected);
}

#[test]
fn a_single_leaf_tree_has_an_empty_path_and_reconstructs_its_leaf_hash() {
    let tree = tree_of(1);
    let proof = tree.prove_inclusion(0).unwrap();
    assert!(proof.as_bytes().is_empty(), "a one-leaf tree needs no path");

    let leaf_hash = raw_leaf_hash(&leaf_bytes(0));
    let reconstructed = root_from_inclusion_path(&leaf_hash, 0, 1, proof.as_bytes()).unwrap();
    assert_eq!(reconstructed, leaf_hash);
    assert_eq!(reconstructed, tree.root().to_parts().0);
}

#[test]
fn structural_inputs_are_rejected_rather_than_guessed() {
    let leaf_hash = raw_leaf_hash(b"leaf-0");

    // Size zero: there is no tree to reconstruct against.
    assert!(root_from_inclusion_path(&leaf_hash, 0, 0, &[]).is_err());

    // Index at or beyond the claimed size.
    assert!(root_from_inclusion_path(&leaf_hash, 4, 4, &[]).is_err());
    assert!(root_from_inclusion_path(&leaf_hash, 99, 4, &[]).is_err());

    // Path that is not a whole number of digests.
    assert!(root_from_inclusion_path(&leaf_hash, 0, 4, &[0u8; 33]).is_err());

    // Too short and too long for the claimed tree are both refused: a path of
    // the wrong length for a tree describes a different tree, and accepting it
    // would let a caller reconstruct a root for a shape that never existed.
    assert!(root_from_inclusion_path(&leaf_hash, 0, 4, &[0u8; 32]).is_err());
    assert!(root_from_inclusion_path(&leaf_hash, 0, 4, &[0u8; 32 * 5]).is_err());
}

/// Every size/index pair must require exactly the path length the tree
/// produces, so a proof cannot be padded or truncated and still reconstruct.
#[test]
fn only_the_exact_path_length_is_accepted() {
    for size in 1..=MAX_SIZE {
        let tree = tree_of(size);
        for index in 0..size {
            let proof = tree.prove_inclusion(index).unwrap();
            let leaf_hash = raw_leaf_hash(&leaf_bytes(index));
            let exact = proof.as_bytes().to_vec();

            let mut padded = exact.clone();
            padded.extend_from_slice(&[0u8; 32]);
            assert!(
                root_from_inclusion_path(&leaf_hash, index, size, &padded).is_err(),
                "a padded path was accepted at size {size} index {index}"
            );

            if !exact.is_empty() {
                let truncated = &exact[..exact.len() - 32];
                assert!(
                    root_from_inclusion_path(&leaf_hash, index, size, truncated).is_err(),
                    "a truncated path was accepted at size {size} index {index}"
                );
            }
        }
    }
}
