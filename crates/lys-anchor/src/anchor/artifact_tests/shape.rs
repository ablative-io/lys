#![cfg(test)]
//! Refused indices, two artifacts for one leaf at two sizes, and the
//! serialized shape and path-node encoding the wire contract names.

use super::*;

#[test]
fn inclusion_artifact_refuses_an_index_the_log_does_not_have() {
    let tmp = TempDir::new().unwrap();
    let anchor = anchor_over(tmp.path(), &[STATEMENT]);

    // Positive control: the indices that exist are served, so the refusals
    // below are about the index and not about a method that refuses always.
    let mut served = 0;
    for index in 0..2_u64 {
        anchor
            .inclusion_artifact(index)
            .expect("every logged leaf has an artifact");
        served += 1;
    }
    assert_eq!(served, 2);

    let mut refused = 0;
    for index in [2_u64, 3, u64::MAX] {
        let err = anchor
            .inclusion_artifact(index)
            .expect_err("an index past the end has no leaf");
        assert!(
            matches!(
                err,
                AnchorError::NoSuchLeaf { leaf_index, tree_size, ref origin }
                    if leaf_index == index && tree_size == 2 && origin == ORIGIN
            ),
            "an absent index must be refused by name, got: {err}"
        );
        refused += 1;
    }
    assert_eq!(refused, 3);

    // Refusing must not have appended anything on the caller's behalf.
    assert_eq!(anchor.tree_size(), 2);
}

#[test]
fn two_artifacts_for_one_leaf_at_two_sizes_disagree_and_both_verify() {
    // The contract the module docs state, made executable. A caller who takes
    // an artifact after further appends gets a different `tree_size` and a
    // different root for the same leaf, and neither artifact is wrong.
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    let anchor = anchor_over(dir, &[STATEMENT]);

    let earlier = anchor.inclusion_artifact(1).unwrap();
    assert_eq!(earlier.tree_size, 2);

    // Somebody else's statement lands, and the anchor is reopened over it.
    append_to(dir, b"a third party's statement");
    let anchor = reopen(dir);
    assert_eq!(anchor.tree_size(), 3);

    let later = anchor.inclusion_artifact(1).unwrap();
    assert_eq!(later.leaf_index, earlier.leaf_index);
    assert_ne!(later.tree_size, earlier.tree_size);
    assert_ne!(later.checkpoint, earlier.checkpoint);

    // Both verify, against different trees — that is the whole point, and a
    // caller who wants agreement has to check for it rather than assume it.
    let key = verifier(&anchor);
    let earlier_body = verify_checkpoint(earlier.checkpoint.as_bytes(), &key)
        .expect("the earlier artifact must still verify");
    let later_body = verify_checkpoint(later.checkpoint.as_bytes(), &key)
        .expect("the later artifact must verify");
    assert_eq!(earlier_body.tree_size(), 2);
    assert_eq!(later_body.tree_size(), 3);
    assert_ne!(earlier_body.root_hash(), later_body.root_hash());
    assert_eq!(hex(&earlier_body.root_hash()), GOLDEN_ROOT_2);

    // Each path leads to its own tree's root, and neither leads to the other's.
    let earlier_root =
        rfc6962_walk(STATEMENT, 1, earlier.tree_size, &path_nodes(&earlier)).unwrap();
    let later_root = rfc6962_walk(STATEMENT, 1, later.tree_size, &path_nodes(&later)).unwrap();
    assert_eq!(earlier_root, earlier_body.root_hash());
    assert_eq!(later_root, later_body.root_hash());
    assert_ne!(earlier_root, later_root);

    // And the leaf itself did not move: the statement is still at index 1, with
    // the leaf hash computed outside Rust.
    assert_eq!(
        hex(&Sha256::digest([&[0x00_u8], STATEMENT].concat())),
        GOLDEN_STATEMENT_LEAF_HASH
    );
}

#[test]
fn the_artifact_has_the_serialized_shape_the_wire_contract_names() {
    // ⚠️ SHAPE COVERAGE, NOT WIRE VERIFICATION. The round trip below goes
    // through our own `Serialize` and our own `Deserialize`, so any change made
    // symmetrically on both sides is invisible to it — it can show that the
    // type survives a trip, and nothing about what a stranger's parser sees.
    // What carries weight here is narrower and stated as such: the field names
    // and the JSON types are compared against literals written in this file,
    // and `deny_unknown_fields` is exercised with input this test supplied.
    // The wire claim proper belongs to the independent cross-language verifier.
    let tmp = TempDir::new().unwrap();
    let anchor = anchor_over(tmp.path(), &[STATEMENT]);
    let artifact = anchor.inclusion_artifact(1).unwrap();

    let text = serde_json::to_string(&artifact).unwrap();
    let value: serde_json::Value = serde_json::from_str(&text).unwrap();
    let object = value.as_object().expect("an artifact is a JSON object");

    // Exactly these five keys, no more and no fewer, named by literals.
    let expected = ["format", "tree_size", "leaf_index", "hashes", "checkpoint"];
    let mut seen = 0;
    for key in expected {
        assert!(
            object.contains_key(key),
            "the artifact has no `{key}` field"
        );
        seen += 1;
    }
    assert_eq!(seen, expected.len());
    assert_eq!(
        object.len(),
        expected.len(),
        "the artifact carries a field this contract does not name: {text}"
    );

    // The JSON types a stranger's parser will meet.
    assert_eq!(object["format"].as_str(), Some(INCLUSION_FORMAT));
    assert_eq!(object["tree_size"].as_u64(), Some(2));
    assert_eq!(object["leaf_index"].as_u64(), Some(1));
    assert_eq!(
        object["hashes"].as_array().map(Vec::len),
        Some(1),
        "a two-leaf tree's inclusion path is one node"
    );
    assert!(object["checkpoint"].as_str().is_some());

    // Shape coverage: the type survives its own round trip.
    let decoded: InclusionProofArtifact = serde_json::from_str(&text).unwrap();
    assert_eq!(decoded.format, artifact.format);
    assert_eq!(decoded.tree_size, artifact.tree_size);
    assert_eq!(decoded.leaf_index, artifact.leaf_index);
    assert_eq!(decoded.hashes, artifact.hashes);
    assert_eq!(decoded.checkpoint, artifact.checkpoint);

    // `deny_unknown_fields`, driven by input this test wrote: a v1 artifact
    // with an extra field is not a v1 artifact, and there is no smuggling into
    // a frozen shape. Without this the round trip alone would be consistent
    // with a decoder that accepts anything.
    let smuggled = text.replace("{\"format\"", "{\"witnessed\":true,\"format\"");
    assert_ne!(smuggled, text, "the injection must actually have applied");
    assert!(
        serde_json::from_str::<InclusionProofArtifact>(&smuggled).is_err(),
        "an unknown field must not be accepted into a frozen shape"
    );
}

#[test]
fn the_path_nodes_are_padded_standard_base64_of_whole_digests() {
    // The decoder in this file is the second party for the `hashes` field, so
    // it is itself controlled here before anything is concluded from it.
    let tmp = TempDir::new().unwrap();
    let statements: Vec<Vec<u8>> = (0..4_u8).map(|n| vec![n; 5]).collect();
    let borrowed: Vec<&[u8]> = statements.iter().map(Vec::as_slice).collect();
    let anchor = anchor_over(tmp.path(), &borrowed);

    let artifact = anchor.inclusion_artifact(2).unwrap();
    let nodes = path_nodes(&artifact);
    assert!(!nodes.is_empty(), "this case needs a non-empty path");

    // Positive control on the decoder: a known base64 string decodes to known
    // bytes. If the decoder returned `None` for everything, or empty output for
    // everything, `path_nodes` above would have failed loudly — but a decoder
    // that returned the *wrong* bytes consistently would not, and this catches
    // it against a vector nothing in this workspace produced.
    assert_eq!(
        decode_standard_base64("bHlz").as_deref(),
        Some(&b"lys"[..]),
        "the decoder does not decode a known vector"
    );
    assert_eq!(
        decode_standard_base64("bA==").as_deref(),
        Some(&b"l"[..]),
        "the decoder mishandles two-character padding"
    );
    assert_eq!(
        decode_standard_base64("bHk=").as_deref(),
        Some(&b"ly"[..]),
        "the decoder mishandles one-character padding"
    );

    // Negative controls: the decoder must refuse what the contract forbids,
    // otherwise the padding assertion in `path_nodes` is checking a decoder
    // that would have accepted the unpadded form anyway.
    let mut refused = 0;
    for bad in ["bHl", "bH!=", "bHlz=", "", "bA=A"] {
        assert!(
            decode_standard_base64(bad).is_none(),
            "the decoder accepted `{bad}`, which is not standard padded base64"
        );
        refused += 1;
    }
    assert_eq!(refused, 5);
}
