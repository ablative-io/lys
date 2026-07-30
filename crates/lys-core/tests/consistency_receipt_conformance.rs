//! D6 conformance gate for `lys/consistency-receipt/v1`: round-trip the
//! artifact against the vendored Go `veraison/go-cose`.
//!
//! `consistency_conformance` already gates the *derivation* — lys's iterative
//! walk against RFC 6962's recursive `SUBPROOF` and §2.1.4.2, transcribed in Go.
//! It says nothing about the COSE envelope those roots are carried in, and a
//! receipt whose root is right but whose envelope no conforming library accepts
//! is exactly as useless as one with the wrong root. This gate covers the
//! artifact.
//!
//! Two claims per case, and the second is the load-bearing one:
//!
//! | direction | what a failure would mean |
//! |---|---|
//! | lys signs, Go verifies | go-cose cannot check a receipt lys issues |
//! | Go signs, bytes compared | the two encoders disagree about the artifact |
//!
//! Byte-identity is asserted for **every** shape rather than a sample, because
//! it is available: Ed25519 is deterministic and both sides emit RFC 8949 §4.2
//! core-deterministic CBOR, so the two implementations must produce the same
//! artifact and not merely compatible ones. And it only holds if they first
//! agreed on the detached payload — the Go side derives the newer root with
//! *its own* recursion over *its own* `SUBPROOF`, so equal bytes means equal
//! roots, equal proof encodings and equal header pins at once.
//!
//! # What this gate does and does not establish
//!
//! It is a **parity** gate: two implementations, one verdict. It is not the
//! rule-isolation suite — that is `receipt::consistency_tests`, where each check
//! has a case only it can catch. The negative cases here are about the two
//! implementations *agreeing* to refuse, which is the property a stranger
//! depends on and which no amount of in-crate testing can show.
//!
//! The re-label parity is the one worth naming. lys refuses an inclusion receipt
//! presented as a consistency one on the protected content type, before any
//! proof is examined; this gate requires an independently written verifier to
//! refuse it too, in both directions. If the discriminator were a lys-only
//! convention, that is where it would show.
//!
//! Gated with the format it tests: with `unstable-anchor` off there is no
//! `receipt` module.
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
use lys_core::receipt::{
    ConsistencyReceipt, sign_consistency_receipt, sign_receipt, verify_consistency_receipt_bytes,
};

/// Seed for the anchor identity. Fixed so the Go side can be handed the same
/// key material and produce the same signatures.
const ANCHOR_SEED: &[u8; 32] = b"lys-consistency-receipt-seed-001";

fn anchor_identity() -> (tempfile::TempDir, Ed25519Identity) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("anchor.key");
    std::fs::write(&path, ANCHOR_SEED).unwrap();
    let identity = Ed25519Identity::load(&path).unwrap();
    (dir, identity)
}

/// Leaf *i* of the shared fixture tree.
fn leaf(index: u64) -> Vec<u8> {
    format!("anchor-checkpoint-{index}").into_bytes()
}

fn to_hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    bytes.iter().fold(String::new(), |mut acc, byte| {
        let _ = write!(acc, "{byte:02x}");
        acc
    })
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

/// Issue a lys consistency receipt over the fixture log, returning it with the
/// older root it descends from and the newer root it proves.
fn issue(old_size: u64, new_size: u64, key: &Ed25519Identity) -> (ConsistencyReceipt, [u8; 32]) {
    let tree = tree_of(new_size);
    let proof = tree.prove_consistency(old_size, new_size).unwrap();
    let path: Vec<[u8; 32]> = proof
        .as_bytes()
        .chunks_exact(32)
        .map(|chunk| <[u8; 32]>::try_from(chunk).unwrap())
        .collect();
    let old_root = root_of(old_size);
    let receipt = sign_consistency_receipt(&old_root, old_size, new_size, &path, key).unwrap();
    (receipt, old_root)
}

/// The `consistency-verify` invocation for a `(old_size, new_size)` pair over
/// the fixture log.
fn verify_args(pubkey_hex: &str, old_size: u64, new_size: u64) -> Vec<String> {
    let mut args = vec![
        "consistency-verify".to_string(),
        pubkey_hex.to_string(),
        old_size.to_string(),
        new_size.to_string(),
    ];
    args.extend((0..new_size).map(|index| to_hex(&leaf(index))));
    args
}

#[test]
fn go_cose_consistency_receipt_conformance_round_trips() {
    let Some(go) = go_or_skip("go-cose consistency receipt conformance") else {
        return;
    };
    let workdir = tempfile::tempdir().unwrap();
    let gocache = workdir.path().join("gocache");
    let bin = workdir.path().join("cosetool");
    build_go_tool(&go, &gocache, &bin);

    let (_dir, key) = anchor_identity();
    let pubkey_hex = to_hex(&key.public_key_bytes());
    let seed_hex = to_hex(ANCHOR_SEED);

    // ---- Every size pair, both directions. Incomplete trees are where an
    // iterative walk and a recursion part company, so sampling would prove
    // little; the pairs are cheap, so all of them run.
    let mut cases = 0usize;
    let mut prepended = 0usize;
    for new_size in 2u64..=17 {
        for old_size in 1u64..new_size {
            let (receipt, _old_root) = issue(old_size, new_size, &key);
            let expected_new_root = root_of(new_size);

            // lys signs, go-cose verifies. The Go side computes the older root
            // from the leaves itself — it never reads it from the artifact —
            // and derives the newer one recursively, so a disagreement about
            // either surfaces as a signature failure.
            let (ok, stdout) = run_built_tool(
                &bin,
                &verify_args(&pubkey_hex, old_size, new_size),
                &receipt.to_cose_bytes(),
            );
            assert!(
                ok,
                "go-cose rejected the lys receipt for {old_size} -> {new_size}"
            );
            assert_eq!(
                String::from_utf8(stdout).unwrap(),
                format!("{} {old_size} {new_size}\n", to_hex(&expected_new_root)),
                "the Go derivation disagreed with lys for {old_size} -> {new_size}"
            );

            // Go signs, and the artifacts must be byte-identical rather than
            // merely mutually acceptable. If this ever fails, check the
            // encoders before the crypto.
            let mut sign_args = vec![
                "consistency-sign".to_string(),
                seed_hex.clone(),
                old_size.to_string(),
                new_size.to_string(),
            ];
            sign_args.extend((0..new_size).map(|index| to_hex(&leaf(index))));
            let (ok, go_receipt) = run_built_tool(&bin, &sign_args, &[]);
            assert!(
                ok,
                "go-cose consistency-sign failed for {old_size} -> {new_size}"
            );
            assert_eq!(
                go_receipt,
                receipt.to_cose_bytes(),
                "go-cose and lys consistency receipts differ for {old_size} -> {new_size}"
            );

            // And lys must accept the artifact Go built, against the older root
            // the caller supplies — which is the only shape of the call there
            // is.
            let (restored, new_root) = verify_consistency_receipt_bytes(
                &go_receipt,
                &key.public_key_bytes(),
                &root_of(old_size),
            )
            .unwrap_or_else(|err| {
                panic!("lys rejected the Go receipt for {old_size} -> {new_size}: {err}")
            });
            assert_eq!(restored.tree_size_1, old_size);
            assert_eq!(restored.tree_size_2, new_size);
            assert_eq!(new_root, expected_new_root);

            cases += 1;
            prepended += usize::from(old_size.is_power_of_two());
        }
    }
    // A loop that ran zero times would satisfy every assertion inside it, and a
    // sweep that missed the power-of-two sizes would leave RFC 6962 §2.1.4.2's
    // prepend branch — where the older root is the seed rather than a proof
    // node — unexercised in the envelope too.
    assert_eq!(cases, 136, "16*17/2 pairs with 1 <= old < new <= 17");
    assert_eq!(prepended, 54, "the power-of-two prepend branch");
}

#[test]
fn go_cose_refuses_what_lys_refuses() {
    let Some(go) = go_or_skip("go-cose consistency receipt negatives") else {
        return;
    };
    let workdir = tempfile::tempdir().unwrap();
    let gocache = workdir.path().join("gocache");
    let bin = workdir.path().join("cosetool");
    build_go_tool(&go, &gocache, &bin);

    let (_dir, key) = anchor_identity();
    let pubkey_hex = to_hex(&key.public_key_bytes());
    let (old_size, new_size) = (5u64, 13u64);

    // ---- Positive control, and it is not decoration. Every other assertion in
    // this test says the Go tool *refused* something, and a verifier that
    // refuses everything satisfies all of them. That is not hypothetical: it is
    // what happened when the Go content-type constant was drifted by one
    // character — the sweep above failed, and this test passed unchanged. So the
    // first thing asserted here is that an honest receipt is still accepted.
    let (honest, honest_old_root) = issue(old_size, new_size, &key);
    let (ok, _stdout) = run_built_tool(
        &bin,
        &verify_args(&pubkey_hex, old_size, new_size),
        &honest.to_cose_bytes(),
    );
    assert!(
        ok,
        "the control failed: go-cose refused an honest receipt, so no refusal below means anything"
    );
    assert!(
        verify_consistency_receipt_bytes(
            &honest.to_cose_bytes(),
            &key.public_key_bytes(),
            &honest_old_root
        )
        .is_ok(),
        "the control failed: lys refused its own honest receipt"
    );

    // ---- A tampered proof node. Go rejects it at the SUBPROOF comparison;
    // lys rejects it because the derived root is one the anchor never signed.
    let (mut tampered, old_root) = issue(old_size, new_size, &key);
    tampered.consistency_path[0][0] ^= 0x01;
    let (ok, _stdout) = run_built_tool(
        &bin,
        &verify_args(&pubkey_hex, old_size, new_size),
        &tampered.to_cose_bytes(),
    );
    assert!(!ok, "go-cose accepted a receipt with a tampered path");
    assert!(
        verify_consistency_receipt_bytes(
            &tampered.to_cose_bytes(),
            &key.public_key_bytes(),
            &old_root
        )
        .is_err(),
        "lys accepted a receipt with a tampered path"
    );

    // ---- A tampered signature. The path comparison and the derivation both
    // pass, so this is the only negative here that reaches go-cose's `Verify`
    // at all — without it the sweep's 136 successes would be the sole evidence
    // that the signature check is wired up, and a verifier that never rejects
    // is indistinguishable from one that never runs.
    let (mut forged, old_root) = issue(old_size, new_size, &key);
    forged.signature[0] ^= 0x01;
    let (ok, _stdout) = run_built_tool(
        &bin,
        &verify_args(&pubkey_hex, old_size, new_size),
        &forged.to_cose_bytes(),
    );
    assert!(!ok, "go-cose accepted a receipt with a forged signature");
    assert!(
        verify_consistency_receipt_bytes(
            &forged.to_cose_bytes(),
            &key.public_key_bytes(),
            &old_root
        )
        .is_err(),
        "lys accepted a receipt with a forged signature"
    );

    // ---- The re-label, both directions. This is the parity that matters: the
    // content type is what separates the two receipt kinds, and if that were a
    // lys-only convention an independently written verifier would happily open
    // one as the other.
    let (consistency, _old_root) = issue(old_size, new_size, &key);
    let mut args = vec![
        "receipt-verify".to_string(),
        pubkey_hex.clone(),
        "0".to_string(),
    ];
    args.extend((0..new_size).map(|index| to_hex(&leaf(index))));
    let (ok, _stdout) = run_built_tool(&bin, &args, &consistency.to_cose_bytes());
    assert!(
        !ok,
        "go-cose opened a consistency receipt as an inclusion receipt"
    );

    let inclusion_tree = tree_of(new_size);
    let inclusion_proof = inclusion_tree.prove_inclusion(0).unwrap();
    let inclusion_path: Vec<[u8; 32]> = inclusion_proof
        .as_bytes()
        .chunks_exact(32)
        .map(|chunk| <[u8; 32]>::try_from(chunk).unwrap())
        .collect();
    let inclusion = sign_receipt(&leaf(0), 0, new_size, &inclusion_path, &key).unwrap();
    let (ok, _stdout) = run_built_tool(
        &bin,
        &verify_args(&pubkey_hex, old_size, new_size),
        &inclusion.to_cose_bytes(),
    );
    assert!(
        !ok,
        "go-cose opened an inclusion receipt as a consistency receipt"
    );

    // ---- A receipt presented against a log the verifier does not hold. The
    // older root the Go side computes is its own, so a receipt from elsewhere
    // has nothing to attach to.
    let (receipt, _old_root) = issue(old_size, new_size, &key);
    let mut args = vec![
        "consistency-verify".to_string(),
        pubkey_hex,
        old_size.to_string(),
        new_size.to_string(),
    ];
    args.extend((0..new_size).map(|index| to_hex(&leaf(index + 100))));
    let (ok, _stdout) = run_built_tool(&bin, &args, &receipt.to_cose_bytes());
    assert!(!ok, "go-cose accepted a receipt against the wrong log");
}
