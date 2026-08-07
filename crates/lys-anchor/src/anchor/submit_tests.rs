#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Gates on [`Anchor::submit`] and [`Anchor::receipt_for`].
//!
//! # The trap this file is shaped around
//!
//! `verify_receipt` reconstructs the root **from the receipt's own inclusion
//! path** and checks the anchor's signature against that reconstruction. The
//! anchor derived and signed the root from that same path. So a receipt whose
//! path this crate chunked *wrongly* is still internally self-consistent, and
//! `verify_receipt` accepts it: a test that only asserts `is_ok()` proves
//! nothing whatever about the path. Anything this crate encodes and decodes
//! with its own pair of functions is invisible to its own suite.
//!
//! Every receipt assertion below is therefore anchored outside that loop.
//!
//! # Where the second party comes from
//!
//! - **The anchor's published checkpoint.** Its root comes from
//!   `AppendOnlyTree::root` — ct-merkle's own accumulator — and travels
//!   through base64 into a signed note that `lys-core`'s `verify_checkpoint`
//!   parses back out. The receipt's root comes from walking a chunked
//!   inclusion path. Two different derivations of the same 32 bytes, and
//!   [`the_receipt_reconstructs_the_root_the_checkpoint_publishes`] holds them
//!   against each other.
//! - **A root and two leaf hashes computed outside Rust.** [`GOLDEN_ROOT_2`],
//!   [`GOLDEN_GENESIS_LEAF_HASH`] and [`GOLDEN_STATEMENT_LEAF_HASH`] were
//!   produced by `openssl dgst -sha256` shell pipelines and independently
//!   reproduced with Python's `hashlib`, which agreed. **That agreement between
//!   two implementations is the whole of the evidence** — the literals are
//!   written out here, so no change to this workspace can move them.
//!
//!   ⚠️ **These constants were first justified by a positive control that
//!   cannot fail, and the claim is retracted here rather than quietly
//!   dropped.** The control was: hash the empty fixture with and without the
//!   `0x00` prefix, expect `e3b0c442…b855` and `6e340b9c…fa01d`. Both values
//!   are indeed correct, and the check is nevertheless **inert**, because on an
//!   *empty* fixture a working `cat` and a failed `cat` contribute the same
//!   thing — nothing. Measured: with the file replaced by a non-existent path,
//!   both legs return byte-identical digests to the working case, and the
//!   pipeline's exit status stays `0` because a pipeline reports its *last*
//!   command. A control fixture must be **non-empty**, or it cannot observe
//!   whether the file was read at all. Restated as house rule: *a control whose
//!   expected value is what a broken instrument returns anyway is not a
//!   control* — and this repo's own law that a check which never fires is
//!   indistinguishable from one that passed.
//! - **`sha2` driven by the test itself**, for the leaf hash — never
//!   `raw_leaf_hash`, which is the helper the implementation used.
//! - **The disk.** Leaf bytes are read back through a `Log` handle opened
//!   fresh over the same directory, which never saw the submission happen.
//!
//! The independence axis for all of these is *implementation*, not platform:
//! one machine, one toolchain, one dependency resolution. The cross-language
//! claim belongs to the Go conformance gate and is not made here.

use std::path::Path;

use lys_core::checkpoint::{NoteVerifierKey, verify_checkpoint};
use lys_core::receipt::verify_receipt;
use lys_log_store::{FileLeafStore, Log};
use sha2::{Digest, Sha256};
use tempfile::TempDir;

use crate::AnchorConfig;
use crate::keys::{FileSigner, Signer};

use super::*;

/// The origin this test supplies to the store. Verifiers are built from *this*
/// literal, never from what the anchor reports back.
const ORIGIN: &str = "example.com/lys/anchor-submit-test";

/// The genesis bytes for every anchor built here.
const GENESIS: &[u8] = b"lys-anchor increment 4 genesis fixture";

/// The statement submitted in the single-submission tests.
const STATEMENT: &[u8] = b"lys-anchor increment 4 statement fixture";

/// `SHA-256(0x00 ‖ GENESIS)` — RFC 6962's leaf hash, computed outside Rust.
const GOLDEN_GENESIS_LEAF_HASH: &str =
    "a5dabd900c94df52610e226a28e883f7e932d4f71e27e2bb4737cb9f2a0f7d1d";

/// `SHA-256(0x00 ‖ STATEMENT)`, computed outside Rust.
const GOLDEN_STATEMENT_LEAF_HASH: &str =
    "4a68beab3afee2c3b01d3b7c277259e12240cc888e5da1773c460f48f079bf5c";

/// `SHA-256(0x01 ‖ GOLDEN_GENESIS_LEAF_HASH ‖ GOLDEN_STATEMENT_LEAF_HASH)` —
/// the RFC 6962 root of the two-leaf tree an anchor holds after exactly one
/// submission. Computed outside Rust from the two literals above.
const GOLDEN_ROOT_2: &str = "6ecd815c5246ecb3f8ab9a7abbc5e1ba2bba773886e7db8dceca7c0739842861";

/// `lys-core`'s conformance fixture seed, reused so key material in tests is
/// deterministic and never generated.
const FIXTURE_SEED: &[u8; 32] = b"lys-go-conformance-test-seed-01!";

/// Lower-case hex, so a 32-byte value can be compared with a literal computed
/// by a tool that is not this workspace.
fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(char::from(DIGITS[usize::from(byte >> 4)]));
        out.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    out
}

/// Loads a signer over the fixture seed, writing the key file into `dir`.
fn signer(dir: &Path) -> FileSigner {
    let path = dir.join("anchor.key");
    std::fs::write(&path, FIXTURE_SEED).unwrap();
    FileSigner::load(&path).unwrap()
}

/// Creates a store at `dir` under [`ORIGIN`] and an anchor over it.
fn create_anchor(dir: &Path) -> Anchor<FileLeafStore, FileSigner> {
    let store = FileLeafStore::create(dir, ORIGIN).unwrap();
    Anchor::create(store, GENESIS, signer(dir), AnchorConfig::unconfigured()).unwrap()
}

/// The verifier a third party would build: the origin they were told, and the
/// public key they were given.
fn verifier(anchor: &Anchor<FileLeafStore, FileSigner>) -> NoteVerifierKey {
    NoteVerifierKey::new(ORIGIN, anchor.signer().public_key()).unwrap()
}

/// Reads leaf `index` back off the disk through a handle that never saw the
/// submission.
fn leaf_from_disk(dir: &Path, index: u64) -> Vec<u8> {
    let log = Log::open(FileLeafStore::open(dir).unwrap()).unwrap();
    log.leaf_bytes(index)
        .expect("the leaf must be on disk")
        .to_vec()
}

#[test]
fn the_leaf_hash_is_sha256_of_the_tag_byte_and_the_statement() {
    let tmp = TempDir::new().unwrap();
    let mut anchor = create_anchor(tmp.path());

    let outcome = anchor
        .submit(Submission {
            statement: STATEMENT,
        })
        .unwrap();

    // Driven by the test, not by `raw_leaf_hash` — the helper the append path
    // used. A change to the domain-separation byte, or to what gets hashed,
    // has to be wrong in two places to survive this.
    let mut hasher = Sha256::new();
    hasher.update([0x00_u8]);
    hasher.update(STATEMENT);
    let expected: [u8; 32] = hasher.finalize().into();
    assert_eq!(outcome.leaf_hash, expected);

    // And against a value no Rust in this workspace produced.
    assert_eq!(hex(&outcome.leaf_hash), GOLDEN_STATEMENT_LEAF_HASH);

    // Genesis is leaf 0, so the submission is leaf 1 and the tree is size 2.
    assert_eq!(outcome.leaf_index, 1);
    assert_eq!(outcome.tree_size, 2);
    // The outcome and the artifact inside it must not be able to disagree.
    assert_eq!(outcome.tree_size, outcome.receipt.tree_size);
    assert_eq!(outcome.leaf_index, outcome.receipt.leaf_index);
}

#[test]
fn the_statement_is_stored_verbatim_and_the_anchor_reads_nothing_into_it() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    let mut anchor = create_anchor(dir);

    // Bytes that are not valid UTF-8, contain a NUL, and would be mangled by
    // any canonicalization, trimming or text handling on the way to storage.
    let statement: &[u8] = &[0xff, 0x00, 0x0a, b'{', 0xc3, 0x28, b'}', 0x0d];
    let outcome = anchor.submit(Submission { statement }).unwrap();

    assert_eq!(leaf_from_disk(dir, outcome.leaf_index), statement);

    // The genesis leaf is still where it was put, so the submission appended
    // rather than displacing anything.
    assert_eq!(leaf_from_disk(dir, 0), GENESIS);
    assert_eq!(
        hex(&Sha256::digest([&[0x00_u8], GENESIS].concat())),
        GOLDEN_GENESIS_LEAF_HASH
    );
}

#[test]
fn every_receipt_in_a_deeper_log_reconstructs_the_root_the_checkpoint_publishes() {
    // The two-leaf case below has an inclusion path of exactly ONE node, and a
    // one-node path is invariant under reordering — so the cross-check there
    // cannot catch a chunker that emits the right nodes in the wrong order.
    // This log is deep enough that most paths have three nodes, which makes
    // order observable.
    let tmp = TempDir::new().unwrap();
    let mut anchor = create_anchor(tmp.path());

    for n in 0..8_u8 {
        anchor
            .submit(Submission {
                statement: &[n; 11],
            })
            .unwrap();
    }
    assert_eq!(anchor.tree_size(), 9);

    // The root from the other derivation: ct-merkle's accumulator, base64, a
    // signed note, and lys-core's checkpoint parser.
    let published = anchor.publish_checkpoint().unwrap();
    let body = verify_checkpoint(published.note.as_bytes(), &verifier(&anchor))
        .expect("the checkpoint must verify");
    assert_eq!(body.tree_size(), 9);

    let key = anchor.signer().public_key();
    let mut checked = 0;
    let mut multi_node_paths = 0;
    for index in 0..9_u64 {
        let leaf = leaf_from_disk(tmp.path(), index);
        let receipt = anchor.receipt_for(index).unwrap();

        // Positive control: self-consistent, which is exactly the property
        // that would survive a wrongly ordered path.
        verify_receipt(&receipt, &leaf, &key).expect("the anchor's own receipt must verify");

        if receipt.inclusion_path.len() > 1 {
            multi_node_paths += 1;
        }
        assert_eq!(
            receipt
                .reconstructed_root(&leaf)
                .expect("the receipt's path must reconstruct"),
            body.root_hash(),
            "leaf {index}'s receipt attests to a root the anchor does not publish"
        );
        checked += 1;
    }
    // Count what fired, and count the cases that carry the property: a suite of
    // one-node paths would satisfy every assertion above while proving nothing
    // about order.
    assert_eq!(checked, 9);
    assert!(
        multi_node_paths >= 8,
        "only {multi_node_paths} of 9 receipts had a path long enough for order to matter"
    );
}

#[test]
fn the_receipt_reconstructs_the_root_the_checkpoint_publishes() {
    let tmp = TempDir::new().unwrap();
    let mut anchor = create_anchor(tmp.path());

    let outcome = anchor
        .submit(Submission {
            statement: STATEMENT,
        })
        .unwrap();

    // The receipt verifies — necessary, and on its own worth nothing here: it
    // reconstructs the root from the receipt's own path, which is the very
    // thing under test. It is asserted as a positive control so the equality
    // below is not the equality of two failures.
    verify_receipt(&outcome.receipt, STATEMENT, &anchor.signer().public_key())
        .expect("the anchor's own receipt must verify");

    // The cross-check. This root came from walking the chunked inclusion path.
    let walked = outcome
        .receipt
        .reconstructed_root(STATEMENT)
        .expect("the receipt's path must reconstruct");

    // This one came from ct-merkle's accumulator, through base64, into a
    // signed note, and back out through lys-core's checkpoint parser — a
    // different derivation of the same 32 bytes, decided by neither of the two
    // functions under test.
    let published = anchor.publish_checkpoint().unwrap();
    let body = verify_checkpoint(published.note.as_bytes(), &verifier(&anchor))
        .expect("the checkpoint must verify");
    assert_eq!(body.tree_size(), 2);
    assert_eq!(
        walked,
        body.root_hash(),
        "the root the receipt attests to is not the root the anchor publishes"
    );

    // And both agree with a value computed outside Rust from the two leaf
    // hashes, so a symmetric error shared by every Rust path here would still
    // be caught.
    assert_eq!(hex(&walked), GOLDEN_ROOT_2);
}

#[test]
fn duplicate_submissions_are_two_events_with_two_independently_verifying_receipts() {
    let tmp = TempDir::new().unwrap();
    let mut anchor = create_anchor(tmp.path());

    // Identical bytes, submitted twice. Not "the same statement" — the anchor
    // has no notion of sameness, and asking it to have one is asking it to
    // decide what a statement means.
    let mut outcomes: Vec<SubmissionOutcome> = Vec::new();
    for _ in 0..2 {
        outcomes.push(
            anchor
                .submit(Submission {
                    statement: STATEMENT,
                })
                .unwrap(),
        );
    }
    // Count what fired before anything is concluded from the loop.
    assert_eq!(outcomes.len(), 2);

    let first = &outcomes[0];
    let second = &outcomes[1];

    // Two distinct indices, in order, after genesis.
    assert_eq!(first.leaf_index, 1);
    assert_eq!(second.leaf_index, 2);
    assert_ne!(first.leaf_index, second.leaf_index);

    // The same bytes, so the same leaf hash — that is not de-duplication, and
    // the log holds both.
    assert_eq!(first.leaf_hash, second.leaf_hash);
    assert_eq!(first.tree_size, 2);
    assert_eq!(second.tree_size, 3);

    // Each receipt verifies against its own index, at its own size, and the
    // two attest to different roots because they are about different trees.
    let key = anchor.signer().public_key();
    let mut verified = 0;
    for outcome in &outcomes {
        verify_receipt(&outcome.receipt, STATEMENT, &key)
            .expect("each receipt must verify on its own terms");
        verified += 1;
    }
    assert_eq!(verified, 2);
    assert_ne!(
        first.receipt.reconstructed_root(STATEMENT).unwrap(),
        second.receipt.reconstructed_root(STATEMENT).unwrap()
    );
    assert_ne!(first.receipt.signature, second.receipt.signature);

    // "Independently" means the receipts are not interchangeable. Take the
    // first receipt and claim the second's index: the walk lands on a root the
    // anchor never signed, and verification fails. Without this, a pair of
    // receipts that both verified for the *wrong* reason would pass above.
    let mut relabelled = first.receipt.clone();
    relabelled.leaf_index = second.leaf_index;
    assert!(
        verify_receipt(&relabelled, STATEMENT, &key).is_err(),
        "a receipt must not verify at an index it was not issued for"
    );

    // Both leaves are on disk, and the log holds three.
    assert_eq!(leaf_from_disk(tmp.path(), 1), STATEMENT);
    assert_eq!(leaf_from_disk(tmp.path(), 2), STATEMENT);
    assert_eq!(anchor.tree_size(), 3);
}

#[test]
fn receipt_for_refuses_an_index_the_log_does_not_have() {
    let tmp = TempDir::new().unwrap();
    let mut anchor = create_anchor(tmp.path());
    anchor
        .submit(Submission {
            statement: STATEMENT,
        })
        .unwrap();

    // Positive control: the indices that exist are served, so the refusals
    // below are about the index and not about a method that refuses always.
    let mut served = 0;
    for index in 0..2_u64 {
        anchor
            .receipt_for(index)
            .expect("every logged leaf has a receipt");
        served += 1;
    }
    assert_eq!(served, 2);

    let mut refused = 0;
    for index in [2_u64, 3, u64::MAX] {
        let err = anchor
            .receipt_for(index)
            .expect_err("an index past the end has no leaf");
        assert!(
            matches!(
                err,
                AnchorError::NoSuchLeaf { leaf_index, tree_size, ref origin }
                    if leaf_index == index && tree_size == 2 && origin == ORIGIN
            ),
            "an absent index must be refused by name, got: {err}"
        );
        refused += 1;
    }
    assert_eq!(refused, 3);

    // Refusing must not have appended anything on the caller's behalf.
    assert_eq!(anchor.tree_size(), 2);
}

#[test]
fn receipt_for_refuses_a_log_that_holds_only_its_genesis_leaf() {
    let tmp = TempDir::new().unwrap();
    let mut anchor = create_anchor(tmp.path());
    assert_eq!(anchor.tree_size(), 1);

    // Named by this layer, not forwarded from lys-core's CDDL-cardinality
    // message, and reported for index 0 — the one index that does exist.
    let err = anchor
        .receipt_for(0)
        .expect_err("a one-leaf tree has no conforming receipt");
    assert!(
        matches!(
            err,
            AnchorError::TreeTooSmallForReceipt { tree_size, ref origin }
                if tree_size == 1 && origin == ORIGIN
        ),
        "a one-leaf log must be refused by name, got: {err}"
    );

    // Positive control, and the proof that the refusal is about the size and
    // nothing else: one submission removes the condition permanently.
    anchor
        .submit(Submission {
            statement: STATEMENT,
        })
        .unwrap();
    anchor
        .receipt_for(0)
        .expect("genesis has a receipt once a second leaf exists");
}
