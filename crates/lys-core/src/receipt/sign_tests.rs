//! Adversarial and end-to-end tests for receipt issuance and verification.
//!
//! Every proof here comes from a **real** `AppendOnlyTree`, never a synthetic
//! path, so the tests exercise the same bytes an anchor would issue. The attack
//! cases are constructed rather than sampled, per the crypto-change standard:
//! forgery, replay, misattribution, algorithm substitution, cross-protocol
//! confusion and malleability each get a case that would pass if the
//! corresponding check were removed.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use crate::attestation;
use crate::error::TrustError;
use crate::merkle::tree::{AppendOnlyTree, RawLeaf};

fn leaf_bytes(index: u64) -> Vec<u8> {
    format!("checkpoint-{index}").into_bytes()
}

fn tree_of(size: u64) -> AppendOnlyTree<RawLeaf> {
    let mut tree = AppendOnlyTree::<RawLeaf>::new();
    for index in 0..size {
        tree.append_raw(&leaf_bytes(index));
    }
    tree
}

/// The RFC 6962 path for `index` in a tree of `size`, as fixed-size nodes.
fn path_of(size: u64, index: u64) -> Vec<[u8; 32]> {
    let tree = tree_of(size);
    let proof = tree.prove_inclusion(index).unwrap();
    proof
        .as_bytes()
        .chunks_exact(32)
        .map(|c| <[u8; 32]>::try_from(c).unwrap())
        .collect()
}

/// Build a deterministic identity from a fixed 32-byte seed, so every test
/// below is reproducible and the failure cases name concrete keys.
fn anchor(seed: &[u8; 32]) -> crate::Ed25519Identity {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("anchor.key");
    std::fs::write(&path, seed).unwrap();
    crate::Ed25519Identity::load(&path).unwrap()
}

fn primary() -> crate::Ed25519Identity {
    anchor(b"lys-anchor-receipt-test-seed-01a")
}

fn other() -> crate::Ed25519Identity {
    anchor(b"lys-anchor-receipt-test-seed-02b")
}

/// The end-to-end property, over every leaf of every tree size a genesis-seeded
/// log can reach: issue a receipt from a real proof, then verify it.
#[test]
fn every_issued_receipt_verifies_at_every_size_and_index() {
    let key = primary();
    let expected = key.public_key_bytes();

    for size in 2..=33u64 {
        for index in 0..size {
            let path = path_of(size, index);
            let leaf = leaf_bytes(index);
            let receipt = sign_receipt(&leaf, index, size, &path, &key)
                .unwrap_or_else(|e| panic!("issuance failed at size {size} index {index}: {e}"));

            verify_receipt(&receipt, &leaf, &expected)
                .unwrap_or_else(|e| panic!("verify failed at size {size} index {index}: {e}"));

            // And through the wire, which is the only form that matters.
            let bytes = receipt.to_cose_bytes();
            let parsed = verify_receipt_bytes(&bytes, &leaf, &expected).unwrap();
            assert_eq!(parsed, receipt);
        }
    }
}

/// The signed root is the tree's actual root — the anchor vouches for a value
/// it derived, not one it was handed.
#[test]
fn the_signed_root_is_the_trees_own_root() {
    let key = primary();
    let size = 8;
    let index = 3;
    let tree = tree_of(size);
    let receipt =
        sign_receipt(&leaf_bytes(index), index, size, &path_of(size, index), &key).unwrap();
    assert_eq!(
        receipt.reconstructed_root(&leaf_bytes(index)).unwrap(),
        tree.root().to_parts().0
    );
}

#[test]
fn a_receipt_for_one_leaf_does_not_verify_for_another() {
    let key = primary();
    let size = 8;
    let receipt = sign_receipt(&leaf_bytes(3), 3, size, &path_of(size, 3), &key).unwrap();
    assert!(matches!(
        verify_receipt(&receipt, &leaf_bytes(4), &key.public_key_bytes()),
        Err(TrustError::ReceiptVerification)
    ));
    // Including a leaf that differs by a single byte.
    let mut nearly = leaf_bytes(3);
    nearly[0] ^= 0x01;
    assert!(verify_receipt(&receipt, &nearly, &key.public_key_bytes()).is_err());
}

/// Misattribution: a valid receipt from an anchor nobody trusts must not pass.
/// This is the same trap as a self-signed certificate — the signature proves
/// someone holding a key spoke, never that the statement is true.
#[test]
fn a_receipt_from_an_unexpected_anchor_is_refused_though_it_is_internally_perfect() {
    let attacker = other();
    let size = 8;
    let index = 3;
    let leaf = leaf_bytes(index);
    let receipt = sign_receipt(&leaf, index, size, &path_of(size, index), &attacker).unwrap();

    // It verifies against its own key: the artifact is not forged.
    verify_receipt(&receipt, &leaf, &attacker.public_key_bytes()).unwrap();

    // And is refused when the caller names the anchor they actually trust.
    assert!(matches!(
        verify_receipt(&receipt, &leaf, &primary().public_key_bytes()),
        Err(TrustError::ReceiptVerification)
    ));
}

#[test]
fn swapping_the_embedded_anchor_key_breaks_the_signature_too() {
    // The key rides in the signature-covered protected header, inheriting the
    // attestation v1→v2 fix. Substituting it invalidates the signature even if
    // the caller is fooled into expecting the substituted key.
    let key = primary();
    let size = 8;
    let index = 3;
    let leaf = leaf_bytes(index);
    let mut receipt = sign_receipt(&leaf, index, size, &path_of(size, index), &key).unwrap();

    receipt.anchor_public_key = other().public_key_bytes();
    assert!(verify_receipt(&receipt, &leaf, &other().public_key_bytes()).is_err());
}

#[test]
fn a_tampered_path_is_refused() {
    let key = primary();
    let size = 9;
    let index = 4;
    let leaf = leaf_bytes(index);
    let expected = key.public_key_bytes();

    for node in 0..path_of(size, index).len() {
        for bit in [0x01u8, 0x80] {
            let mut receipt =
                sign_receipt(&leaf, index, size, &path_of(size, index), &key).unwrap();
            receipt.inclusion_path[node][0] ^= bit;
            assert!(
                verify_receipt(&receipt, &leaf, &expected).is_err(),
                "a flipped bit in path node {node} was accepted"
            );
        }
    }
}

#[test]
fn reordering_the_path_is_refused() {
    let key = primary();
    let size = 16;
    let index = 5;
    let leaf = leaf_bytes(index);
    let mut receipt = sign_receipt(&leaf, index, size, &path_of(size, index), &key).unwrap();
    receipt.inclusion_path.reverse();
    assert!(verify_receipt(&receipt, &leaf, &key.public_key_bytes()).is_err());
}

#[test]
fn a_tampered_leaf_index_is_refused() {
    // Index 5 and index 6 sit at the same depth in a size-16 tree, so the path
    // length still matches and only the walk direction differs — the case a
    // length-only check would miss.
    let key = primary();
    let size = 16;
    let leaf = leaf_bytes(5);
    let mut receipt = sign_receipt(&leaf, 5, size, &path_of(size, 5), &key).unwrap();
    receipt.leaf_index = 6;
    assert!(verify_receipt(&receipt, &leaf, &key.public_key_bytes()).is_err());
}

#[test]
fn a_tampered_signature_is_refused() {
    let key = primary();
    let size = 8;
    let index = 3;
    let leaf = leaf_bytes(index);
    for byte in [0usize, 31, 32, 63] {
        let mut receipt = sign_receipt(&leaf, index, size, &path_of(size, index), &key).unwrap();
        receipt.signature[byte] ^= 0x01;
        assert!(verify_receipt(&receipt, &leaf, &key.public_key_bytes()).is_err());
    }
}

#[test]
fn a_non_canonical_signature_scalar_is_refused() {
    // Strict verification rejects a malleable `s`; non-repudiation requires a
    // unique valid signature per message. Setting the high bits of `s` puts it
    // outside the canonical range.
    let key = primary();
    let size = 8;
    let index = 3;
    let leaf = leaf_bytes(index);
    let mut receipt = sign_receipt(&leaf, index, size, &path_of(size, index), &key).unwrap();
    receipt.signature[63] |= 0xe0;
    assert!(verify_receipt(&receipt, &leaf, &key.public_key_bytes()).is_err());
}

#[test]
fn a_receipt_cannot_be_replayed_against_a_different_tree() {
    // The same leaf at the same index, but in a tree that grew. The root
    // changed, so the old signature does not cover the new shape.
    let key = primary();
    let index = 3;
    let leaf = leaf_bytes(index);
    let receipt = sign_receipt(&leaf, index, 8, &path_of(8, index), &key).unwrap();

    let mut replayed = receipt;
    replayed.tree_size = 9;
    replayed.inclusion_path = path_of(9, index);
    assert!(verify_receipt(&replayed, &leaf, &key.public_key_bytes()).is_err());
}

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

/// Cross-protocol confusion, in both directions and at the signature level —
/// not merely at the parser level.
#[test]
fn an_attestation_signature_is_never_a_valid_receipt_signature() {
    let key = primary();
    let size = 8;
    let index = 3;
    let leaf = leaf_bytes(index);

    // Sign an attestation over the receipt's root, the most favourable case for
    // an attacker: same key, same 32 bytes of "meaning".
    let receipt = sign_receipt(&leaf, index, size, &path_of(size, index), &key).unwrap();
    let root = receipt.reconstructed_root(&leaf).unwrap();
    let attestation = attestation::sign_attestation(&root, &key);

    // Graft the attestation's signature onto the receipt.
    let mut grafted = receipt.clone();
    grafted.signature = attestation.signature;
    assert!(
        verify_receipt(&grafted, &leaf, &key.public_key_bytes()).is_err(),
        "an attestation signature must not satisfy a receipt"
    );

    // And the reverse: the receipt's signature must not satisfy the attestation.
    let mut grafted_back = attestation;
    grafted_back.signature = receipt.signature;
    assert!(attestation::verify_attestation(&grafted_back, &root).is_err());
}

#[test]
fn the_two_artifact_families_sign_different_bytes_for_the_same_key() {
    // The COSE `Sig_structure` prefix is identical for both — `0x84 0x6A
    // "Signature1"` — so byte-0 disjointness does not separate them. What does
    // is the protected bucket inside the signed bytes.
    let key = primary();

    // Read each family's protected bucket out of a real artifact, through the
    // public API only — the buckets are what the signatures actually cover.
    let attestation_bytes = attestation::sign_attestation(b"payload", &key).to_cose_bytes();
    let attestation_protected = protected_bucket_of(&attestation_bytes);

    let leaf = leaf_bytes(3);
    let receipt_bytes = sign_receipt(&leaf, 3, 8, &path_of(8, 3), &key)
        .unwrap()
        .to_cose_bytes();
    let receipt_protected = protected_bucket_of(&receipt_bytes);

    assert_ne!(attestation_protected, receipt_protected);
    assert_eq!(attestation_protected[0], 0xa3, "attestation: 3-entry map");
    assert_eq!(receipt_protected[0], 0xa4, "receipt: 4-entry map");

    // Both signed preimages nonetheless start identically, which is exactly why
    // the separation has to live in the bucket rather than in the prefix.
    let attestation_preimage =
        crate::cbor::sig_structure_bytes(&attestation_protected, b"whatever");
    let receipt_preimage = crate::cbor::sig_structure_bytes(&receipt_protected, b"whatever");
    assert_eq!(&attestation_preimage[..12], &receipt_preimage[..12]);
    assert_eq!(&attestation_preimage[..12], b"\x84\x6aSignature1");
    assert_ne!(attestation_preimage, receipt_preimage);
}

/// Extract the protected bucket bstr from a tagged `COSE_Sign1`.
fn protected_bucket_of(cose: &[u8]) -> Vec<u8> {
    let ciborium::value::Value::Tag(18, boxed) = ciborium::de::from_reader(cose).unwrap() else {
        panic!("tagged COSE_Sign1")
    };
    let ciborium::value::Value::Array(items) = *boxed else {
        panic!("4-array")
    };
    let ciborium::value::Value::Bytes(protected) = &items[0] else {
        panic!("protected is a bstr")
    };
    protected.clone()
}

/// Non-oracle discipline: every rejection must be the same value, so a
/// network-exposed verifier cannot be used to learn which check failed.
#[test]
fn every_verification_failure_is_the_same_error() {
    let key = primary();
    let size = 8;
    let index = 3;
    let leaf = leaf_bytes(index);
    let expected = key.public_key_bytes();
    let good = sign_receipt(&leaf, index, size, &path_of(size, index), &key).unwrap();

    let mut wrong_anchor = good.clone();
    wrong_anchor.anchor_public_key = other().public_key_bytes();

    let mut bad_path = good.clone();
    bad_path.inclusion_path[0][0] ^= 0x01;

    let mut short_path = good.clone();
    short_path.inclusion_path.pop();

    let mut bad_index = good.clone();
    bad_index.leaf_index = 99;

    let mut bad_sig = good.clone();
    bad_sig.signature[0] ^= 0x01;

    let cases = [
        ("wrong anchor", wrong_anchor, leaf.clone()),
        ("tampered path", bad_path, leaf.clone()),
        ("inconsistent path length", short_path, leaf.clone()),
        ("index outside the tree", bad_index, leaf.clone()),
        ("forged signature", bad_sig, leaf),
        ("wrong leaf", good, leaf_bytes(4)),
    ];

    for (name, receipt, candidate_leaf) in cases {
        let err = verify_receipt(&receipt, &candidate_leaf, &expected).unwrap_err();
        assert!(
            matches!(err, TrustError::ReceiptVerification),
            "{name} produced a distinguishable error: {err:?}"
        );
        assert_eq!(
            format!("{err}"),
            format!("{}", TrustError::ReceiptVerification),
            "{name} produced a distinguishable message"
        );
    }
}

#[test]
fn malformed_bytes_and_bad_signatures_are_indistinguishable_through_the_bytes_api() {
    let key = primary();
    let size = 8;
    let index = 3;
    let leaf = leaf_bytes(index);
    let expected = key.public_key_bytes();
    let good = sign_receipt(&leaf, index, size, &path_of(size, index), &key)
        .unwrap()
        .to_cose_bytes();

    let mut truncated = good.clone();
    truncated.truncate(good.len() / 2);

    let mut trailing = good.clone();
    trailing.push(0x00);

    let mut forged = good;
    let last = forged.len() - 1;
    forged[last] ^= 0x01;

    for mutant in [truncated, trailing, forged, vec![], vec![0xff; 10]] {
        let err = verify_receipt_bytes(&mutant, &leaf, &expected).unwrap_err();
        assert!(matches!(err, TrustError::ReceiptVerification));
    }
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
