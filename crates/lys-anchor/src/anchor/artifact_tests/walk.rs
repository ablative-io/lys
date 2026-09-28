#![cfg(test)]
//! The embedded checkpoint verifies, the published path walks to the root
//! it signs, the declared sizes are the anchor's own, and a genesis-only
//! anchor has an empty path.

use super::*;

#[test]
fn the_embedded_checkpoint_verifies_under_a_verifier_built_from_the_literal_origin() {
    let tmp = TempDir::new().unwrap();
    let anchor = anchor_over(tmp.path(), &[STATEMENT]);

    let artifact = anchor.inclusion_artifact(1).unwrap();

    // The frozen wire marker, against a literal in this file.
    assert_eq!(artifact.format, INCLUSION_FORMAT);

    // The checkpoint is checked with the origin this test handed the store and
    // the key the signer advertises — not with anything the anchor reported.
    let body = verify_checkpoint(artifact.checkpoint.as_bytes(), &verifier(&anchor))
        .expect("the artifact's checkpoint must verify");
    assert_eq!(body.tree_size(), 2);

    // And the root it commits to is the one computed outside Rust, so a
    // symmetric error shared by every Rust path here would still be caught.
    assert_eq!(hex(&body.root_hash()), GOLDEN_ROOT_2);

    // Negative control. Without it, "verifies" could mean "this verifier
    // accepts anything": the same bytes, checked under a verifier named for a
    // different log, must be refused. The origin binding is what stops one
    // log's artifact being accepted for another.
    let wrong_name = NoteVerifierKey::new(OTHER_ORIGIN, anchor.signer().public_key()).unwrap();
    assert!(
        verify_checkpoint(artifact.checkpoint.as_bytes(), &wrong_name).is_err(),
        "a checkpoint signed for one origin must not verify under another's name"
    );

    // Second negative control, on the other half of the binding: the right
    // origin with a key that is not this anchor's.
    let wrong_key = NoteVerifierKey::new(ORIGIN, [0x11_u8; 32]).unwrap();
    assert!(
        verify_checkpoint(artifact.checkpoint.as_bytes(), &wrong_key).is_err(),
        "a checkpoint must not verify under a key that did not sign it"
    );

    // The checkpoint is carried verbatim, trailing newline included — a reader
    // re-verifies these exact bytes, so a trimmed note is an unverifiable one.
    assert!(
        artifact.checkpoint.ends_with('\n'),
        "the embedded note must be verbatim, including its trailing newline"
    );
}

#[test]
fn the_published_path_walks_to_the_root_the_checkpoint_signs() {
    // A two-leaf tree has a one-node path, and a one-node path is invariant
    // under reordering — so it cannot catch a path emitted in the wrong order.
    // This log is deep enough that almost every path has three nodes.
    let tmp = TempDir::new().unwrap();
    let statements: Vec<Vec<u8>> = (0..8_u8).map(|n| vec![n; 11]).collect();
    let borrowed: Vec<&[u8]> = statements.iter().map(Vec::as_slice).collect();
    let anchor = anchor_over(tmp.path(), &borrowed);
    assert_eq!(anchor.tree_size(), 9);

    let mut walked = 0;
    let mut multi_node_paths = 0;
    for index in 0..9_u64 {
        let artifact = anchor.inclusion_artifact(index).unwrap();
        let body = verify_checkpoint(artifact.checkpoint.as_bytes(), &verifier(&anchor))
            .expect("the artifact's checkpoint must verify");

        let nodes = path_nodes(&artifact);
        if nodes.len() > 1 {
            multi_node_paths += 1;
        }

        // The leaf the artifact is about, read off the log rather than
        // remembered from the loop that wrote it.
        let leaf: &[u8] = if index == 0 {
            GENESIS
        } else {
            &statements[usize::try_from(index).unwrap() - 1]
        };

        let root = rfc6962_walk(leaf, artifact.leaf_index, artifact.tree_size, &nodes)
            .expect("the published path must satisfy RFC 6962 §2.1.1");
        assert_eq!(
            root,
            body.root_hash(),
            "leaf {index}'s published path leads to a root the anchor does not sign"
        );
        walked += 1;
    }
    // Count what fired, and count the cases that carry the property: a suite of
    // one-node paths would satisfy every assertion above while proving nothing
    // about order.
    assert_eq!(walked, 9);
    assert!(
        multi_node_paths >= 8,
        "only {multi_node_paths} of 9 artifacts had a path long enough for order to matter"
    );

    // Positive control on the walk itself. Every assertion above compares two
    // values this file derived; if `rfc6962_walk` returned the checkpoint root
    // for *anything*, they would all pass. Reversing a three-node path must
    // reach a different root.
    let artifact = anchor.inclusion_artifact(3).unwrap();
    let mut reversed = path_nodes(&artifact);
    assert!(
        reversed.len() >= 3,
        "this case needs order to be observable"
    );
    reversed.reverse();
    let body = verify_checkpoint(artifact.checkpoint.as_bytes(), &verifier(&anchor)).unwrap();
    assert_ne!(
        rfc6962_walk(
            &statements[2],
            artifact.leaf_index,
            artifact.tree_size,
            &reversed
        ),
        Some(body.root_hash()),
        "the walk accepts a reordered path, so it cannot be evidence of order"
    );
}

#[test]
fn the_declared_sizes_are_the_anchors_own_and_the_index_is_the_one_asked_for() {
    let tmp = TempDir::new().unwrap();
    let statements: Vec<Vec<u8>> = (0..4_u8).map(|n| vec![n; 7]).collect();
    let borrowed: Vec<&[u8]> = statements.iter().map(Vec::as_slice).collect();
    let anchor = anchor_over(tmp.path(), &borrowed);
    assert_eq!(anchor.tree_size(), 5);

    let mut checked = 0;
    for index in 0..5_u64 {
        let artifact = anchor.inclusion_artifact(index).unwrap();
        assert_eq!(artifact.leaf_index, index);
        assert_eq!(artifact.tree_size, anchor.tree_size());

        // Not just equal to the anchor's field: equal to the size inside the
        // signed checkpoint the artifact carries, which is the only one a
        // reader is entitled to believe.
        let body = verify_checkpoint(artifact.checkpoint.as_bytes(), &verifier(&anchor))
            .expect("the artifact's checkpoint must verify");
        assert_eq!(body.tree_size(), artifact.tree_size);
        checked += 1;
    }
    assert_eq!(checked, 5);
}

#[test]
fn a_genesis_only_anchor_has_an_artifact_with_an_empty_path() {
    // `receipt_for` refuses this tree — RFC 9942 types a receipt's inclusion
    // path as one-or-more nodes and a one-leaf tree's path is empty. The JSON
    // artifact has no such rule, and this asserts that the difference is real
    // rather than an untested claim in the module docs. (`receipt_for` itself
    // cannot be named here: it is behind `unstable-anchor` and this file is not.)
    let tmp = TempDir::new().unwrap();
    let anchor = anchor_over(tmp.path(), &[]);
    assert_eq!(anchor.tree_size(), 1);

    let artifact = anchor.inclusion_artifact(0).unwrap();
    assert_eq!(artifact.tree_size, 1);
    assert_eq!(artifact.leaf_index, 0);
    assert!(
        artifact.hashes.is_empty(),
        "the sole leaf of a one-leaf tree has an empty path"
    );

    // The root of a one-leaf tree is its leaf hash — checked against a value
    // computed outside Rust, so the empty path is not merely accepted but
    // correct.
    let body = verify_checkpoint(artifact.checkpoint.as_bytes(), &verifier(&anchor))
        .expect("the artifact's checkpoint must verify");
    assert_eq!(hex(&body.root_hash()), GOLDEN_GENESIS_LEAF_HASH);
    assert_eq!(
        rfc6962_walk(GENESIS, 0, 1, &[]),
        Some(body.root_hash()),
        "RFC 6962 §2.1.1 must accept the empty path for a one-leaf tree"
    );
}
