#![cfg(test)]
//! The frontier's fold is written apart from `lys-core`'s tree, so the tests
//! hold the two to one root at every size, and pin the empty and one-leaf
//! roots as literals so neither side can drift by agreeing with the other.

use lys_core::merkle::{AppendOnlyTree, RawLeaf};

use super::*;

fn leaf(index: u64) -> Vec<u8> {
    format!("leaf-{index}").into_bytes()
}

#[test]
fn the_frontier_root_matches_the_full_tree_at_every_size() {
    let mut frontier = Frontier::new();
    let mut tree = AppendOnlyTree::<RawLeaf>::new();
    let mut compared = 0;
    for index in 0..130 {
        assert_eq!(frontier.root_hash(), tree.root(), "size {index}");
        assert_eq!(
            frontier.nodes().len(),
            usize::try_from(frontier.size().count_ones()).unwrap()
        );
        let bytes = leaf(index);
        frontier.push(&bytes);
        tree.append_raw(&bytes);
        compared += 1;
    }
    assert_eq!(frontier.root_hash(), tree.root());
    assert_eq!(compared, 130, "every size was compared");
}

#[test]
fn the_empty_root_is_the_sha256_of_nothing() {
    let empty = Frontier::new().root();
    assert_eq!(
        empty,
        [
            0xe3, 0xb0, 0xc4, 0x42, 0x98, 0xfc, 0x1c, 0x14, 0x9a, 0xfb, 0xf4, 0xc8, 0x99, 0x6f,
            0xb9, 0x24, 0x27, 0xae, 0x41, 0xe4, 0x64, 0x9b, 0x93, 0x4c, 0xa4, 0x95, 0x99, 0x1b,
            0x78, 0x52, 0xb8, 0x55,
        ]
    );
}

#[test]
fn a_one_leaf_root_is_the_leaf_hash() {
    let mut frontier = Frontier::new();
    let leaf_hash = frontier.push(b"leaf-0");
    assert_eq!(frontier.root(), leaf_hash);
    assert_eq!(leaf_hash, raw_leaf_hash(b"leaf-0"));
}

#[test]
fn a_frontier_rebuilt_from_its_parts_extends_to_the_same_root() {
    let mut whole = Frontier::from_leaves((0..37).map(leaf));
    let mut resumed = Frontier::from_parts(whole.size(), whole.nodes().to_vec()).unwrap();
    for index in 37..80 {
        whole.push(&leaf(index));
        resumed.push(&leaf(index));
    }
    assert_eq!(resumed, whole);
    assert_eq!(
        resumed.root_hash(),
        AppendOnlyTree::<RawLeaf>::reconstruct_from_raw_leaves((0..80).map(leaf)).root()
    );
}

#[test]
fn parts_whose_node_count_is_not_the_sizes_set_bits_are_refused() {
    let err = Frontier::from_parts(6, vec![[0u8; 32]]).unwrap_err();
    assert!(
        matches!(
            err,
            StoreError::FrontierMalformed {
                size: 6,
                nodes: 1,
                expected: 2
            }
        ),
        "{err}"
    );
    assert!(Frontier::from_parts(0, vec![[0u8; 32]]).is_err());
    assert!(Frontier::from_parts(0, Vec::new()).is_ok());
}
