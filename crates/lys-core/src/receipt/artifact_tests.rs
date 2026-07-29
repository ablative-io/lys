//! Round-trip and canonical-encoding-strictness tests for [`AnchorReceipt`].
//!
//! The mutants here all have or could have **cryptographically valid
//! signatures** — they are re-encodings of a genuine receipt, not forgeries.
//! Vanilla COSE verifiers accept several of them. lys rejects them because a
//! receipt with two valid encodings is a receipt whose bytes are not its
//! identity, and every downstream comparison (bundle dedup, a log of issued
//! receipts, a cache key) then depends on which encoding you happened to see.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;

const ANCHOR_KEY: [u8; 32] = [0xa1; 32];
const SIGNATURE: [u8; 64] = [0x5e; 64];
const NODE_A: [u8; 32] = [0x11; 32];
const NODE_B: [u8; 32] = [0x22; 32];

/// A CBOR length that must fit a one-byte head, checked rather than truncated.
fn small(n: usize) -> u8 {
    u8::try_from(n).unwrap()
}

fn sample() -> AnchorReceipt {
    AnchorReceipt {
        anchor_public_key: ANCHOR_KEY,
        tree_size: 4,
        leaf_index: 1,
        inclusion_path: vec![NODE_A, NODE_B],
        signature: SIGNATURE,
    }
}

#[test]
fn round_trip_is_identity_on_the_value() {
    let receipt = sample();
    let parsed = AnchorReceipt::from_cose_bytes(&receipt.to_cose_bytes()).unwrap();
    assert_eq!(parsed, receipt);
}

#[test]
fn round_trip_is_identity_on_the_bytes() {
    let bytes = sample().to_cose_bytes();
    let reencoded = AnchorReceipt::from_cose_bytes(&bytes)
        .unwrap()
        .to_cose_bytes();
    assert_eq!(reencoded, bytes);
}

#[test]
fn round_trip_holds_across_the_interesting_shapes() {
    let cases: Vec<(u64, u64, Vec<[u8; 32]>)> = vec![
        (2, 0, vec![NODE_A]),
        (2, 1, vec![NODE_A]),
        (1, 0, vec![]),
        (1000, 999, vec![NODE_A; 10]),
        (u64::MAX, u64::MAX - 1, vec![NODE_B; 64]),
    ];
    for (tree_size, leaf_index, inclusion_path) in cases {
        let receipt = AnchorReceipt {
            anchor_public_key: ANCHOR_KEY,
            tree_size,
            leaf_index,
            inclusion_path,
            signature: SIGNATURE,
        };
        let bytes = receipt.to_cose_bytes();
        assert_eq!(AnchorReceipt::from_cose_bytes(&bytes).unwrap(), receipt);
    }
}

#[test]
fn trailing_garbage_is_refused() {
    // ciborium stops at the end of the first value, so only the re-encode gate
    // catches this. It is the reason that gate exists.
    let mut bytes = sample().to_cose_bytes();
    bytes.push(0x00);
    assert!(AnchorReceipt::from_cose_bytes(&bytes).is_err());

    let mut bytes = sample().to_cose_bytes();
    bytes.extend_from_slice(&[0xff; 16]);
    assert!(AnchorReceipt::from_cose_bytes(&bytes).is_err());
}

#[test]
fn an_indefinite_length_array_is_refused() {
    // `9f ... ff` instead of `84`: valid CBOR, same parsed value, different
    // bytes. RFC 8949 §4.2 forbids it in deterministic encoding.
    let good = sample().to_cose_bytes();
    let mut indefinite = vec![0xd2, 0x9f];
    indefinite.extend_from_slice(&good[2..]);
    indefinite.push(0xff);
    assert!(AnchorReceipt::from_cose_bytes(&indefinite).is_err());
}

#[test]
fn an_oversized_integer_head_is_refused() {
    // Encode tree size 4 as `19 00 04` (two-byte head) rather than `04`. The
    // value is identical; the bytes are not shortest-form.
    let good = sample().to_cose_bytes();
    let needle = [0x83, 0x04, 0x01]; // array(3), size 4, index 1
    let at = good
        .windows(needle.len())
        .position(|w| w == needle)
        .expect("inner proof header present");

    let mut fat = good[..at].to_vec();
    fat.extend_from_slice(&[0x83, 0x19, 0x00, 0x04, 0x01]);
    fat.extend_from_slice(&good[at + needle.len()..]);
    // The inner proof grew by two bytes, so its bstr length prefix is now wrong
    // and the document is malformed — which is itself a rejection. Fix the
    // prefix so the mutant is well-formed CBOR and the *canonicality* gate is
    // what rejects it.
    let prefix_at = at - 2;
    assert_eq!(fat[prefix_at], 0x58);
    fat[prefix_at + 1] += 2;
    assert!(
        ciborium::de::from_reader::<ciborium::value::Value, _>(fat.as_slice()).is_ok(),
        "the mutant must be well-formed CBOR for this test to mean anything"
    );
    assert!(AnchorReceipt::from_cose_bytes(&fat).is_err());
}

#[test]
fn a_reordered_protected_map_is_refused() {
    // Swap labels 3 and 4 in the protected bucket. Every value is unchanged and
    // a permissive verifier reads the same headers; the bytes differ, so the
    // signature over them would differ too.
    let good = sample().to_cose_bytes();
    let protected = &good[4..84];

    let mut reordered = vec![0xa4, 0x01, 0x27];
    reordered.extend_from_slice(&[0x04, 0x58, 0x20]);
    reordered.extend_from_slice(&ANCHOR_KEY);
    reordered.extend_from_slice(&protected[3..41]); // label 3 + content type
    reordered.extend_from_slice(&[0x19, 0x01, 0x8b, 0x01]);
    assert_eq!(reordered.len(), 80);

    let mut mutant = good[..4].to_vec();
    mutant.extend_from_slice(&reordered);
    mutant.extend_from_slice(&good[84..]);
    assert!(AnchorReceipt::from_cose_bytes(&mutant).is_err());
}

#[test]
fn a_duplicate_protected_key_is_refused() {
    // `{1: -8, 1: -8, 3: ct, 4: kid, 395: 1}` — a 5-entry map whose duplicate
    // is the shape that splits verifiers into "first wins" and "last wins".
    let good = sample().to_cose_bytes();
    let mut protected = vec![0xa5, 0x01, 0x27, 0x01, 0x27];
    protected.extend_from_slice(&good[6..84]);

    let mut mutant = vec![0xd2, 0x84, 0x58, small(protected.len())];
    mutant.extend_from_slice(&protected);
    mutant.extend_from_slice(&good[84..]);
    assert!(AnchorReceipt::from_cose_bytes(&mutant).is_err());
}

#[test]
fn parsing_is_not_verification_and_says_so() {
    // A receipt with a garbage signature and an attacker-chosen anchor key
    // parses perfectly. This is deliberate — `from_cose_bytes` is labelled as
    // parsing — and it is why nothing may trust a parsed receipt's fields.
    let receipt = AnchorReceipt {
        anchor_public_key: [0xff; 32],
        tree_size: 4,
        leaf_index: 1,
        inclusion_path: vec![NODE_A, NODE_B],
        signature: [0x00; 64],
    };
    let parsed = AnchorReceipt::from_cose_bytes(&receipt.to_cose_bytes()).unwrap();
    assert_eq!(parsed.anchor_public_key, [0xff; 32]);
}

#[test]
fn reconstructed_root_is_consistent_with_the_merkle_walk() {
    let tree_size = 4;
    let leaf_index = 1;
    let receipt = sample();
    let leaf = b"a leaf";

    let mut flattened = NODE_A.to_vec();
    flattened.extend_from_slice(&NODE_B);
    let expected = crate::merkle::root_from_inclusion_path(
        &crate::merkle::raw_leaf_hash(leaf),
        leaf_index,
        tree_size,
        &flattened,
    )
    .unwrap();

    assert_eq!(receipt.reconstructed_root(leaf).unwrap(), expected);
}

#[test]
fn reconstructed_root_reports_inconsistent_proofs_with_a_reason() {
    // The diagnostic path is deliberately not non-oracle: it is for an operator
    // holding their own receipt. `verify_receipt` collapses the same failures.
    let inconsistent = AnchorReceipt {
        anchor_public_key: ANCHOR_KEY,
        tree_size: 4,
        leaf_index: 1,
        inclusion_path: vec![NODE_A], // one node short for a size-4 tree
        signature: SIGNATURE,
    };
    let err = inconsistent.reconstructed_root(b"a leaf").unwrap_err();
    assert!(
        matches!(err, crate::error::TrustError::MerkleTree { .. }),
        "expected an actionable Merkle error, got {err:?}"
    );
    assert!(!format!("{err}").is_empty());
}

#[test]
fn an_index_outside_the_tree_cannot_reconstruct() {
    let bad = AnchorReceipt {
        anchor_public_key: ANCHOR_KEY,
        tree_size: 4,
        leaf_index: 4,
        inclusion_path: vec![NODE_A, NODE_B],
        signature: SIGNATURE,
    };
    assert!(bad.reconstructed_root(b"a leaf").is_err());
}

#[test]
fn the_debug_form_carries_no_private_material() {
    // A receipt holds only public material, but the check is asserted rather
    // than assumed: the struct is constructed from an identity's key, and a
    // future field could carry more than intended.
    let rendered = format!("{:?}", sample());
    assert!(rendered.contains("AnchorReceipt"));
    assert!(!rendered.to_lowercase().contains("seed"));
    assert!(!rendered.to_lowercase().contains("secret"));
    assert!(!rendered.to_lowercase().contains("private"));
}
