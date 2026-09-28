#![cfg(test)]
//! Decoder refusals: structural mutants, the payload slot, the protected
//! header pins, the proof shape, inner proofs, path length, signature length,
//! and the attestation family.

use super::*;

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
