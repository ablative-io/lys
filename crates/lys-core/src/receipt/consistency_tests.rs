#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Gates on `lys/consistency-receipt/v1`.
//!
//! # The re-labelling attack is constructed here, not argued
//!
//! The wire draft rules that the two receipt kinds must carry different content
//! types, on the grounds that they otherwise sign byte-identical `Sig_structure`
//! bytes and an inclusion receipt could be presented as a consistency one with a
//! valid signature. Until `a_relabelled_inclusion_receipt_is_refused` existed
//! that was a hypothesis about our own code. It now builds the artifact and
//! requires the refusal.

use super::*;
use crate::keys::identity::Ed25519Identity;
use crate::merkle::{AppendOnlyTree, RawLeaf};
use crate::receipt::sign_receipt;

const ANCHOR_SEED: &[u8; 32] = b"lys-consistency-receipt-seed-001";
/// A second anchor, for proving a receipt does not verify under a key that did
/// not issue it. Typed `&[u8; 32]` so a wrong length is a compile error.
const OTHER_SEED: &[u8; 32] = b"lys-consistency-OTHER-anchor-key";

fn anchor() -> (tempfile::TempDir, Ed25519Identity) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("anchor.key");
    std::fs::write(&path, ANCHOR_SEED).unwrap();
    let identity = Ed25519Identity::load(&path).unwrap();
    (dir, identity)
}

fn leaf(index: u64) -> Vec<u8> {
    format!("consistency-receipt-leaf-{index}").into_bytes()
}

fn tree_of(size: u64) -> AppendOnlyTree<RawLeaf> {
    let mut tree = AppendOnlyTree::<RawLeaf>::new();
    for index in 0..size {
        tree.append_raw(&leaf(index));
    }
    tree
}

fn root_of(size: u64) -> [u8; 32] {
    let (root, _size) = tree_of(size).root().to_parts();
    root
}

/// The `SUBPROOF` between two sizes, as 32-byte nodes.
fn path_between(old_size: u64, new_size: u64) -> Vec<[u8; 32]> {
    tree_of(new_size)
        .prove_consistency(old_size, new_size)
        .unwrap()
        .as_bytes()
        .chunks_exact(32)
        .map(|chunk| chunk.try_into().unwrap())
        .collect()
}

/// A signed receipt for the growth from `old_size` to `new_size`.
fn signed(old_size: u64, new_size: u64, key: &Ed25519Identity) -> ConsistencyReceipt {
    sign_consistency_receipt(
        &root_of(old_size),
        old_size,
        new_size,
        &path_between(old_size, new_size),
        key,
    )
    .unwrap()
}

#[test]
fn a_signed_receipt_verifies_and_returns_the_newer_root() {
    let (_dir, key) = anchor();
    let mut verified = 0;
    for new_size in 2u64..=13 {
        for old_size in 1u64..new_size {
            let receipt = signed(old_size, new_size, &key);
            let new_root =
                verify_consistency_receipt(&receipt, &key.public_key_bytes(), &root_of(old_size))
                    .unwrap_or_else(|err| panic!("({old_size} -> {new_size}): {err}"));
            assert_eq!(
                new_root,
                root_of(new_size),
                "({old_size} -> {new_size}) verification returned the wrong newer root"
            );
            verified += 1;
        }
    }
    assert_eq!(verified, 78, "12*13/2 pairs with 1 <= old < new <= 13");
}

#[test]
fn a_relabelled_inclusion_receipt_is_refused() {
    // THE attack the differing content type exists to stop, built rather than
    // argued. An inclusion receipt signs root R at size S with a detached
    // payload; a consistency receipt signs the NEWER root the same way. Splice
    // the inclusion receipt's real signature and key into a consistency
    // artifact whose proof derives exactly R, and the signature is genuine over
    // exactly the right bytes — the ONLY thing standing in the way is the
    // content type inside the signed bucket.
    let (_dir, key) = anchor();
    let (old_size, new_size) = (5u64, 13u64);

    // A real inclusion receipt from this anchor over the size-13 tree, whose
    // signature therefore covers root_of(13).
    let tree = tree_of(new_size);
    let proof = tree.prove_inclusion(0).unwrap();
    let inclusion_path: Vec<[u8; 32]> = proof
        .as_bytes()
        .chunks_exact(32)
        .map(|chunk| chunk.try_into().unwrap())
        .collect();
    let inclusion = sign_receipt(&leaf(0), 0, new_size, &inclusion_path, &key).unwrap();

    // The consistency proof for 5 -> 13 derives exactly root_of(13) — the same
    // 32 bytes the inclusion receipt's signature covers.
    let derived = crate::merkle::root_from_consistency_path(
        &root_of(old_size),
        old_size,
        new_size,
        &path_between(old_size, new_size)
            .iter()
            .flatten()
            .copied()
            .collect::<Vec<u8>>(),
    )
    .unwrap();
    assert_eq!(
        derived,
        root_of(new_size),
        "the fixture is only an attack if both artifacts are about the same root"
    );

    // Graft: the inclusion receipt's signature, presented as a consistency one.
    let relabelled = ConsistencyReceipt {
        anchor_public_key: inclusion.anchor_public_key,
        tree_size_1: old_size,
        tree_size_2: new_size,
        consistency_path: path_between(old_size, new_size),
        signature: inclusion.signature,
    };
    let err = verify_consistency_receipt(&relabelled, &key.public_key_bytes(), &root_of(old_size))
        .unwrap_err();
    assert!(matches!(err, TrustError::ReceiptVerification), "{err}");

    // And the control that makes the refusal meaningful: the inclusion receipt
    // is genuinely valid on its own terms, so the rejection above is the
    // re-labelling and not a broken fixture.
    crate::receipt::verify_receipt(&inclusion, &leaf(0), &key.public_key_bytes())
        .expect("the inclusion receipt must be valid, or the attack proves nothing");
}

#[test]
fn an_inclusion_artifact_does_not_parse_as_a_consistency_receipt() {
    // The same separation one layer earlier: refused at the protected header,
    // before any proof is examined.
    let (_dir, key) = anchor();
    let tree = tree_of(8);
    let path: Vec<[u8; 32]> = tree
        .prove_inclusion(3)
        .unwrap()
        .as_bytes()
        .chunks_exact(32)
        .map(|chunk| chunk.try_into().unwrap())
        .collect();
    let inclusion_bytes = sign_receipt(&leaf(3), 3, 8, &path, &key)
        .unwrap()
        .to_cose_bytes();

    assert!(ConsistencyReceipt::from_cose_bytes(&inclusion_bytes).is_err());

    // And the reverse direction, so this is a separation rather than one
    // decoder simply being stricter than the other.
    let consistency_bytes = signed(5, 13, &key).to_cose_bytes();
    assert!(crate::receipt::AnchorReceipt::from_cose_bytes(&consistency_bytes).is_err());
}

#[test]
fn issuance_refuses_equal_sizes() {
    // The ruling reversed in `308d95e`: an equal-size proof leaves no derivation
    // standing, so the signature check would degenerate to an existential query
    // over the anchor's signing history.
    let (_dir, key) = anchor();
    let err = sign_consistency_receipt(&root_of(7), 7, 7, &[], &key).unwrap_err();
    assert!(format!("{err}").contains("strictly below"), "{err}");
}

#[test]
fn verification_refuses_equal_sizes_even_with_a_genuine_signature() {
    // Issuance refusing is not enough on its own: a hand-built artifact never
    // passes through `sign_consistency_receipt`. This is the verifier's own
    // refusal, with a signature the anchor really did produce over the value
    // an equal-size "derivation" would return.
    let (_dir, key) = anchor();
    let old_root = root_of(7);
    let protected = crate::receipt::encoding::protected_bytes(
        crate::receipt::encoding::CONSISTENCY_CONTENT_TYPE,
        &key.public_key_bytes(),
    );
    let signature = key.sign(&crate::cbor::sig_structure_bytes(&protected, &old_root));
    let forged = ConsistencyReceipt {
        anchor_public_key: key.public_key_bytes(),
        tree_size_1: 7,
        tree_size_2: 7,
        consistency_path: Vec::new(),
        signature,
    };
    let err = verify_consistency_receipt(&forged, &key.public_key_bytes(), &old_root).unwrap_err();
    assert!(matches!(err, TrustError::ReceiptVerification), "{err}");
}

#[test]
fn the_older_root_must_be_the_callers_own() {
    // A verifier that took the older root from the artifact would let the anchor
    // pick both endpoints. Here the caller supplies a root it did not hold.
    let (_dir, key) = anchor();
    let receipt = signed(5, 13, &key);
    let mut wrong = root_of(5);
    wrong[0] ^= 0x01;
    let err = verify_consistency_receipt(&receipt, &key.public_key_bytes(), &wrong).unwrap_err();
    assert!(matches!(err, TrustError::ReceiptVerification), "{err}");
}

#[test]
fn a_receipt_from_another_anchor_is_refused() {
    // A receipt verifies against whatever key it carries, so the caller must
    // name the anchor it expects — the same trap as a self-signed certificate.
    let (_dir, key) = anchor();
    let other_dir = tempfile::tempdir().unwrap();
    let other_path = other_dir.path().join("other.key");
    std::fs::write(&other_path, OTHER_SEED).unwrap();
    let other = Ed25519Identity::load(&other_path).unwrap();

    let receipt = signed(5, 13, &key);
    let err =
        verify_consistency_receipt(&receipt, &other.public_key_bytes(), &root_of(5)).unwrap_err();
    assert!(matches!(err, TrustError::ReceiptVerification), "{err}");
}

#[test]
fn tampering_with_the_path_is_refused() {
    let (_dir, key) = anchor();
    let mut tampered = signed(5, 13, &key);
    tampered.consistency_path[0][0] ^= 0x01;
    let err =
        verify_consistency_receipt(&tampered, &key.public_key_bytes(), &root_of(5)).unwrap_err();
    assert!(matches!(err, TrustError::ReceiptVerification), "{err}");
}

#[test]
fn round_trip_is_identity_on_the_bytes() {
    let (_dir, key) = anchor();
    let mut swept = 0;
    for new_size in 2u64..=9 {
        for old_size in 1u64..new_size {
            let bytes = signed(old_size, new_size, &key).to_cose_bytes();
            let reencoded = ConsistencyReceipt::from_cose_bytes(&bytes)
                .unwrap_or_else(|err| panic!("({old_size} -> {new_size}): {err}"))
                .to_cose_bytes();
            assert_eq!(reencoded, bytes);
            swept += 1;
        }
    }
    assert_eq!(swept, 36, "8*9/2 pairs with 1 <= old < new <= 9");
}

#[test]
fn a_non_canonical_encoding_is_refused_even_when_the_signature_would_verify() {
    // Canonical-encoding strictness: trailing bytes make the artifact differ
    // from the canonical re-encoding of its own fields.
    let (_dir, key) = anchor();
    let mut bytes = signed(5, 13, &key).to_cose_bytes();
    bytes.push(0x00);
    assert!(ConsistencyReceipt::from_cose_bytes(&bytes).is_err());
}

#[test]
fn verify_bytes_returns_the_same_root_as_the_two_step_path() {
    let (_dir, key) = anchor();
    let bytes = signed(5, 13, &key).to_cose_bytes();
    let (parsed, root) =
        verify_consistency_receipt_bytes(&bytes, &key.public_key_bytes(), &root_of(5)).unwrap();
    assert_eq!(root, root_of(13));
    assert_eq!(parsed.to_cose_bytes(), bytes);
}

#[test]
fn every_verification_failure_is_the_same_error() {
    // Non-oracle: a forged signature must not be distinguishable from a wrong
    // anchor, a bad older root, or a malformed proof. This held once by variant
    // coincidence and broke the moment receipts stopped reusing
    // `InvalidSignature` — hence the explicit case count.
    let (_dir, key) = anchor();
    let good = signed(5, 13, &key);

    let mut wrong_key = good.clone();
    wrong_key.anchor_public_key[0] ^= 0x01;
    let mut wrong_sig = good.clone();
    wrong_sig.signature[0] ^= 0x01;
    let mut wrong_sizes = good.clone();
    wrong_sizes.tree_size_2 = 99;
    let mut wrong_path = good.clone();
    wrong_path.consistency_path.pop();

    let failures = [
        verify_consistency_receipt(&wrong_key, &key.public_key_bytes(), &root_of(5)),
        verify_consistency_receipt(&wrong_sig, &key.public_key_bytes(), &root_of(5)),
        verify_consistency_receipt(&wrong_sizes, &key.public_key_bytes(), &root_of(5)),
        verify_consistency_receipt(&wrong_path, &key.public_key_bytes(), &root_of(5)),
        verify_consistency_receipt(&good, &key.public_key_bytes(), &[0u8; 32]),
    ];
    let mut counted = 0;
    for outcome in &failures {
        let err = outcome.as_ref().unwrap_err();
        assert!(
            matches!(err, TrustError::ReceiptVerification),
            "a distinguishable failure is an oracle: {err}"
        );
        counted += 1;
    }
    assert_eq!(counted, 5, "every failure mode must have been exercised");
}
