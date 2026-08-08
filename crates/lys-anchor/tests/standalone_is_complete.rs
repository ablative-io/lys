#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! The DP19 gate: **a single anchor, zero witnesses, pinning to nobody, is
//! fully functional** — and the functionality is exercised in the shape that
//! has no federation in it.
//!
//! BUILD-PLAN §7.2. DP19 is *"it needs to be able to run standalone as well"*,
//! which is a hard structural requirement rather than a deployment mode.
//!
//! # The second party is the compiler, and the axis is reachability
//!
//! Nothing below names `witness`, `observe`, `Observation`, `Relation`,
//! `WitnessProjection` or any other item behind the `federation` feature, and
//! nothing below names `submit`, `receipt_for`, `SubmissionOutcome` or
//! `AnchorReceipt` behind `unstable-anchor`. There is no `#[cfg]` in this file:
//! it is one suite, compiled in **both** feature shapes, and in the default
//! shape those items **do not exist**. So the default `cargo test --workspace`
//! run compiling and passing this file is the claim, and a core path that
//! reached a federation item — or a standalone path that needed the draft
//! receipt gate to function — would break that run rather than pass it
//! quietly.
//!
//! Reachability is the only axis on which DP19 is a claim at all. It is
//! deliberately **not** a claim about algorithm, language, toolchain or
//! platform: the cross-language work belongs to `stranger_verification.rs`,
//! `checkpoint_note_conformance.rs` and `anchor_receipt_conformance.rs`, and is
//! not restated here.
//!
//! # Why this test could not exist before the append was ungated
//!
//! Until `Anchor::append` was split out of `Anchor::submit`, an anchor's only
//! mutator was gated on `unstable-anchor` — because its **return value**
//! carried a draft wire format. The consequence was not a missing convenience:
//! **a default-features anchor was frozen at tree size 1 forever.** `create`
//! wrote genesis and nothing else could append. So a "standalone is complete"
//! suite in the default shape could not have appended anything, and the only
//! leaf its inclusion artifact could have described was genesis. A missing
//! feature passes every test that exists, and the test that would have caught
//! this is the one that could not be written.
//!
//! # Where the checks are keyed
//!
//! Every value asserted against is one **this file** supplied or computed — the
//! origin literal, the statement bytes, the counted index — never a value read
//! back off the anchor and compared with itself. The artifact is checked by
//! `lys-core`'s third-party entry point, [`verify_inclusion_artifact`], after a
//! round trip through `serde_json`, and the verifier key is built from the
//! origin literal above rather than from `anchor.origin()`.

use std::path::Path;

use lys_anchor::{
    AcceptAll, Anchor, AnchorConfig, FileSigner, STANDALONE_DISCLOSURE, Signer, Submission,
    SubmitterContext, WitnessPosture,
};
use lys_core::checkpoint::{NoteVerifierKey, verify_checkpoint};
use lys_core::tlog::{InclusionProofArtifact, verify_inclusion_artifact};
use lys_log_store::FileLeafStore;
use tempfile::TempDir;

/// The origin this suite supplies to the store. Verifiers are built from *this*
/// literal, never from what the anchor reports back.
const ORIGIN: &str = "example.com/lys/standalone-gate";

/// The genesis bytes for every anchor built here.
const GENESIS: &[u8] = b"lys-anchor standalone gate genesis fixture";

/// The statements this suite appends. Three, so an inclusion path has more than
/// one node for at least one of them.
const STATEMENTS: [&[u8]; 3] = [
    b"the first statement a standalone anchor ever admitted",
    b"the second, so the tree is not a two-leaf special case",
    b"the third, so at least one inclusion path has two nodes",
];

/// `lys-core`'s conformance fixture seed, so key material is deterministic and
/// never generated.
const FIXTURE_SEED: &[u8; 32] = b"lys-go-conformance-test-seed-01!";

/// Loads a signer over the fixture seed, writing the key file into `dir`.
fn signer(dir: &Path) -> FileSigner {
    let path = dir.join("anchor.key");
    std::fs::write(&path, FIXTURE_SEED).unwrap();
    FileSigner::load(&path).unwrap()
}

/// Creates a store at `dir` under [`ORIGIN`] and an anchor over it.
fn create_anchor(dir: &Path) -> Anchor<FileLeafStore, FileSigner, AcceptAll> {
    let store = FileLeafStore::create(dir, ORIGIN).unwrap();
    Anchor::create(
        store,
        GENESIS,
        signer(dir),
        AcceptAll,
        AnchorConfig::unconfigured(),
    )
    .unwrap()
}

#[test]
fn the_whole_standalone_path_completes_with_no_peer_in_existence() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();

    // 1. Create. Genesis is leaf 0 and nothing else is in the log.
    let mut anchor = create_anchor(dir);
    assert_eq!(anchor.tree_size(), 1);

    // 2. Append. The step that did not exist in this shape before.
    let mut appended = Vec::new();
    for statement in STATEMENTS {
        appended.push(
            anchor
                .append(Submission { statement }, SubmitterContext::Unidentified)
                .unwrap(),
        );
    }
    assert_eq!(appended.len(), STATEMENTS.len());
    assert_eq!(anchor.tree_size(), 4);
    for (offset, outcome) in appended.iter().enumerate() {
        // Keyed on the index this file counted, not on one the anchor reported.
        assert_eq!(outcome.leaf_index, u64::try_from(offset).unwrap() + 1);
        assert_eq!(outcome.tree_size, outcome.leaf_index + 1);
    }

    // 3. Publish a checkpoint, and verify it as a stranger holding only the
    //    origin this file chose and the anchor's public key.
    let published = anchor.publish_checkpoint().unwrap();
    let verifier = NoteVerifierKey::new(ORIGIN, anchor.signer().public_key()).unwrap();
    let body = verify_checkpoint(published.note.as_bytes(), &verifier).unwrap();
    assert_eq!(body.origin(), ORIGIN);
    assert_eq!(body.tree_size(), 4);

    // 4. Produce a JSON artifact for every appended leaf, take it through a
    //    real codec, and verify it the way a third party would.
    let mut artifacts_checked = 0_u32;
    for (offset, statement) in STATEMENTS.iter().enumerate() {
        let leaf_index = u64::try_from(offset).unwrap() + 1;
        let artifact = anchor.inclusion_artifact(leaf_index).unwrap();
        let json = serde_json::to_string(&artifact).unwrap();
        let parsed: InclusionProofArtifact = serde_json::from_str(&json).unwrap();

        // The leaf bytes handed to the verifier are this file's, not the ones
        // the anchor would have read back for itself.
        let checked = verify_inclusion_artifact(&parsed, statement, &verifier).unwrap();
        assert_eq!(checked.origin(), ORIGIN);
        assert_eq!(parsed.leaf_index, leaf_index);
        assert_eq!(parsed.tree_size, 4);
        artifacts_checked += 1;
    }
    // Count what fired: a loop that ran zero times satisfies every assertion
    // inside it.
    assert_eq!(artifacts_checked, 3);

    // 5. Read status, with no second signature emitted to get it.
    let status = anchor.status();
    assert_eq!(status.origin, ORIGIN);
    assert_eq!(status.tree_size(), 4);
    assert_eq!(status.root.as_bytes(), body.root_hash());
    assert_eq!(status.recovered_to, None);
    assert_eq!(status.posture, WitnessPosture::Unwitnessed);
}

#[test]
fn a_fresh_anchor_is_unwitnessed_and_says_so_in_full() {
    let tmp = TempDir::new().unwrap();
    let anchor = create_anchor(tmp.path());

    let posture = anchor.status().posture;

    // ⚠️ This equality cannot fail while `WitnessPosture` has one variant.
    // §7.2 asks for it and it becomes load-bearing when increment 8 makes a
    // witnessed posture expressible; today it is not evidence, and the
    // assertions below it are.
    assert_eq!(posture, WitnessPosture::Unwitnessed);

    // A disclosure that can be silently dropped is not a disclosure. These
    // phrases are written out here rather than read from the constant the
    // implementation renders, so deleting the sentence from that constant does
    // not delete it from the expectation too.
    let rendered = posture.to_string();
    let required = [
        "no witnesses",
        "equivocate undetectably",
        "hold two histories",
        "no local check catches that",
        "one external party keeping its own durable memory",
    ];
    let mut found = 0_u32;
    for phrase in required {
        assert!(
            rendered.contains(phrase),
            "the posture's Display must contain {phrase:?}, got {rendered:?}"
        );
        found += 1;
    }
    assert_eq!(found, 5);

    // And the whole of it, so a shortened rendering is caught as well.
    assert_eq!(rendered, STANDALONE_DISCLOSURE);
}

#[test]
fn a_standalone_anchor_can_be_read_without_the_ability_to_sign_for_it() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    let (root, published, public_key) = {
        let mut anchor = create_anchor(dir);
        for statement in STATEMENTS {
            anchor
                .append(Submission { statement }, SubmitterContext::Unidentified)
                .unwrap();
        }
        (
            anchor.root(),
            anchor.publish_checkpoint().unwrap(),
            anchor.signer().public_key(),
        )
    };

    // No signer, no admission policy, and no signature emitted as a side
    // effect of asking what the log holds.
    let reader = Anchor::open_read_only(
        FileLeafStore::open(dir).unwrap(),
        AnchorConfig::unconfigured(),
    )
    .unwrap();

    let status = reader.status();
    assert_eq!(status.origin, ORIGIN);
    assert_eq!(status.tree_size(), 4);
    assert_eq!(status.root, root);
    assert_eq!(status.posture, WitnessPosture::Unwitnessed);
    assert_eq!(reader.leaf_bytes(0).unwrap(), GENESIS);

    // The root the keyless reader reports is the one the key-holding anchor
    // signed a checkpoint over — reached by a different route entirely, through
    // base64 into a note that `lys-core`'s verifier parses back out.
    let verifier = NoteVerifierKey::new(ORIGIN, public_key).unwrap();
    let body = verify_checkpoint(published.note.as_bytes(), &verifier).unwrap();
    assert_eq!(reader.root().as_bytes(), body.root_hash());
    assert_eq!(reader.root().num_leaves(), body.tree_size());
}
