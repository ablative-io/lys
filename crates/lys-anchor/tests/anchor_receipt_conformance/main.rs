#![cfg(test)]
//! The receipts an anchor issues, judged by `veraison/go-cose` and by RFC
//! 6962's *recursive* `PATH`, transcribed in Go.
//!
//! # The hole this exists for, stated before anything is asserted
//!
//! `sign_receipt` derives the Merkle root **from the inclusion path it was
//! handed** and signs that. `verify_receipt` derives the root **from the
//! receipt's own path** and checks the signature against it. The two share one
//! reconstruction, so an anchor that chunked `Proof::as_bytes()` wrongly —
//! reversed, offset, a node dropped — signs a root no tree ever had, and its
//! own verifier reconstructs the same wrong root and agrees.
//!
//! **Every assertion of the form `verify_receipt(..).is_ok()` is blind to that
//! by construction**, and so is every round trip through `to_cose_bytes` and
//! back: with one party, the agreement is structural. That is not a hypothesis
//! here — [`go_refuses_the_mis_chunked_receipts_lys_accepts`] builds two such
//! receipts and asserts, in as many words, that lys accepts both.
//!
//! What closes it is a reader that derives the path **from the leaves, not
//! from the artifact**. `cose-conformance`'s `receipt-verify` does exactly
//! that: it computes `PATH(m, D[n])` from RFC 6962 §2.1.1's recursive
//! definition over the leaves *this test supplies on the command line*, and
//! compares it against the artifact's path element by element, **before** it
//! reaches the signature. A wrongly-chunked path is then a path disagreement
//! rather than a receipt that verifies and means something else.
//!
//! # What this gate is NOT the only guard for, measured rather than assumed
//!
//! The tempting claim — *only the Go gate catches a mis-chunked path* — was
//! tested and is **false**, and it is written down here so nobody re-derives it
//! from the paragraph above. Three drifts were injected into a throwaway copy of
//! the tree, each proven to have landed by diffing against the pristine file:
//!
//! | injection | in-crate result | this gate |
//! |---|---|---|
//! | `proof_nodes` reverses the node order | **3 in-crate tests fail** | both tests fail |
//! | `proof_nodes` flips a bit in node 0 | **4 in-crate tests fail** | both tests fail |
//! | `merkle::leaf`'s RFC 6962 `0x00` tag becomes `0x02` | **4 in-crate tests fail** | both tests fail |
//!
//! `submit_tests`'s `..._reconstructs_the_root_the_checkpoint_publishes` is why:
//! it holds the receipt's reconstructed root against `publish_checkpoint`'s,
//! which reaches the root through `Log::tree()` and never touches
//! `proof_nodes`. That is a real second party — but it is a **code-path** one,
//! both sides being `lys-core`'s Merkle, so a change that moved the tree and the
//! receipt together would satisfy it. The third injection was chosen to be
//! exactly that shape, and the frozen leaf-hash literal in `submit_tests` caught
//! it independently.
//!
//! So the honest statement of what this file adds is narrower than "it is the
//! only guard", and narrower is the point: it supplies the **algorithm axis**
//! for a rule the crate otherwise pins only on its own code paths, and it is
//! the only place the bytes `submit` actually emits are compared against an
//! artifact built by another implementation.
//!
//! # Which axis of independence this is, and which it is not
//!
//! **Algorithm, language and toolchain.** A different language, a different
//! author, a different derivation — recursive descent on the largest power of
//! two, against lys's iterative upward walk — and a Go tool that predates
//! `lys-anchor` and has never heard of it. It was not written to agree with
//! this crate; it was written from the RFC.
//!
//! **Not platform, and not custody.** One machine, one Go toolchain, one
//! vendored dependency resolution. Two implementations agreeing here says
//! nothing about a third environment.
//!
//! # What the leaves are keyed on
//!
//! Every Go invocation is handed the leaf bytes **this test submitted** and the
//! index **this test counted**, never a value read back off the anchor or out
//! of the artifact. A check keyed on what the producer reported would be the
//! producer agreeing with itself through a subprocess.
//!
//! # Path length is why the sweep is not a sweep of small trees
//!
//! A two-leaf tree's inclusion path is **one node**, and a one-node path is
//! invariant under reordering — so a sweep of small trees would leave the
//! ordering rule pinned by nothing, and a reversal injection would come back
//! green in a way that reads exactly like "the gate does not catch it". The
//! sweep therefore counts the cases whose path has two or more nodes and
//! asserts that count against a literal.
//!
//! Note also what is *recognition* rather than enforcement: asserting
//! `inclusion_path.len()` is right proves nothing about order, since a reversed
//! path has the same length. Only Go's element-wise comparison enforces it, and
//! the length counters below exist solely to prove the sweep reached the shapes
//! where that comparison can discriminate.
//!
//! # Byte-identity, not mutual acceptance
//!
//! Ed25519 is deterministic and both sides emit RFC 8949 §4.2 core-deterministic
//! CBOR, so for one seed, one index and one set of leaves there is exactly one
//! correct artifact. Every sweep case therefore has Go *build* the receipt and
//! compares byte for byte. An acceptance can be satisfied by a verifier that is
//! too permissive; byte-identity cannot — and it only holds if the two sides
//! first agreed on the detached root, the path encoding and every header pin at
//! once.
//!
//! # Availability
//!
//! Gated with the format it tests: with `unstable-anchor` off there is no
//! `submit`, no `SubmissionOutcome` and no receipt, and this file compiles to
//! nothing. It runs under `--all-features`.
//!
//! # Environment contract
//!
//! See [`harness`] — vendored, network-free (`GOPROXY=off`), and a hard failure
//! rather than a skip when `LYS_REQUIRE_GO` is set.

#![cfg(feature = "unstable-anchor")]

#[path = "../harness/mod.rs"]
mod harness;

use std::path::Path;

use harness::{GoScaffold, build_go_tool, go_or_skip, run_built_tool};
use lys_anchor::{
    AcceptAll, Anchor, AnchorConfig, FileSigner, InProcessSigner, Signer, Submission,
    SubmitterContext,
};
use lys_core::merkle::tree::{AppendOnlyTree, RawLeaf};
use lys_core::receipt::{sign_receipt, verify_receipt};
use lys_log_store::FileLeafStore;
use tempfile::TempDir;

mod inclusion_path;
mod mis_chunked;

/// The origin the store is created under. A receipt carries no origin — see
/// `wire::submission` — so this is here to create the log, not to be checked.
const ORIGIN: &str = "example.com/lys/anchor-receipt-gate";

/// Leaf 0, written by `Anchor::create` and never afterwards.
const GENESIS: &[u8] = b"lys-anchor receipt-gate genesis fixture";

/// The anchor's seed. Fixed because byte-identity against a Go-built artifact
/// is only checkable if both sides sign under the same key.
const ANCHOR_SEED: &[u8; 32] = b"lys-anchor-receipt-gate-seed-01!";

/// How large the fixture log grows. Chosen so the sweep spans incomplete trees
/// — where an iterative walk and a recursion part company — and reaches paths
/// of four nodes.
const LOG_SIZE: u64 = 12;

/// The index the negative cases are built at, and the size they are built
/// against. Picked so the honest path has several nodes and reversing it is a
/// real permutation rather than a no-op; the control below asserts that rather
/// than trusting it.
const NEGATIVE_INDEX: u64 = 3;

/// Leaf `index` of the fixture log, **as this test supplied it** — genesis at
/// 0, and the statement this test submitted at every other index. Nothing here
/// reads the anchor back.
fn leaf(index: u64) -> Vec<u8> {
    if index == 0 {
        GENESIS.to_vec()
    } else {
        format!("anchor-receipt-gate statement {index}").into_bytes()
    }
}

fn to_hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    bytes.iter().fold(String::new(), |mut acc, byte| {
        write!(acc, "{byte:02x}").expect("writing to a String cannot fail");
        acc
    })
}

/// The RFC 6962 root of the first `size` fixture leaves, rebuilt from the bytes
/// this test submitted.
///
/// This is a **code-path** cross-check, not an algorithm-independent one — both
/// this and the anchor are `lys-core`'s Merkle. It is here so the Go tool's
/// printed root has something to be compared against; the independent judgement
/// of that root is Go's own recursive `MTH`, which the signature check enforces.
fn root_of(size: u64) -> [u8; 32] {
    let mut tree = AppendOnlyTree::<RawLeaf>::new();
    for index in 0..size {
        tree.append_raw(&leaf(index));
    }
    let (root, _) = tree.root().to_parts();
    root
}

fn signer(dir: &Path) -> FileSigner {
    let path = dir.join("anchor.key");
    std::fs::write(&path, ANCHOR_SEED).unwrap();
    FileSigner::load(&path).unwrap()
}

/// One statement offered to the anchor.
///
/// The anchor's admission surface is not what this gate measures, so the
/// submission carries the bytes and nothing else; a gate that also exercised a
/// policy would be two rules on one case.
fn submission(statement: &[u8]) -> Submission<'_> {
    Submission { statement }
}

/// `receipt-verify <pubkey-hex> <leaf-index> <leaf-hex>…` over the first
/// `tree_size` fixture leaves.
///
/// `leaf_index` and `tree_size` are required positionals with no sentinel and
/// no `Option` meaning "whatever the artifact says": a helper that could be
/// told to infer either from the receipt would let a caller switch off the only
/// comparison that closes the hole.
fn verify_args(pubkey_hex: &str, leaf_index: u64, tree_size: u64) -> Vec<String> {
    assert!(tree_size >= 2, "a receipt needs at least two leaves");
    assert!(leaf_index < tree_size, "leaf index outside the tree");
    let mut args = vec![
        "receipt-verify".to_string(),
        pubkey_hex.to_string(),
        leaf_index.to_string(),
    ];
    args.extend((0..tree_size).map(|index| to_hex(&leaf(index))));
    args
}

/// `receipt-sign <seed-hex> <leaf-index> <leaf-hex>…`, same shape, same rules.
fn sign_args(seed_hex: &str, leaf_index: u64, tree_size: u64) -> Vec<String> {
    assert!(tree_size >= 2, "a receipt needs at least two leaves");
    assert!(leaf_index < tree_size, "leaf index outside the tree");
    let mut args = vec![
        "receipt-sign".to_string(),
        seed_hex.to_string(),
        leaf_index.to_string(),
    ];
    args.extend((0..tree_size).map(|index| to_hex(&leaf(index))));
    args
}

/// Builds the vendored `cose-conformance` tool into a throwaway directory,
/// returning it with the directories that must outlive it.
fn cose_tool(go: &Path) -> (TempDir, TempDir, std::path::PathBuf) {
    let gocache_dir = TempDir::new().unwrap();
    let bin_dir = TempDir::new().unwrap();
    let bin = bin_dir.path().join("cosetool");
    build_go_tool(
        go,
        GoScaffold::Cose,
        &gocache_dir.path().join("gocache"),
        &bin,
    );
    (gocache_dir, bin_dir, bin)
}
