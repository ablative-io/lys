#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Gates on [`proof_nodes`].
//!
//! # Where the second party comes from
//!
//! Splitting bytes into 32-byte chunks and re-joining them is the archetypal
//! self-agreeing test: any symmetric mistake — an off-by-one width, a reversed
//! order — survives a round trip through this crate's own pair of functions.
//! So the chunking is never checked against itself here.
//!
//! - **`lys-core`'s `root_from_inclusion_path`**, written before this crate
//!   existed, is the judge of whether the nodes are the right values in the
//!   right order. It walks the path against a leaf hash and an index and
//!   arrives at a root, and that root is compared with the one the tree
//!   reports. A reordered, widened or truncated path produces a different
//!   root and fails. The axis is *implementation*, not language: it is Rust in
//!   this workspace.
//! - **The bytes themselves.** The order check compares the concatenated
//!   output with the exact input buffer, whose contents this test wrote.
//!
//! # Refusals are counted, and open with a positive control
//!
//! A function that refused every length would satisfy every "it was refused"
//! assertion at once, so the accepting lengths are asserted first, and the
//! number of refusals that actually fired is asserted against the number of
//! cases offered.

use lys_core::merkle::{AppendOnlyTree, RawLeaf, raw_leaf_hash, root_from_inclusion_path};

use super::*;

/// The lengths a real proof can have: whole digests, including none at all.
const WHOLE_DIGEST_LENGTHS: [usize; 5] = [0, 32, 64, 96, 320];

/// Lengths no RFC 6962 path can have. Chosen either side of each boundary so a
/// check that only caught "shorter than one digest" would still fail.
const RAGGED_LENGTHS: [usize; 6] = [1, 31, 33, 63, 65, 319];

#[test]
fn a_length_that_is_not_a_whole_number_of_digests_is_refused() {
    // Positive control first: the function accepts, so the refusals below are
    // about the length and not about a function that rejects everything.
    let mut accepted = 0;
    for len in WHOLE_DIGEST_LENGTHS {
        let nodes = proof_nodes(&vec![0x5a_u8; len]).expect("a whole number of digests is a path");
        assert_eq!(nodes.len(), len / 32);
        accepted += 1;
    }
    assert_eq!(accepted, WHOLE_DIGEST_LENGTHS.len());

    let mut refused = 0;
    for len in RAGGED_LENGTHS {
        let err = proof_nodes(&vec![0x5a_u8; len]).expect_err("a ragged length is not a path");
        assert!(
            matches!(err, AnchorError::MalformedInclusionPath { byte_len } if byte_len == len),
            "a ragged length must be refused by name and report itself, got: {err}"
        );
        refused += 1;
    }
    // Count what fired: a loop that ran zero times satisfies every assertion
    // inside it.
    assert_eq!(refused, RAGGED_LENGTHS.len());
}

#[test]
fn the_nodes_are_the_input_bytes_in_order_and_nothing_else() {
    // Every byte of node i is i, so a swap, a reversal or a one-byte slip
    // between nodes is visible without comparing to a computed value.
    let mut input = Vec::new();
    for node in 0..5_u8 {
        input.extend_from_slice(&[node; 32]);
    }

    let nodes = proof_nodes(&input).unwrap();

    assert_eq!(nodes.len(), 5);
    for (position, node) in nodes.iter().enumerate() {
        let expected = u8::try_from(position).unwrap();
        assert_eq!(*node, [expected; 32], "node {position} is not in its place");
    }
    // And nothing was added or lost: the concatenation is the input.
    assert_eq!(nodes.concat(), input);
}

#[test]
fn a_chunked_path_walks_to_the_root_the_tree_reports() {
    // The judge is lys-core's reconstruction, which knows nothing about this
    // crate. If the chunking widened, reordered or truncated the path, the
    // walk lands somewhere else and the roots differ.
    let mut tree = AppendOnlyTree::<RawLeaf>::new();
    let leaves: Vec<Vec<u8>> = (0..9_u8).map(|n| vec![n; 7]).collect();
    for leaf in &leaves {
        tree.append_raw(leaf);
    }
    let (root, tree_size) = tree.root().to_parts();
    assert_eq!(tree_size, 9);

    let mut checked = 0;
    for (index, leaf) in leaves.iter().enumerate() {
        let leaf_index = u64::try_from(index).unwrap();
        let proof = tree.prove_inclusion(leaf_index).unwrap();
        let nodes = proof_nodes(proof.as_bytes()).unwrap();

        // Non-trivial path: a tree of nine leaves gives every leaf a path of
        // at least one node, so this is not nine reconstructions of nothing.
        assert!(!nodes.is_empty());

        let walked =
            root_from_inclusion_path(&raw_leaf_hash(leaf), leaf_index, tree_size, &nodes.concat())
                .expect("a chunked path must reconstruct");
        assert_eq!(walked, root, "leaf {leaf_index}'s path walked elsewhere");
        checked += 1;
    }
    assert_eq!(checked, 9);
}
