//! D6 conformance gate for `lys/anchor-receipt/v1`: round-trip the receipt
//! implementation against the vendored Go `veraison/go-cose`.
//!
//! This gate carries more weight than the attestation one, because the Go side
//! independently derives the value the signature covers. A receipt's payload is
//! the **detached** Merkle root, so the Go tool recomputes it from RFC 6962
//! §2.1's *recursive* `MTH` while lys computes it with an *iterative* upward
//! walk — go-cose's signature check passes only if two independently written
//! algorithms agree. It also re-derives the inclusion path from the RFC's
//! recursive `PATH` and requires it to equal the one the artifact carries.
//!
//! Gated with the format it tests: with `unstable-anchor` off there is no
//! `receipt` module, and compiling the whole file out leaves no unused shared
//! harness behind either.
//!
//! # Environment contract
//!
//! See [`harness`] — vendored, network-free, and a hard failure rather than a
//! skip when `LYS_REQUIRE_GO` is set.

#![cfg(feature = "unstable-anchor")]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod harness;

use harness::{build_go_tool, go_or_skip, run_built_tool};
use lys_core::Ed25519Identity;
use lys_core::merkle::tree::{AppendOnlyTree, RawLeaf};
use lys_core::receipt::{AnchorReceipt, sign_receipt, verify_receipt_bytes};

const RECEIPT_SEED: &[u8; 32] = b"lys-receipt-conformance-seed-001";

fn receipt_identity() -> (tempfile::TempDir, Ed25519Identity) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("anchor.key");
    std::fs::write(&path, RECEIPT_SEED).unwrap();
    let identity = Ed25519Identity::load(&path).unwrap();
    (dir, identity)
}

fn receipt_leaf(index: u64) -> Vec<u8> {
    format!("anchor-checkpoint-{index}").into_bytes()
}

fn to_hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    bytes.iter().fold(String::new(), |mut acc, b| {
        let _ = write!(acc, "{b:02x}");
        acc
    })
}

/// Issue a receipt from a real tree of `size` for `index`.
fn issue(size: u64, index: u64, key: &Ed25519Identity) -> (AnchorReceipt, [u8; 32]) {
    let mut tree = AppendOnlyTree::<RawLeaf>::new();
    for i in 0..size {
        tree.append_raw(&receipt_leaf(i));
    }
    let proof = tree.prove_inclusion(index).unwrap();
    let path: Vec<[u8; 32]> = proof
        .as_bytes()
        .chunks_exact(32)
        .map(|c| <[u8; 32]>::try_from(c).unwrap())
        .collect();
    let receipt = sign_receipt(&receipt_leaf(index), index, size, &path, key).unwrap();
    (receipt, tree.root().to_parts().0)
}

/// The D6 gate for receipts: a receipt no off-the-shelf library verifies is
/// worthless, so go-cose must verify ours.
///
/// This test carries more weight than the attestation gate, because the Go side
/// independently derives the values the signature covers. The receipt's payload
/// is the **detached** Merkle root, and the Go tool recomputes it from RFC 6962
/// §2.1's *recursive* `MTH` definition while lys computes it with an *iterative*
/// upward walk. go-cose's signature check therefore passes only if two
/// independently written algorithms agree on the root — and the Go tool also
/// re-derives the inclusion path from the RFC's recursive `PATH` and requires it
/// to equal the one the artifact carries.
///
/// That closes the hand-written-walk risk from a second direction. The 561
/// in-crate cases prove the walk agrees with `ct-merkle`'s tree; this proves it
/// agrees with the RFC's own recursive definition, transcribed in another
/// language by different code.
#[test]
fn go_cose_receipt_conformance_round_trips() {
    let Some(go) = go_or_skip("go-cose receipt conformance") else {
        return;
    };
    let workdir = tempfile::tempdir().unwrap();
    let gocache = workdir.path().join("gocache");
    let bin = workdir.path().join("cosetool");
    build_go_tool(&go, &gocache, &bin);

    let (_dir, key) = receipt_identity();
    let pubkey_hex = to_hex(&key.public_key_bytes());
    let seed_hex = to_hex(RECEIPT_SEED);

    // ---- Shape sweep (Rust -> Go), every leaf of every size a genesis-seeded
    // log passes through. Incomplete trees are where an iterative walk and a
    // recursive definition part company, so sampling would prove little.
    let mut sweep_cases = 0usize;
    for size in 2..=17u64 {
        for index in 0..size {
            sweep_cases += 1;
            let (receipt, expected_root) = issue(size, index, &key);
            let mut args = vec![
                "receipt-verify".to_string(),
                pubkey_hex.clone(),
                index.to_string(),
            ];
            args.extend((0..size).map(|i| to_hex(&receipt_leaf(i))));

            let (ok, stdout) = run_built_tool(&bin, &args, &receipt.to_cose_bytes());
            assert!(
                ok,
                "go-cose rejected the lys receipt at size {size} index {index}"
            );
            assert_eq!(
                String::from_utf8(stdout).unwrap(),
                format!("{} {size} {index}\n", to_hex(&expected_root)),
                "the Go recursive MTH disagreed with lys at size {size} index {index}"
            );
        }
    }
    // A loop that silently ran zero times would pass every assertion above.
    assert_eq!(
        sweep_cases, 152,
        "the shape sweep must cover every leaf of sizes 2..=17"
    );

    // ---- Go -> Rust: a receipt built entirely by go-cose, with its path
    // derived from RFC 6962's recursive PATH, must verify under lys.
    let size = 5u64;
    let index = 2u64;
    let mut args = vec!["receipt-sign".to_string(), seed_hex, index.to_string()];
    args.extend((0..size).map(|i| to_hex(&receipt_leaf(i))));
    let (ok, go_receipt) = run_built_tool(&bin, &args, &[]);
    assert!(ok, "go-cose receipt-sign failed");

    let restored =
        verify_receipt_bytes(&go_receipt, &receipt_leaf(index), &key.public_key_bytes()).unwrap();
    assert_eq!(restored.tree_size, size);
    assert_eq!(restored.leaf_index, index);

    // Byte-identity is the stronger claim and it holds: Ed25519 is
    // deterministic and both sides encode core-deterministic CBOR, so the two
    // implementations produce the same artifact rather than merely compatible
    // ones. If this ever fails, check the encoders before the crypto.
    let (ours, _root) = issue(size, index, &key);
    assert_eq!(
        go_receipt,
        ours.to_cose_bytes(),
        "go-cose and lys receipts must be byte-identical"
    );

    // ---- Negative parity: a tampered path node must be rejected by BOTH. The
    // Go side rejects it at the path comparison; lys rejects it because the
    // recomputed root is one the anchor never signed.
    let (mut tampered, _root) = issue(size, index, &key);
    tampered.inclusion_path[0][0] ^= 0x01;
    let mut args = vec![
        "receipt-verify".to_string(),
        pubkey_hex.clone(),
        index.to_string(),
    ];
    args.extend((0..size).map(|i| to_hex(&receipt_leaf(i))));
    let (ok, _stdout) = run_built_tool(&bin, &args, &tampered.to_cose_bytes());
    assert!(!ok, "go-cose accepted a receipt with a tampered path");
    assert!(
        verify_receipt_bytes(
            &tampered.to_cose_bytes(),
            &receipt_leaf(index),
            &key.public_key_bytes()
        )
        .is_err()
    );

    // ---- And a receipt presented against the wrong leaf: the Go side computes
    // a root over different leaves, so the detached payload no longer matches.
    let (receipt, _root) = issue(size, index, &key);
    let mut args = vec!["receipt-verify".to_string(), pubkey_hex, index.to_string()];
    args.extend((0..size).map(|i| to_hex(&receipt_leaf(i + 100))));
    let (ok, _stdout) = run_built_tool(&bin, &args, &receipt.to_cose_bytes());
    assert!(!ok, "go-cose accepted a receipt against the wrong leaves");
}
