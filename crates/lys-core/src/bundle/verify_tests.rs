#![cfg(test)]
//! Bundle verification tests, built from real logs, real anchors and real
//! receipts throughout — no synthetic artifacts.
//!
//! The tests that matter most follow the shape that made the certificate↔
//! attestation join convincing: **assert each half is valid on its own first**,
//! so when the combined check refuses, the refusal is demonstrably the join and
//! not a broken fixture. A chain test that only shows "invalid input rejected"
//! proves nothing about whether the links are checked at all.

use super::*;
use crate::bundle::artifact::BundleLink;
use crate::checkpoint::{CheckpointBody, NoteVerifierKey, sign_note, verify_checkpoint};
use crate::merkle::tree::{AppendOnlyTree, RawLeaf};
use crate::receipt::{sign_receipt, verify_receipt_bytes};
use crate::tlog::{build_inclusion_artifact, verify_inclusion_artifact};
use crate::{Ed25519Identity, TrustError};

#[path = "verify_tests/chain.rs"]
mod chain;
#[path = "verify_tests/refusals.rs"]
mod refusals;
#[path = "verify_tests/wire.rs"]
mod wire;

// ------------------------------------------------------------------ fixtures

/// A log or anchor: an origin, a key, and a tree.
struct Party {
    origin: String,
    identity: Ed25519Identity,
    tree: AppendOnlyTree<RawLeaf>,
    temp_dir: tempfile::TempDir,
}

impl Party {
    fn new(origin: &str, seed: &[u8; 32]) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("key");
        std::fs::write(&path, seed).unwrap();
        Self {
            origin: origin.to_string(),
            identity: Ed25519Identity::load(&path).unwrap(),
            tree: AppendOnlyTree::<RawLeaf>::new(),
            temp_dir: dir,
        }
    }

    /// Removes the fixture's temporary directory, failing on error.
    fn close(self) -> std::io::Result<()> {
        self.temp_dir.close()
    }

    fn verifier(&self) -> NoteVerifierKey {
        NoteVerifierKey::new(&self.origin, self.identity.public_key_bytes()).unwrap()
    }

    /// This party's own signed checkpoint at its current size.
    fn checkpoint(&self) -> String {
        let body = CheckpointBody::from_root(&self.origin, &self.tree.root()).unwrap();
        sign_note(&body.encode(), &self.origin, &self.identity).unwrap()
    }

    /// A receipt from this party proving `leaf` at `index`.
    fn receipt_over(&self, leaf: &[u8], index: u64) -> Vec<u8> {
        let size = self.tree.root().to_parts().1;
        let proof = self.tree.prove_inclusion(index).unwrap();
        let path: Vec<[u8; 32]> = proof
            .as_bytes()
            .chunks_exact(32)
            .map(|c| <[u8; 32]>::try_from(c).unwrap())
            .collect();
        sign_receipt(leaf, index, size, &path, &self.identity)
            .unwrap()
            .to_cose_bytes()
    }
}

/// An anchor that has notarized `notarized` (appended after a genesis leaf, so
/// its tree size is never 1 — the receipt format cannot express size 1).
fn anchor_over(origin: &str, seed: &[u8; 32], notarized: &[u8]) -> Party {
    let mut anchor = Party::new(origin, seed);
    anchor.tree.append_raw(b"genesis");
    anchor.tree.append_raw(notarized);
    anchor
}

const CHILD_LEAF: &[u8] = b"entry-1";

/// A child log holding three entries, with the leaf at index 1 proven.
fn child_log() -> Party {
    let mut child = Party::new("child.example", b"lys-bundle-test-child-seed-0001a");
    for leaf in [b"entry-0".as_slice(), CHILD_LEAF, b"entry-2".as_slice()] {
        child.tree.append_raw(leaf);
    }
    child
}

/// A complete one-link scenario: child log, one anchor over its checkpoint.
struct OneLink {
    child: Party,
    anchor: Party,
    bundle: VerificationBundle,
}

impl OneLink {
    /// Removes both parties' temporary directories, failing on error.
    fn close(self) -> std::io::Result<()> {
        self.child.close()?;
        self.anchor.close()
    }
}

fn one_link() -> OneLink {
    let child = child_log();
    let artifact =
        build_inclusion_artifact(&child.tree, CHILD_LEAF, &child.origin, &child.identity, 1)
            .unwrap();
    let child_note = artifact.checkpoint.clone();

    let anchor = anchor_over(
        "anchor-a.example",
        b"lys-bundle-test-anchor-a-seed-01",
        child_note.as_bytes(),
    );
    let receipt = anchor.receipt_over(child_note.as_bytes(), 1);

    let bundle = VerificationBundle::new(
        CHILD_LEAF,
        artifact,
        vec![BundleLink::new(&child_note, &receipt)],
    );
    OneLink {
        child,
        anchor,
        bundle,
    }
}

/// A two-link scenario: anchor B notarizes anchor A's own checkpoint.
struct TwoLink {
    child: Party,
    anchor_a: Party,
    anchor_b: Party,
    bundle: VerificationBundle,
}

impl TwoLink {
    /// Removes all three parties' temporary directories, failing on error.
    fn close(self) -> std::io::Result<()> {
        self.child.close()?;
        self.anchor_a.close()?;
        self.anchor_b.close()
    }
}

fn two_link() -> TwoLink {
    let OneLink {
        child,
        anchor: anchor_a,
        bundle: base,
    } = one_link();

    let a_note = anchor_a.checkpoint();
    let anchor_b = anchor_over(
        "anchor-b.example",
        b"lys-bundle-test-anchor-b-seed-01",
        a_note.as_bytes(),
    );
    let receipt_b = anchor_b.receipt_over(a_note.as_bytes(), 1);

    let mut links = base.links.clone();
    links.push(BundleLink::new(&a_note, &receipt_b));
    let bundle = VerificationBundle::new(CHILD_LEAF, base.inclusion_proof, links);

    TwoLink {
        child,
        anchor_a,
        anchor_b,
        bundle,
    }
}
