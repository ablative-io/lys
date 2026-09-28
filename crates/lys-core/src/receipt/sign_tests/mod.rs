#![cfg(test)]
//! Adversarial and end-to-end tests for receipt issuance and verification.
//!
//! Every proof here comes from a **real** `AppendOnlyTree`, never a synthetic
//! path, so the tests exercise the same bytes an anchor would issue. The attack
//! cases are constructed rather than sampled, per the crypto-change standard:
//! forgery, replay, misattribution, algorithm substitution, cross-protocol
//! confusion and malleability each get a case that would pass if the
//! corresponding check were removed.

use super::*;
use crate::attestation;
use crate::error::TrustError;
use crate::merkle::tree::{AppendOnlyTree, RawLeaf};

mod families;
mod issuance;
mod uniform_failure;
mod verification;

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
