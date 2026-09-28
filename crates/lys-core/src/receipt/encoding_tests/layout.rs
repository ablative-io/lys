#![cfg(test)]
//! The bytes the encoder writes: content types, the protected bucket, the
//! artifact layout, the detached payload, the verifiable data proof, and the
//! fields decoding recovers.

use super::*;

#[test]
fn the_content_type_is_the_frozen_slash_free_string() {
    // F2's lesson: a one-character confusion between a slash context tag and a
    // hyphen HKDF info is permanent in a public reference. Pinned literally.
    assert_eq!(CONTENT_TYPE, "application/vnd.lys.receipt.v1+cbor");
    assert_eq!(CONTENT_TYPE.len(), 35);
    assert!(
        !CONTENT_TYPE.contains("anchor-receipt"),
        "the media type uses the dotted vnd. form, not the context-tag form"
    );
}

#[test]
fn protected_bucket_matches_the_specification_bytes() {
    let built = protected_bytes(CONTENT_TYPE, &ANCHOR_KEY);
    assert_eq!(built, expected_protected(&ANCHOR_KEY));
    assert_eq!(built.len(), 80, "the protected bucket is always 80 bytes");
}

#[test]
fn the_consistency_content_type_is_the_frozen_string() {
    assert_eq!(
        CONSISTENCY_CONTENT_TYPE,
        "application/vnd.lys.consistency-receipt.v1+cbor"
    );
    assert_eq!(CONSISTENCY_CONTENT_TYPE.len(), 47);
}

/// The consistency protected bucket, written out from the spec table exactly as
/// [`expected_protected`] is — the same 92 bytes a conforming implementation
/// would assemble, not whatever this crate's encoder happens to emit.
fn expected_consistency_protected(key: &[u8; 32]) -> Vec<u8> {
    let mut out = vec![
        0xa4, // map(4)
        0x01, 0x27, // 1 (alg) => -8 (EdDSA)
        0x03, 0x78, 0x2f, // 3 (content type) => text(47)
    ];
    out.extend_from_slice(b"application/vnd.lys.consistency-receipt.v1+cbor");
    out.extend_from_slice(&[0x04, 0x58, 0x20]); // 4 (kid) => bstr(32)
    out.extend_from_slice(key);
    out.extend_from_slice(&[0x19, 0x01, 0x8b, 0x01]); // 395 (vds) => 1
    out
}

#[test]
fn the_consistency_protected_bucket_matches_the_specification_bytes() {
    let built = protected_bytes(CONSISTENCY_CONTENT_TYPE, &ANCHOR_KEY);
    assert_eq!(built, expected_consistency_protected(&ANCHOR_KEY));
    assert_eq!(built.len(), 92, "80 plus the 12-byte longer media type");
}

#[test]
fn the_two_receipt_kinds_sign_different_bytes() {
    // THE gate on the re-labelling attack. It asserts ONLY the separation
    // property — the spec bytes of each bucket are pinned by their own tests
    // above, and repeating those pins here would make this case fail whenever
    // either constant changed, for reasons having nothing to do with
    // separation. The rule this case owns is that a consistency code path
    // passing the INCLUSION constant must be caught, and that mistake is a
    // single token at a call site.
    let inclusion = protected_bytes(CONTENT_TYPE, &ANCHOR_KEY);
    let consistency = protected_bytes(CONSISTENCY_CONTENT_TYPE, &ANCHOR_KEY);
    assert_ne!(inclusion, consistency);

    // The consequence that actually matters: the COSE signing preimages differ,
    // so an anchor's signature over one can never satisfy the other. The
    // `Sig_structure` prefix is identical for both — byte-0 disjointness does
    // not separate them, the protected bucket inside the signed bytes does.
    let root = [0x33u8; 32];
    let inclusion_preimage = crate::cbor::sig_structure_bytes(&inclusion, &root);
    let consistency_preimage = crate::cbor::sig_structure_bytes(&consistency, &root);
    assert_eq!(&inclusion_preimage[..12], b"\x84\x6aSignature1");
    assert_eq!(&consistency_preimage[..12], b"\x84\x6aSignature1");
    assert_ne!(
        inclusion_preimage, consistency_preimage,
        "identical preimages would make an inclusion receipt re-labellable"
    );
}

#[test]
fn protected_bucket_is_fixed_width_for_every_key() {
    for byte in [0x00u8, 0x7f, 0xff] {
        assert_eq!(protected_bytes(CONTENT_TYPE, &[byte; 32]).len(), 80);
    }
}

#[test]
fn protected_labels_ascend_so_numeric_and_bytewise_order_coincide() {
    // RFC 8949 §4.2 sorts by encoded key bytes. For {1, 3, 4, 395} that is the
    // same as ascending numeric order, and the invariant docs claim so.
    let built = protected_bytes(CONTENT_TYPE, &ANCHOR_KEY);
    let Value::Map(entries) = ciborium::de::from_reader(built.as_slice()).unwrap() else {
        panic!("protected bucket is a map");
    };
    let labels: Vec<i128> = entries
        .iter()
        .map(|(k, _)| match k {
            Value::Integer(i) => i128::from(*i),
            other => panic!("non-integer label: {other:?}"),
        })
        .collect();
    assert_eq!(labels, vec![1, 3, 4, 395]);
    assert!(labels.windows(2).all(|w| w[0] < w[1]));
}

#[test]
fn artifact_layout_matches_the_specification_bytes() {
    let built = sample();

    let mut expected = vec![0xd2, 0x84]; // tag(18), array(4)
    let protected = expected_protected(&ANCHOR_KEY);
    expected.extend_from_slice(&[0x58, 0x50]); // bstr(80)
    expected.extend_from_slice(&protected);
    expected.extend_from_slice(&expected_unprotected(4, 1, &[NODE_A, NODE_B]));
    expected.push(0xf6); // nil — detached payload
    expected.extend_from_slice(&[0x58, 0x40]); // bstr(64)
    expected.extend_from_slice(&SIGNATURE);

    assert_eq!(built, expected);
}

#[test]
fn the_payload_slot_is_cbor_nil_because_the_root_is_detached() {
    let built = sample();
    // The payload sits immediately after the unprotected bucket.
    let offset = 2 + 2 + 80 + expected_unprotected(4, 1, &[NODE_A, NODE_B]).len();
    assert_eq!(built[offset], 0xf6, "payload must be nil, never the root");

    // And the root must appear nowhere in the artifact: a verifier that could
    // read it would not have to recompute it.
    let root = crate::merkle::root_from_inclusion_path(&[0x33; 32], 1, 4, &{
        let mut p = NODE_A.to_vec();
        p.extend_from_slice(&NODE_B);
        p
    })
    .unwrap();
    assert!(
        !built.windows(32).any(|w| w == root),
        "the signed root must not be carried in the artifact"
    );
}

#[test]
fn the_vdp_reproduces_the_rfc_9942_double_wrapper() {
    // Both nesting levels are easy to drop and dropping either would make the
    // receipt unparseable by every conforming implementation.
    let built = sample();
    let Value::Tag(18, boxed) = ciborium::de::from_reader(built.as_slice()).unwrap() else {
        panic!("tagged");
    };
    let Value::Array(items) = *boxed else {
        panic!("4-array")
    };
    let Value::Map(unprotected) = &items[1] else {
        panic!("map")
    };
    let (label, vdp) = &unprotected[0];
    assert_eq!(*label, Value::Integer(396.into()));

    let Value::Map(by_type) = vdp else {
        panic!("vdp is a map keyed by proof type")
    };
    let (proof_type, proofs) = &by_type[0];
    assert_eq!(*proof_type, Value::Integer((-1).into()));

    // Level 1: an ARRAY of proofs, not a bare proof.
    let Value::Array(proofs) = proofs else {
        panic!("the value at -1 is an array of proofs")
    };
    assert_eq!(proofs.len(), 1);

    // Level 2: each proof is a bstr WRAPPING cbor, not a bare array.
    let Value::Bytes(raw) = &proofs[0] else {
        panic!("each proof is a bstr .cbor")
    };
    let Value::Array(proof) = ciborium::de::from_reader(raw.as_slice()).unwrap() else {
        panic!("the wrapped value is the 3-array")
    };
    assert_eq!(proof.len(), 3);
    assert_eq!(proof[0], Value::Integer(4.into()));
    assert_eq!(proof[1], Value::Integer(1.into()));
    let Value::Array(path) = &proof[2] else {
        panic!("path array")
    };
    assert_eq!(path.len(), 2);
}

#[test]
fn decoding_recovers_every_encoded_field() {
    let fields = decode_fields(&sample()).unwrap();
    assert_eq!(fields.anchor_public_key, ANCHOR_KEY);
    assert_eq!(fields.tree_size, 4);
    assert_eq!(fields.leaf_index, 1);
    assert_eq!(fields.inclusion_path, vec![NODE_A, NODE_B]);
    assert_eq!(fields.signature, SIGNATURE);
}

#[test]
fn large_sizes_and_indices_round_trip_through_the_shortest_form() {
    // The uint heads widen; the decoder must still recover the values.
    for (size, index) in [(2u64, 0u64), (1000, 999), (u64::MAX, u64::MAX - 1)] {
        let nodes = [NODE_A];
        let bytes = artifact_bytes(&ANCHOR_KEY, size, index, &nodes, &SIGNATURE);
        let fields = decode_fields(&bytes).unwrap();
        assert_eq!(fields.tree_size, size);
        assert_eq!(fields.leaf_index, index);
    }
}

#[test]
fn an_empty_path_decodes_because_it_is_the_true_path_for_a_one_leaf_tree() {
    // lys never issues one (see sign_receipt), but refusing to *parse* a
    // mathematically correct proof another implementation made would be
    // refusing a true statement. Nothing is admitted: the reconstruction
    // independently requires the length that (index, size) demands.
    let bytes = artifact_bytes(&ANCHOR_KEY, 1, 0, &[], &SIGNATURE);
    let fields = decode_fields(&bytes).unwrap();
    assert!(fields.inclusion_path.is_empty());
    assert_eq!(fields.tree_size, 1);
}
