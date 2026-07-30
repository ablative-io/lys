//! Byte-exactness and structural-rejection tests for
//! `lys/anchor-receipt/v1`.
//!
//! Expected bytes here are **hand-assembled from the specification**, not
//! captured from the encoder's own output. A golden file produced by the code
//! under test only proves the code is self-consistent; these literals prove it
//! agrees with RFC 9052, RFC 8949 and RFC 9942. If the encoder changes, these
//! fail — which is the point, because the format is frozen the moment an anchor
//! signs under it.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use ciborium::value::Value;

use super::*;

/// A fixed anchor key. Value is irrelevant to encoding; only its length is.
const ANCHOR_KEY: [u8; 32] = [0xa1; 32];
/// A fixed signature.
const SIGNATURE: [u8; 64] = [0x5e; 64];
/// Two fixed path nodes.
const NODE_A: [u8; 32] = [0x11; 32];
const NODE_B: [u8; 32] = [0x22; 32];

/// A CBOR length that must fit a one-byte head, checked rather than truncated.
fn small(n: usize) -> u8 {
    u8::try_from(n).unwrap()
}

/// The protected bucket, written out byte by byte from the spec table.
fn expected_protected(key: &[u8; 32]) -> Vec<u8> {
    let mut out = vec![
        0xa4, // map(4)
        0x01, 0x27, // 1 (alg) => -8 (EdDSA)
        0x03, 0x78, 0x23, // 3 (content type) => text(35)
    ];
    out.extend_from_slice(b"application/vnd.lys.receipt.v1+cbor");
    out.extend_from_slice(&[0x04, 0x58, 0x20]); // 4 (kid) => bstr(32)
    out.extend_from_slice(key);
    out.extend_from_slice(&[0x19, 0x01, 0x8b, 0x01]); // 395 (vds) => 1
    out
}

/// The inner `bstr .cbor` inclusion proof, written out from the RFC 9942 CDDL.
fn expected_inner_proof(tree_size: u8, leaf_index: u8, nodes: &[[u8; 32]]) -> Vec<u8> {
    let mut out = vec![0x83, tree_size, leaf_index]; // array(3), two small uints
    out.push(0x80 | small(nodes.len())); // array(n), n < 24
    for node in nodes {
        out.extend_from_slice(&[0x58, 0x20]);
        out.extend_from_slice(node);
    }
    out
}

/// The unprotected bucket `{396: {-1: [<bstr .cbor proof>]}}`.
fn expected_unprotected(tree_size: u8, leaf_index: u8, nodes: &[[u8; 32]]) -> Vec<u8> {
    let proof = expected_inner_proof(tree_size, leaf_index, nodes);
    let mut out = vec![
        0xa1, // map(1)
        0x19, 0x01, 0x8c, // 396 (vdp)
        0xa1, // map(1)
        0x20, // -1 (inclusion)
        0x81, // array(1) — exactly one proof
    ];
    assert!(proof.len() < 256);
    out.extend_from_slice(&[0x58, small(proof.len())]);
    out.extend_from_slice(&proof);
    out
}

/// A canonical two-node receipt over tree size 4, leaf index 1.
fn sample() -> Vec<u8> {
    artifact_bytes(&ANCHOR_KEY, 4, 1, &[NODE_A, NODE_B], &SIGNATURE)
}

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

/// Every structural mutant must be refused. Each case is a shape a permissive
/// COSE library would accept.
#[test]
fn structural_mutants_are_all_refused() {
    let good = sample();

    // Oversize input, rejected before parsing.
    assert!(decode_fields(&vec![0u8; MAX_ARTIFACT_LEN + 1]).is_err());

    // Tag stripped: an untagged COSE_Sign1 is a different object.
    let mut untagged = good.clone();
    untagged.remove(0);
    assert!(decode_fields(&untagged).is_err());

    // Wrong tag (17 = COSE_Mac0).
    let mut wrong_tag = good.clone();
    wrong_tag[0] = 0xd1;
    assert!(decode_fields(&wrong_tag).is_err());

    // Trailing garbage is caught by the caller's re-encode gate, but a
    // truncated artifact must fail here.
    assert!(decode_fields(&good[..good.len() - 1]).is_err());

    // Empty and near-empty inputs.
    assert!(decode_fields(&[]).is_err());
    assert!(decode_fields(&[0xd2]).is_err());
}

#[test]
fn a_non_nil_payload_is_refused() {
    // A receipt's root is detached. An artifact carrying any payload is not a
    // receipt, even if the payload happens to be the correct root.
    let good = sample();
    let payload_offset = 2 + 2 + 80 + expected_unprotected(4, 1, &[NODE_A, NODE_B]).len();
    assert_eq!(good[payload_offset], 0xf6);

    // Replace nil with an empty bstr — the other common "no payload" encoding.
    let mut empty_bstr = good.clone();
    empty_bstr[payload_offset] = 0x40;
    assert!(decode_fields(&empty_bstr).is_err());

    // Replace nil with `false`, another simple value.
    let mut falsey = good;
    falsey[payload_offset] = 0xf4;
    assert!(decode_fields(&falsey).is_err());
}

#[test]
fn every_protected_header_pin_is_enforced() {
    // The protected bucket's bytes start after `d2 84 58 50`.
    const P: usize = 4;

    let good = sample();

    // alg -7 (ES256) substituted for -8. An algorithm-substitution attack is
    // the classic COSE confusion, and the pin is what stops it.
    let mut alg = good.clone();
    assert_eq!(alg[P + 2], 0x27, "alg value byte");
    alg[P + 2] = 0x26; // -7
    assert!(decode_fields(&alg).is_err());

    // alg -19 (the RFC 9864 preference we deliberately did not adopt).
    let mut alg19 = good.clone();
    alg19[P + 2] = 0x32; // -19
    assert!(decode_fields(&alg19).is_err());

    // Content type mutated by one character.
    let mut ct = good.clone();
    let ct_at = P + 6;
    assert_eq!(&ct[ct_at..ct_at + 11], b"application");
    ct[ct_at] = b'A';
    assert!(decode_fields(&ct).is_err());

    // vds 2 instead of 1: a different verifiable data structure entirely.
    let mut vds = good.clone();
    let vds_at = P + 79;
    assert_eq!(vds[vds_at], 0x01, "vds value byte");
    vds[vds_at] = 0x02;
    assert!(decode_fields(&vds).is_err());

    // A 3-entry protected map (attestation's arity) with vds dropped.
    let mut arity = good;
    arity[P] = 0xa3;
    assert!(decode_fields(&arity).is_err());
}

#[test]
fn every_vdp_shape_deviation_is_refused() {
    // Rebuilt rather than spliced, so each mutant is a well-formed CBOR
    // document that differs only in the vdp's shape.
    let protected = expected_protected(&ANCHOR_KEY);
    let proof = expected_inner_proof(4, 1, &[NODE_A, NODE_B]);

    let assemble = |unprotected: Vec<u8>| {
        let mut out = vec![0xd2, 0x84, 0x58, 0x50];
        out.extend_from_slice(&protected);
        out.extend_from_slice(&unprotected);
        out.push(0xf6);
        out.extend_from_slice(&[0x58, 0x40]);
        out.extend_from_slice(&SIGNATURE);
        out
    };
    let wrapped = |body: Vec<u8>| {
        let mut out = vec![0xa1, 0x19, 0x01, 0x8c];
        out.extend_from_slice(&body);
        out
    };

    // An empty unprotected map: the proof is simply missing.
    assert!(decode_fields(&assemble(vec![0xa0])).is_err());

    // The proof as a bare array at -1, dropping the array-of-proofs wrapper.
    let mut bare = vec![0xa1, 0x20];
    bare.extend_from_slice(&proof);
    assert!(decode_fields(&assemble(wrapped(bare))).is_err());

    // The proof as a bare bstr at -1, dropping only the outer array.
    let mut bare_bstr = vec![0xa1, 0x20, 0x58, small(proof.len())];
    bare_bstr.extend_from_slice(&proof);
    assert!(decode_fields(&assemble(wrapped(bare_bstr))).is_err());

    // An unwrapped array inside the proofs array, dropping the bstr .cbor.
    let mut unwrapped = vec![0xa1, 0x20, 0x81];
    unwrapped.extend_from_slice(&proof);
    assert!(decode_fields(&assemble(wrapped(unwrapped))).is_err());

    // Zero proofs.
    assert!(decode_fields(&assemble(wrapped(vec![0xa1, 0x20, 0x80]))).is_err());

    // Two proofs. RFC 9942 permits it; lys does not, because a receipt carries
    // one signature over one root and a reader could act on a proof the
    // verifier never checked.
    let mut two = vec![0xa1, 0x20, 0x82];
    for _ in 0..2 {
        two.extend_from_slice(&[0x58, small(proof.len())]);
        two.extend_from_slice(&proof);
    }
    assert!(decode_fields(&assemble(wrapped(two))).is_err());

    // A consistency proof (-2), which is specified but not issued at launch.
    let mut consistency = vec![0xa1, 0x21, 0x81, 0x58, small(proof.len())];
    consistency.extend_from_slice(&proof);
    assert!(decode_fields(&assemble(wrapped(consistency))).is_err());

    // An extra unprotected entry alongside the vdp.
    let mut extra = vec![
        0xa2,
        0x19,
        0x01,
        0x8c,
        0xa1,
        0x20,
        0x81,
        0x58,
        small(proof.len()),
    ];
    extra.extend_from_slice(&proof);
    extra.extend_from_slice(&[0x01, 0x27]); // a smuggled alg in the clear
    assert!(decode_fields(&assemble(extra)).is_err());

    // The vdp under the wrong label (395, vds).
    let mut wrong_label = vec![
        0xa1,
        0x19,
        0x01,
        0x8b,
        0xa1,
        0x20,
        0x81,
        0x58,
        small(proof.len()),
    ];
    wrong_label.extend_from_slice(&proof);
    assert!(decode_fields(&assemble(wrong_label)).is_err());
}

#[test]
fn malformed_inner_proofs_are_refused() {
    let protected = expected_protected(&ANCHOR_KEY);
    let assemble = |proof: Vec<u8>| {
        let mut out = vec![0xd2, 0x84, 0x58, 0x50];
        out.extend_from_slice(&protected);
        out.extend_from_slice(&[0xa1, 0x19, 0x01, 0x8c, 0xa1, 0x20, 0x81]);
        assert!(proof.len() < 256);
        out.extend_from_slice(&[0x58, small(proof.len())]);
        out.extend_from_slice(&proof);
        out.push(0xf6);
        out.extend_from_slice(&[0x58, 0x40]);
        out.extend_from_slice(&SIGNATURE);
        out
    };

    // A 2-array and a 4-array instead of the required 3.
    assert!(decode_fields(&assemble(vec![0x82, 0x04, 0x01])).is_err());
    assert!(decode_fields(&assemble(vec![0x84, 0x04, 0x01, 0x80, 0x00])).is_err());

    // A negative tree size or leaf index: `uint` in the CDDL, so major type 1
    // is out of type even though it parses as an integer.
    assert!(decode_fields(&assemble(vec![0x83, 0x20, 0x01, 0x80])).is_err());
    assert!(decode_fields(&assemble(vec![0x83, 0x04, 0x20, 0x80])).is_err());

    // A path node that is not 32 bytes.
    let mut short_node = vec![0x83, 0x04, 0x01, 0x81, 0x58, 0x1f];
    short_node.extend_from_slice(&[0x11; 31]);
    assert!(decode_fields(&assemble(short_node)).is_err());

    // A path element that is not a byte string.
    assert!(decode_fields(&assemble(vec![0x83, 0x04, 0x01, 0x81, 0x01])).is_err());

    // The path as a bstr rather than an array of bstr.
    let mut flat = vec![0x83, 0x04, 0x01, 0x58, 0x20];
    flat.extend_from_slice(&NODE_A);
    assert!(decode_fields(&assemble(flat)).is_err());
}

#[test]
fn a_path_longer_than_any_tree_could_require_is_refused_before_hashing() {
    // 64 nodes is the maximum for a tree of up to u64::MAX leaves, so 65 is
    // structurally impossible rather than merely large.
    let nodes = vec![NODE_A; MAX_PATH_ELEMENTS + 1];
    let bytes = artifact_bytes(&ANCHOR_KEY, u64::MAX, 0, &nodes, &SIGNATURE);
    assert!(decode_fields(&bytes).is_err());

    // Exactly 64 is structurally acceptable (whether it is *consistent* is the
    // reconstruction's business, not the decoder's).
    let at_limit = vec![NODE_A; MAX_PATH_ELEMENTS];
    let bytes = artifact_bytes(&ANCHOR_KEY, u64::MAX, 0, &at_limit, &SIGNATURE);
    assert_eq!(
        decode_fields(&bytes).unwrap().inclusion_path.len(),
        MAX_PATH_ELEMENTS
    );
}

#[test]
fn a_signature_of_the_wrong_length_is_refused() {
    let protected = expected_protected(&ANCHOR_KEY);
    let build = |sig_head: &[u8], sig: &[u8]| {
        let mut out = vec![0xd2, 0x84, 0x58, 0x50];
        out.extend_from_slice(&protected);
        out.extend_from_slice(&expected_unprotected(4, 1, &[NODE_A, NODE_B]));
        out.push(0xf6);
        out.extend_from_slice(sig_head);
        out.extend_from_slice(sig);
        out
    };
    assert!(decode_fields(&build(&[0x58, 0x3f], &[0x5e; 63])).is_err());
    assert!(decode_fields(&build(&[0x58, 0x41], &[0x5e; 65])).is_err());
    assert!(decode_fields(&build(&[0x40], &[])).is_err());
}

#[test]
fn an_attestation_artifact_never_decodes_as_a_receipt() {
    // Cross-protocol confusion: both are tagged COSE_Sign1 signed by the same
    // kind of key. Only the pinned content type and shape separate them.
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("id.key");
    std::fs::write(&path, b"receipt-cross-protocol-seed-0001").unwrap();
    let identity = crate::Ed25519Identity::load(&path).unwrap();
    let attestation = crate::attestation::sign_attestation(b"payload", &identity);
    assert!(decode_fields(&attestation.to_cose_bytes()).is_err());
}

#[test]
fn a_receipt_never_decodes_as_an_attestation() {
    assert!(crate::attestation::Attestation::from_cose_bytes(&sample()).is_err());
}
