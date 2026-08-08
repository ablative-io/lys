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
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

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
        let _ = write!(acc, "{byte:02x}");
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
    let (root, _size) = tree.root().to_parts();
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

#[test]
fn go_cose_derives_the_inclusion_path_the_anchor_emitted_node_for_node() {
    let Some(go) = go_or_skip("lys-anchor receipt conformance") else {
        return;
    };
    let (_gocache_dir, _bin_dir, bin) = cose_tool(&go);

    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    let store = FileLeafStore::create(dir, ORIGIN).unwrap();
    let mut anchor = Anchor::create(
        store,
        GENESIS,
        signer(dir),
        // The admission surface is not what this gate measures: a policy that
        // could refuse would put a second rule on every case.
        AcceptAll,
        AnchorConfig::unconfigured(),
    )
    .unwrap();
    let pubkey_hex = to_hex(&anchor.signer().public_key());
    let seed_hex = to_hex(ANCHOR_SEED);

    let mut submissions = 0u64;
    let mut cases = 0usize;
    let mut multi_node = 0usize;

    for tree_size in 2..=LOG_SIZE {
        // ---- What `submit` reports, against what the artifact it returned
        // actually proves. Go is handed this test's own counter as the index,
        // never `outcome.leaf_index` — a check keyed on the value under test is
        // no check at all.
        let expected_index = tree_size - 1;
        let statement = leaf(expected_index);
        let outcome = anchor
            .submit(submission(&statement), SubmitterContext::Unidentified)
            .unwrap();
        submissions += 1;

        let (ok, stdout) = run_built_tool(
            &bin,
            &verify_args(&pubkey_hex, expected_index, tree_size),
            &outcome.receipt.to_cose_bytes(),
        );
        assert!(
            ok,
            "go-cose rejected the receipt `submit` returned for index \
             {expected_index} of {tree_size}"
        );
        // Not `ok` alone: a tool that exited 0 without doing anything satisfies
        // that. The root is Go's own recursive MTH over the supplied leaves.
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            format!(
                "{} {tree_size} {expected_index}\n",
                to_hex(&root_of(tree_size))
            ),
            "the Go derivation disagreed with the anchor at index \
             {expected_index} of {tree_size}"
        );
        assert_eq!(
            outcome.leaf_index, expected_index,
            "`submit` reported an index the emitted receipt does not prove"
        );
        assert_eq!(
            outcome.tree_size, tree_size,
            "`submit` reported a size the emitted receipt does not prove"
        );

        // ---- Every index at this size. Incomplete trees are where a recursion
        // and an iterative walk part company, so the whole triangle runs rather
        // than a sample.
        for leaf_index in 0..tree_size {
            let receipt = anchor.receipt_for(leaf_index).unwrap();

            let (ok, stdout) = run_built_tool(
                &bin,
                &verify_args(&pubkey_hex, leaf_index, tree_size),
                &receipt.to_cose_bytes(),
            );
            assert!(
                ok,
                "go-cose rejected the anchor's receipt for index {leaf_index} \
                 of {tree_size}"
            );
            assert_eq!(
                String::from_utf8(stdout).unwrap(),
                format!("{} {tree_size} {leaf_index}\n", to_hex(&root_of(tree_size))),
                "the Go derivation disagreed with the anchor for index \
                 {leaf_index} of {tree_size}"
            );

            // Go builds the same artifact from the same seed, index and leaves,
            // and the two must be the same bytes.
            let (ok, go_receipt) =
                run_built_tool(&bin, &sign_args(&seed_hex, leaf_index, tree_size), &[]);
            assert!(
                ok,
                "go-cose receipt-sign failed for index {leaf_index} of {tree_size}"
            );
            assert_eq!(
                go_receipt,
                receipt.to_cose_bytes(),
                "go-cose and the anchor built different receipts for index \
                 {leaf_index} of {tree_size} — check the encoders before the crypto"
            );

            if receipt.inclusion_path.len() >= 2 {
                multi_node += 1;
            }
            cases += 1;
        }
    }

    // Count what fired. A loop that ran zero times satisfies every assertion
    // inside it, and a Go gate that never spawned looks exactly like one that
    // passed.
    assert_eq!(submissions, LOG_SIZE - 1, "one submission per size above 1");
    assert_eq!(cases, 77, "sum of n for n in 2..=12");

    // And the sweep reached the shapes where element-wise comparison can
    // discriminate at all: a one-node path is invariant under reordering, so
    // without this the ordering rule would be pinned by nothing.
    assert_eq!(
        multi_node, 72,
        "cases whose inclusion path has two or more nodes"
    );
}

#[test]
fn go_refuses_the_mis_chunked_receipts_lys_accepts() {
    let Some(go) = go_or_skip("lys-anchor receipt negatives") else {
        return;
    };
    let (_gocache_dir, _bin_dir, bin) = cose_tool(&go);

    // Grown here rather than in a helper: a helper would have to name
    // `Anchor<..>`'s type parameters, and this file has no business pinning
    // those.
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    let store = FileLeafStore::create(dir, ORIGIN).unwrap();
    let mut anchor = Anchor::create(
        store,
        GENESIS,
        signer(dir),
        // The admission surface is not what this gate measures: a policy that
        // could refuse would put a second rule on every case.
        AcceptAll,
        AnchorConfig::unconfigured(),
    )
    .unwrap();
    for leaf_index in 1..LOG_SIZE {
        let statement = leaf(leaf_index);
        let outcome = anchor
            .submit(submission(&statement), SubmitterContext::Unidentified)
            .unwrap();
        assert_eq!(
            outcome.leaf_index, leaf_index,
            "submit appended out of order"
        );
    }
    assert_eq!(anchor.tree_size(), LOG_SIZE);

    let anchor_key = anchor.signer().public_key();
    let pubkey_hex = to_hex(&anchor_key);
    let (index, size) = (NEGATIVE_INDEX, LOG_SIZE);
    let statement = leaf(index);

    // ---- Positive control, and it is the FIRST assertion on purpose. Every
    // case below asserts that the Go tool *refused* something, and a verifier
    // that refuses everything satisfies all of them at once. That is not
    // hypothetical in this repository: drifting a Go content-type constant by
    // one character left a conformance test made of refusals entirely green.
    let honest = anchor.receipt_for(index).unwrap();
    let (ok, stdout) = run_built_tool(
        &bin,
        &verify_args(&pubkey_hex, index, size),
        &honest.to_cose_bytes(),
    );
    assert!(
        ok,
        "the control failed: go-cose refused an honest receipt, so no refusal \
         below means anything"
    );
    assert_eq!(
        String::from_utf8(stdout).unwrap(),
        format!("{} {size} {index}\n", to_hex(&root_of(size))),
        "the control failed: go-cose exited 0 without agreeing about the root"
    );
    // And the permutation below is a real one. A one-node path reversed is the
    // same path, and the refusals would then be measuring nothing.
    assert!(
        honest.inclusion_path.len() >= 2,
        "the control failed: a path of {} node(s) cannot be reordered",
        honest.inclusion_path.len()
    );

    let identity = anchor.signer().identity();

    // ---- The hole itself: an anchor that chunked its proof in the wrong ORDER.
    // The receipt is signed over the root that wrong path reconstructs to, so
    // the artifact is internally consistent and describes a tree that does not
    // exist.
    let mut reversed = honest.inclusion_path.clone();
    reversed.reverse();
    assert_ne!(
        reversed, honest.inclusion_path,
        "the reversal must change the path"
    );
    let mis_ordered = sign_receipt(&statement, index, size, &reversed, identity).unwrap();
    assert_ne!(
        mis_ordered.signature, honest.signature,
        "a different path must reconstruct to a different signed root"
    );
    assert!(
        verify_receipt(&mis_ordered, &statement, &anchor_key).is_ok(),
        "lys was expected to ACCEPT its own mis-ordered receipt — it reconstructs \
         the root from the receipt's own path, so it has nothing to disagree \
         with. If this ever fails, lys grew a second party and this gate's \
         premise needs rewriting, not deleting"
    );
    let (ok, _stdout) = run_built_tool(
        &bin,
        &verify_args(&pubkey_hex, index, size),
        &mis_ordered.to_cose_bytes(),
    );
    assert!(
        !ok,
        "go-cose accepted a receipt whose inclusion path is in the wrong order"
    );

    // ---- The same hole, one flipped bit rather than a permutation. Separated
    // from the reordering because an injection cannot tell two rules apart when
    // one case guards both: a chunker can get the order right and the offset
    // wrong, and vice versa.
    let mut flipped = honest.inclusion_path.clone();
    flipped[0][0] ^= 0x01;
    let mis_hashed = sign_receipt(&statement, index, size, &flipped, identity).unwrap();
    assert!(
        verify_receipt(&mis_hashed, &statement, &anchor_key).is_ok(),
        "lys was expected to ACCEPT its own receipt over a corrupted path node"
    );
    let (ok, _stdout) = run_built_tool(
        &bin,
        &verify_args(&pubkey_hex, index, size),
        &mis_hashed.to_cose_bytes(),
    );
    assert!(
        !ok,
        "go-cose accepted a receipt carrying a corrupted inclusion-path node"
    );

    // ---- The mirror image, so the two acceptances above are attributable to
    // the anchor having signed the wrong root rather than to `verify_receipt`
    // accepting anything. Here the path is altered *after* signing, and both
    // implementations must refuse.
    let mut tampered = honest.clone();
    tampered.inclusion_path[0][0] ^= 0x01;
    assert!(
        verify_receipt(&tampered, &statement, &anchor_key).is_err(),
        "lys accepted a receipt whose path was altered after signing"
    );
    let (ok, _stdout) = run_built_tool(
        &bin,
        &verify_args(&pubkey_hex, index, size),
        &tampered.to_cose_bytes(),
    );
    assert!(
        !ok,
        "go-cose accepted a receipt whose path was altered after signing"
    );

    // ---- And the leaves Go is handed are load-bearing: the same honest
    // receipt, offered against a log this verifier does not hold. Without this,
    // every acceptance above could be explained by a tool that ignores its leaf
    // arguments entirely.
    let mut wrong_log = vec!["receipt-verify".to_string(), pubkey_hex, index.to_string()];
    wrong_log.extend((0..size).map(|i| to_hex(&leaf(i + 100))));
    let (ok, _stdout) = run_built_tool(&bin, &wrong_log, &honest.to_cose_bytes());
    assert!(
        !ok,
        "go-cose accepted a receipt against leaves it does not describe"
    );
}
