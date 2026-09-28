#![cfg(test)]
//! What the script refuses: a changed leaf byte, a flipped path node, a
//! changed leaf index, a contradicting tree size, an unknown field and the
//! wrong format label.

use super::*;

#[test]
fn the_script_refuses_a_leaf_with_one_byte_changed() {
    let Some(python) = python_or_skip("lys-anchor stranger verification") else {
        return;
    };
    let case = build_case(3);

    // Keyed on the leaf's own first byte rather than on a literal this test
    // chose, so it stays a corruption whatever the fixture holds.
    let mut corrupted = case.leaf.clone();
    corrupted[0] ^= 0x01;
    assert_ne!(corrupted, case.leaf, "the corruption must change a byte");
    assert_eq!(corrupted.len(), case.leaf.len(), "only the byte may change");
    std::fs::write(&case.leaf_path, &corrupted).unwrap();

    assert_refused(&python, &case, None, "a leaf with one byte flipped");
    case.close().unwrap();
}

#[test]
fn the_script_refuses_a_flipped_node_in_the_inclusion_path() {
    let Some(python) = python_or_skip("lys-anchor stranger verification") else {
        return;
    };
    let case = build_case(3);

    let mut value = case.artifact.clone();
    let hashes = value["hashes"].as_array_mut().unwrap();
    assert!(
        hashes.len() > 1,
        "leaf 3 of a five-leaf tree must have a multi-node path"
    );
    let node = hashes[0].as_str().unwrap();
    // One base64 character, which is six bits of the first byte. The character
    // is chosen against the node's own leading character, so this is a change
    // for any node the fixture happens to produce.
    let head = if node.starts_with('A') { 'B' } else { 'A' };
    let flipped = format!("{head}{}", &node[1..]);
    assert_ne!(flipped, node, "the flip must change the node");
    hashes[0] = serde_json::Value::String(flipped);
    rewrite_artifact(&case, &value);

    assert_refused(&python, &case, None, "a flipped node in the inclusion path");
    case.close().unwrap();
}

#[test]
fn the_script_refuses_a_changed_leaf_index() {
    let Some(python) = python_or_skip("lys-anchor stranger verification") else {
        return;
    };
    let case = build_case(3);

    let mut value = case.artifact.clone();
    assert_eq!(value["leaf_index"].as_u64(), Some(3));
    // Index 2 is leaf 3's sibling, so the path is the same length and the same
    // nodes: only the sidedness of the innermost step changes. A verifier that
    // ignored the index entirely — hashing the path in a fixed order — would
    // still accept this, which is why the sibling is the case to use rather
    // than an index from another part of the tree.
    value["leaf_index"] = serde_json::Value::from(2u64);
    rewrite_artifact(&case, &value);

    assert_refused(&python, &case, None, "a changed leaf_index");
    case.close().unwrap();
}

#[test]
fn the_script_refuses_a_tree_size_that_contradicts_the_checkpoint() {
    let Some(python) = python_or_skip("lys-anchor stranger verification") else {
        return;
    };
    let case = build_case(3);

    let mut value = case.artifact.clone();
    assert_eq!(value["tree_size"].as_u64(), Some(TREE_SIZE));
    // Six rather than four, and this is the whole case rather than a detail:
    // for leaf 3, tree sizes 5, 6, 7 and 8 share one RFC 6962 sidedness
    // sequence, so a five-leaf artifact relabelled `tree_size: 6` recomputes
    // the five-leaf root and matches the five-leaf checkpoint. Nothing in the
    // Merkle walk can notice. Only the comparison against the signed
    // checkpoint's own size refuses it — see the module docs. Four would have
    // been caught by the path length instead, and would have proven nothing
    // about this rule.
    value["tree_size"] = serde_json::Value::from(TREE_SIZE + 1);
    rewrite_artifact(&case, &value);

    assert_refused(
        &python,
        &case,
        None,
        "a tree_size the checkpoint contradicts",
    );
    case.close().unwrap();
}

#[test]
fn the_script_refuses_an_artifact_carrying_an_unknown_field() {
    let Some(python) = python_or_skip("lys-anchor stranger verification") else {
        return;
    };
    let case = build_case(3);

    // `InclusionProofArtifact` is `deny_unknown_fields` (D2: unknown fields in a
    // v1 artifact are not valid v1). A stranger's verifier that shrugged at an
    // extra field would accept a shape `lys-core` itself rejects, and the two
    // would disagree about what a v1 artifact is.
    let mut value = case.artifact.clone();
    value["smuggled"] = serde_json::Value::from("not part of v1");
    rewrite_artifact(&case, &value);

    assert_refused(&python, &case, None, "an artifact with an unknown field");
    case.close().unwrap();
}

#[test]
fn the_script_refuses_an_artifact_labelled_with_the_wrong_format() {
    let Some(python) = python_or_skip("lys-anchor stranger verification") else {
        return;
    };
    let case = build_case(3);

    // The consistency format, on an inclusion artifact: everything else about
    // the file is genuine and recomputes correctly, so only the kind check can
    // catch this. Cross-kind confusion is the attack the `format` field exists
    // to make impossible.
    let mut value = case.artifact.clone();
    value["format"] = serde_json::Value::from("lys/log-consistency-proof/v1");
    rewrite_artifact(&case, &value);

    assert_refused(
        &python,
        &case,
        None,
        "an artifact labelled as the wrong kind",
    );
    case.close().unwrap();
}
