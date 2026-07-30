//! Interop gate for `lys/verification-bundle/v1`: an independent Go
//! implementation must reach the **same verdict** as lys on every bundle, and
//! derive the same values from the ones it accepts.
//!
//! # Why this gate is shaped differently from the receipt gate
//!
//! The receipt gate can compare bytes, because a receipt is a single signed
//! artifact with a deterministic encoding — byte-identity is the strongest
//! claim available and it holds. A bundle is not like that. It is a container
//! whose entire value lies in the *relationships* between the artifacts inside
//! it, and those are established by checks, not by bytes. Confirming that Go can
//! parse the JSON would prove nothing about the thing that matters.
//!
//! So this gate asserts **verdict parity over a battery of cases**, in both
//! directions: every bundle lys accepts, the Go tool must accept; every bundle
//! lys refuses, the Go tool must refuse. A one-sided check would let a missing
//! relationship check pass unnoticed on whichever side omitted it — and a
//! missing relationship check is precisely the failure mode this format has, one
//! that reports success having established far less than a reader assumes.
//!
//! Each case's expected verdict is also written down here independently, so the
//! table is checked against the spec rather than against whatever the two
//! implementations happen to agree on. Two implementations agreeing on a wrong
//! answer is a real possibility when one was written by the same author.
//!
//! # The positive controls are load-bearing
//!
//! Six cases in the table are expected to be ACCEPTED, and they are what makes
//! the refusals meaningful. In particular
//! `unrelated_log_bundle_is_valid_on_its_own` and
//! `two_link_chain_truncated_to_one` establish that the artifacts used to build
//! the splice attacks are individually valid, and that a shorter chain is a true
//! weaker claim rather than an error. Without them, a verifier that refused
//! everything would pass every negative case in this file.
//!
//! One accepted case documents a limit rather than a capability:
//! `a_relabelled_size_survives_on_the_final_link`. Both implementations accept a
//! receipt whose `tree_size` was relabelled within its walk class when no
//! further link exists to contradict it. That is the honest reading of the
//! format, pinned as a test so it cannot quietly become a claim of more.
//!
//! # Every check here was proven load-bearing by drift injection
//!
//! Each of the two chain checks was removed in turn, on **both** sides, and in
//! every case exactly one case flipped and nothing else moved. No drift was
//! committed:
//!
//! | removed check | the only case that fails |
//! |---|---|
//! | the link-0 join | `link_zero_over_an_unrelated_log` |
//! | the rung's root comparison | `anchor_equivocates_at_the_same_tree_size` |
//! | the rung's size comparison | `relabelled_tree_size_contradicts_the_anchors_checkpoint` |
//!
//! The last two are why those two cases exist in the shape they do. The obvious
//! rung attack — an anchor that grows after issuing a receipt
//! (`anchor_published_a_root_its_receipt_never_vouched_for`) — changes both the
//! root *and* the size, so it cannot show either comparison is individually
//! necessary; removing the root check left it caught by the size check, and the
//! drift went unnoticed. Isolating each half needed a case that trips only that
//! half: equivocation at an unchanged size, and a relabelled size over an
//! unchanged root.
//!
//! # Independence of the Go side
//!
//! Nothing lys wrote is trusted on the other side of the comparison: signed
//! notes are opened by `golang.org/x/mod/sumdb/note` (the C2SP reference
//! implementation) against lys's hand-written note verifier, receipt signatures
//! go through `veraison/go-cose`, verifier-key text forms are re-parsed by
//! `note.NewVerifier` which recomputes the key ID lys derived, and Merkle roots
//! are rebuilt from RFC 6962 §2.1.1's *recursive* structure against lys's
//! iterative walk.
//!
//! # Environment contract
//!
//! See [`harness`] — vendored, network-free, and a hard failure rather than a
//! skip when `LYS_REQUIRE_GO` is set.

// The whole gate belongs to the draft format: with `unstable-anchor` off there is
// no `bundle` module to verify, and compiling the file out (rather than gating
// items inside it) leaves no unused shared harness behind either.
#![cfg(feature = "unstable-anchor")]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod harness;

use std::fmt::Write as _;

use harness::{build_go_tool, go_or_skip, run_built_tool};
use lys_core::Ed25519Identity;
use lys_core::bundle::{BundleLink, VerificationBundle, VerifiedBundle, verify_bundle};
use lys_core::checkpoint::{CheckpointBody, NoteVerifierKey, sign_note};
use lys_core::merkle::tree::{AppendOnlyTree, RawLeaf};
use lys_core::receipt::sign_receipt;
use lys_core::tlog::build_inclusion_artifact;

// ---------------------------------------------------------------- fixtures

/// A log or an anchor: an origin, a key, and a tree.
///
/// Built entirely through the public API, deliberately: this gate stands in for
/// the third party who holds nothing but the published crate, so anything it
/// needs a private helper for would be something a stranger could not do.
struct Party {
    origin: String,
    identity: Ed25519Identity,
    tree: AppendOnlyTree<RawLeaf>,
    _dir: tempfile::TempDir,
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
            _dir: dir,
        }
    }

    fn verifier(&self) -> NoteVerifierKey {
        NoteVerifierKey::new(&self.origin, self.identity.public_key_bytes()).unwrap()
    }

    /// The verifier-key text form, which is what the Go tool is handed. One key
    /// per party serves both roles — opening its notes and verifying its
    /// receipts — and the Go side enforces that binding independently.
    fn spec(&self) -> String {
        self.verifier().to_spec()
    }

    /// This party's own signed checkpoint at its current size.
    fn checkpoint(&self) -> String {
        let body = CheckpointBody::from_root(&self.origin, &self.tree.root()).unwrap();
        sign_note(&body.encode(), &self.origin, &self.identity).unwrap()
    }

    /// A receipt from this party proving `leaf` sits at `index` in its tree.
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

/// An anchor that has notarized `notarized`, appended after a genesis leaf so
/// its tree size is never 1 — the receipt format cannot express size 1.
fn anchor_over(origin: &str, seed: &[u8; 32], notarized: &[u8]) -> Party {
    let mut anchor = Party::new(origin, seed);
    anchor.tree.append_raw(b"genesis");
    anchor.tree.append_raw(notarized);
    anchor
}

/// The index the notarized checkpoint sits at in every anchor built above.
const NOTARIZED_INDEX: u64 = 1;

const CHILD_LEAF: &[u8] = b"entry-1";
const CHILD_LEAF_INDEX: u64 = 1;

/// A log with three entries, the middle one to be proven.
fn log_with_entries(origin: &str, seed: &[u8; 32]) -> Party {
    let mut log = Party::new(origin, seed);
    for leaf in [b"entry-0".as_slice(), CHILD_LEAF, b"entry-2".as_slice()] {
        log.tree.append_raw(leaf);
    }
    log
}

/// One complete scenario: the parties, the bundle, and the keys a verifier needs.
struct Scenario {
    bundle: VerificationBundle,
    log_key: String,
    anchors: Vec<String>,
}

/// A log, plus `depth` anchors each notarizing the one below it.
///
/// `depth` 0 is an unnotarized bundle — a leaf proven in a log nobody has
/// countersigned, which is weaker but perfectly valid.
fn chain(depth: usize) -> Scenario {
    let log = log_with_entries("child.example", b"lys-bundle-conformance-child-01a");
    let artifact = build_inclusion_artifact(
        &log.tree,
        CHILD_LEAF,
        &log.origin,
        &log.identity,
        CHILD_LEAF_INDEX,
    )
    .unwrap();

    let mut links = Vec::new();
    let mut anchors = Vec::new();
    // Each anchor notarizes the checkpoint below it: the log's own for the
    // first, then each anchor's own published checkpoint in turn.
    let mut notarized = artifact.checkpoint.clone();
    for level in 0..depth {
        let anchor = anchor_over(
            &format!("anchor-{level}.example"),
            &anchor_seed(level),
            notarized.as_bytes(),
        );
        let receipt = anchor.receipt_over(notarized.as_bytes(), NOTARIZED_INDEX);
        links.push(BundleLink::new(&notarized, &receipt));
        anchors.push(anchor.spec());
        notarized = anchor.checkpoint();
    }

    Scenario {
        bundle: VerificationBundle::new(CHILD_LEAF, artifact, links),
        log_key: log.spec(),
        anchors,
    }
}

/// Distinct 32-byte seeds per chain level, checked for length at the source so
/// a mistyped literal cannot silently become a different key.
fn anchor_seed(level: usize) -> [u8; 32] {
    let seed = format!("lys-bundle-conformance-anchor-{level:02}");
    let bytes = seed.into_bytes();
    assert_eq!(bytes.len(), 32, "anchor seed must be exactly 32 bytes");
    bytes.try_into().unwrap()
}

/// A log and its inclusion artifact, the starting point every scenario shares.
fn child_and_artifact() -> (Party, lys_core::tlog::InclusionProofArtifact) {
    let log = log_with_entries("child.example", b"lys-bundle-conformance-child-01a");
    let artifact = build_inclusion_artifact(
        &log.tree,
        CHILD_LEAF,
        &log.origin,
        &log.identity,
        CHILD_LEAF_INDEX,
    )
    .unwrap();
    (log, artifact)
}

/// A two-link chain in which anchor 0 grew **after** issuing its receipt and
/// then published a checkpoint that receipt never vouched for.
///
/// Every artifact here is individually valid: the receipt verifies over the
/// log's checkpoint, anchor 0's later checkpoint carries anchor 0's own
/// signature over its own origin, and anchor 1's receipt validly notarizes that
/// checkpoint. Only the *rung* is broken — the root anchor 0 vouched for is not
/// the root anchor 0 published — which is the case that lands squarely on the
/// root comparison rather than tripping a signature check on the way there.
///
/// This is the "history moved between vouching and publishing" attack, and it is
/// the one a chain of individually-valid receipts cannot detect without the
/// comparison.
fn divergent_rung() -> Scenario {
    let (log, artifact) = child_and_artifact();
    let log_note = artifact.checkpoint.clone();

    let mut anchor_0 = anchor_over("anchor-0.example", &anchor_seed(0), log_note.as_bytes());
    let receipt_0 = anchor_0.receipt_over(log_note.as_bytes(), NOTARIZED_INDEX);
    anchor_0
        .tree
        .append_raw(b"appended-after-the-receipt-was-issued");
    let diverged_note = anchor_0.checkpoint();

    let anchor_1 = anchor_over(
        "anchor-1.example",
        &anchor_seed(1),
        diverged_note.as_bytes(),
    );
    let receipt_1 = anchor_1.receipt_over(diverged_note.as_bytes(), NOTARIZED_INDEX);

    Scenario {
        bundle: VerificationBundle::new(
            CHILD_LEAF,
            artifact,
            vec![
                BundleLink::new(&log_note, &receipt_0),
                BundleLink::new(&diverged_note, &receipt_1),
            ],
        ),
        log_key: log.spec(),
        anchors: vec![anchor_0.spec(), anchor_1.spec()],
    }
}

/// A two-link chain in which anchor 0 **equivocates**: it vouches for one root
/// in its receipt and publishes a different root, at the same tree size, in its
/// own checkpoint.
///
/// This is the case that isolates the rung's *root* comparison. Growth after
/// issuing a receipt (see [`divergent_rung`]) changes the tree size too, so the
/// size comparison catches it either way; equivocation at an unchanged size is
/// caught by nothing but the root check. Both checkpoints here carry anchor 0's
/// genuine signature over its own origin, and both are size 2 — the only
/// disagreement is which history the anchor is describing, which is exactly the
/// dishonesty a notarization chain exists to expose.
fn equivocating_anchor() -> Scenario {
    let (log, artifact) = child_and_artifact();
    let log_note = artifact.checkpoint.clone();

    // The tree anchor 0 actually vouched for.
    let vouched = anchor_over("anchor-0.example", &anchor_seed(0), log_note.as_bytes());
    let receipt_0 = vouched.receipt_over(log_note.as_bytes(), NOTARIZED_INDEX);

    // A second history under the SAME key and origin, at the same size.
    let published = anchor_over(
        "anchor-0.example",
        &anchor_seed(0),
        b"a-second-entry-that-was-never-notarized",
    );
    let published_note = published.checkpoint();

    let anchor_1 = anchor_over(
        "anchor-1.example",
        &anchor_seed(1),
        published_note.as_bytes(),
    );
    let receipt_1 = anchor_1.receipt_over(published_note.as_bytes(), NOTARIZED_INDEX);

    Scenario {
        bundle: VerificationBundle::new(
            CHILD_LEAF,
            artifact,
            vec![
                BundleLink::new(&log_note, &receipt_0),
                BundleLink::new(&published_note, &receipt_1),
            ],
        ),
        log_key: log.spec(),
        anchors: vec![vouched.spec(), anchor_1.spec()],
    }
}

/// A chain whose first receipt **relabels its tree size**, exercising the one
/// half of the rung check that nothing else reaches.
///
/// The receipt format's `tree_size` is authenticated only as far as it changes
/// the reconstruction, and sizes 3 and 4 share a decision sequence at index 1 —
/// so a receipt carrying a size-4 path while claiming size 3 reconstructs the
/// same root and carries a valid signature. That malleability is documented in
/// `receipt::sign`, and it is a property of the RFC's proof format rather than
/// of either implementation, which is why both sides here accept the relabelled
/// receipt in isolation.
///
/// The bundle is what discharges it: the anchor's own note-signed checkpoint
/// says size 4, and the rung requires the two signed statements to agree. With
/// `links` truncated to one there is nothing to compare against, and the lie
/// survives — the honest limit, pinned below as an accepted case rather than
/// left as a claim.
fn relabelled_tree_size(with_second_link: bool) -> Scenario {
    let (log, artifact) = child_and_artifact();
    let log_note = artifact.checkpoint.clone();

    let mut anchor_0 = Party::new("anchor-0.example", &anchor_seed(0));
    anchor_0.tree.append_raw(b"genesis");
    anchor_0.tree.append_raw(log_note.as_bytes());
    anchor_0.tree.append_raw(b"later-0");
    anchor_0.tree.append_raw(b"later-1");

    let proof = anchor_0.tree.prove_inclusion(NOTARIZED_INDEX).unwrap();
    let path: Vec<[u8; 32]> = proof
        .as_bytes()
        .chunks_exact(32)
        .map(|c| <[u8; 32]>::try_from(c).unwrap())
        .collect();
    // Claiming 3 while carrying the size-4 path: same walk, same root, valid
    // signature.
    let relabelled = sign_receipt(
        log_note.as_bytes(),
        NOTARIZED_INDEX,
        3,
        &path,
        &anchor_0.identity,
    )
    .unwrap()
    .to_cose_bytes();

    let mut links = vec![BundleLink::new(&log_note, &relabelled)];
    let mut anchors = vec![anchor_0.spec()];
    if with_second_link {
        let a0_note = anchor_0.checkpoint();
        let anchor_1 = anchor_over("anchor-1.example", &anchor_seed(1), a0_note.as_bytes());
        links.push(BundleLink::new(
            &a0_note,
            &anchor_1.receipt_over(a0_note.as_bytes(), NOTARIZED_INDEX),
        ));
        anchors.push(anchor_1.spec());
    }

    Scenario {
        bundle: VerificationBundle::new(CHILD_LEAF, artifact, links),
        log_key: log.spec(),
        anchors,
    }
}

/// An entirely separate log-and-anchor pair, used to build the splice attacks:
/// artifacts that are individually valid but about the wrong log.
fn unrelated() -> Scenario {
    let log = log_with_entries("other.example", b"lys-bundle-conformance-other-01a");
    let artifact = build_inclusion_artifact(
        &log.tree,
        CHILD_LEAF,
        &log.origin,
        &log.identity,
        CHILD_LEAF_INDEX,
    )
    .unwrap();
    let checkpoint = artifact.checkpoint.clone();
    let anchor = anchor_over(
        "anchor-x.example",
        b"lys-bundle-conformance-anchor-xx",
        checkpoint.as_bytes(),
    );
    let receipt = anchor.receipt_over(checkpoint.as_bytes(), NOTARIZED_INDEX);

    Scenario {
        bundle: VerificationBundle::new(
            CHILD_LEAF,
            artifact,
            vec![BundleLink::new(&checkpoint, &receipt)],
        ),
        log_key: log.spec(),
        anchors: vec![anchor.spec()],
    }
}

// ------------------------------------------------------------- the case table

/// One case: a serialized bundle, the keys to verify it with, and the verdict
/// the wire spec requires. `json` rather than a struct because some cases are
/// container-shape attacks that cannot be expressed as a `VerificationBundle`
/// at all, and both implementations must be fed the same bytes.
struct Case {
    name: &'static str,
    json: String,
    log_key: String,
    anchors: Vec<String>,
    accept: bool,
}

fn case(name: &'static str, scenario: &Scenario, accept: bool) -> Case {
    Case {
        name,
        json: serde_json::to_string(&scenario.bundle).unwrap(),
        log_key: scenario.log_key.clone(),
        anchors: scenario.anchors.clone(),
        accept,
    }
}

/// Re-serialize a bundle after editing its JSON as a generic value, for the
/// shape attacks that a typed bundle cannot express.
fn edited_json(scenario: &Scenario, edit: impl FnOnce(&mut serde_json::Value)) -> String {
    let mut value: serde_json::Value = serde_json::to_value(&scenario.bundle).unwrap();
    edit(&mut value);
    serde_json::to_string(&value).unwrap()
}

fn cases() -> Vec<Case> {
    let mut cases = Vec::new();

    // ---- Accepted: the three legitimate shapes, plus the truncation control.
    cases.push(case("unnotarized_bundle", &chain(0), true));
    cases.push(case("one_link_chain", &chain(1), true));
    cases.push(case("two_link_chain", &chain(2), true));

    // Dropping the last link is a true weaker bundle, not an error: the
    // remaining chain still joins to the inclusion proof.
    let mut truncated = chain(2);
    truncated.bundle.links.pop();
    truncated.anchors.pop();
    cases.push(case("two_link_chain_truncated_to_one", &truncated, true));

    // The unrelated pair, valid on its own — the control that makes the two
    // splice refusals below demonstrably about the join rather than about a
    // broken fixture.
    cases.push(case(
        "unrelated_log_bundle_is_valid_on_its_own",
        &unrelated(),
        true,
    ));

    // ---- Refused: the join. Link 0 carries a receipt that verifies perfectly,
    // over a checkpoint that belongs to a different log than the one the
    // inclusion proof was verified against.
    let base = chain(1);
    let other = unrelated();
    let spliced = Scenario {
        bundle: VerificationBundle::new(
            CHILD_LEAF,
            base.bundle.inclusion_proof.clone(),
            other.bundle.links.clone(),
        ),
        log_key: base.log_key,
        anchors: other.anchors.clone(),
    };
    cases.push(case("link_zero_over_an_unrelated_log", &spliced, false));

    // ---- Refused: the rung. Link 1 is a valid receipt over a checkpoint that
    // is not anchor 0's own, so the ladder has a missing step.
    let mut misrung = chain(2);
    misrung.bundle.links[1] = other.bundle.links[0].clone();
    misrung.anchors[1].clone_from(&other.anchors[0]);
    cases.push(case("spliced_unrelated_second_link", &misrung, false));

    // ---- Refused: the rung again, this time reached on its own terms. Every
    // artifact is individually valid and every signature checks out; only the
    // root anchor 0 vouched for and the root anchor 0 published disagree.
    cases.push(case(
        "anchor_published_a_root_its_receipt_never_vouched_for",
        &divergent_rung(),
        false,
    ));

    // ---- Refused: equivocation at an unchanged size. Isolates the rung's root
    // comparison — nothing else in the verification can catch it.
    cases.push(case(
        "anchor_equivocates_at_the_same_tree_size",
        &equivocating_anchor(),
        false,
    ));

    // ---- Refused: a relabelled tree size, caught by the anchor's own
    // note-signed checkpoint. The only case where the rung's size comparison is
    // load-bearing alone — the root comparison passes.
    cases.push(case(
        "relabelled_tree_size_contradicts_the_anchors_checkpoint",
        &relabelled_tree_size(true),
        false,
    ));

    // ---- ACCEPTED, and it documents a real limit rather than a check: on the
    // final link there is no published checkpoint to contradict a relabelled
    // size, so the claim stands. Both implementations agree, because the
    // malleability belongs to the RFC's proof format, not to either of them.
    cases.push(case(
        "a_relabelled_size_survives_on_the_final_link",
        &relabelled_tree_size(false),
        true,
    ));

    // ---- Refused: order is load-bearing and validated, not assumed.
    let mut reordered = chain(2);
    reordered.bundle.links.swap(0, 1);
    reordered.anchors.swap(0, 1);
    cases.push(case("reordered_two_link_chain", &reordered, false));

    // ---- Refused: attribution. A receipt is only evidence about the anchor
    // whose key it names, so verifying link 0 under the wrong anchor's key must
    // fail even though both keys are real and both anchors exist.
    let mut swapped_keys = chain(2);
    swapped_keys.anchors.swap(0, 1);
    cases.push(case("anchor_keys_swapped", &swapped_keys, false));

    let mut wrong_log = chain(1);
    wrong_log.log_key = wrong_log.anchors[0].clone();
    cases.push(case("log_key_is_actually_the_anchors", &wrong_log, false));

    // ---- Refused: the key count must match exactly. Verifying a prefix would
    // report success for less than the bundle asserts.
    let mut too_few = chain(2);
    too_few.anchors.pop();
    cases.push(case("fewer_anchor_keys_than_links", &too_few, false));

    let mut too_many = chain(1);
    too_many.anchors.push(too_many.anchors[0].clone());
    cases.push(case("more_anchor_keys_than_links", &too_many, false));

    // ---- Refused: a populated counter-anchor slot. Nothing can verify one in
    // v1, and carrying an unverified time attestation is how a reader comes to
    // believe it.
    let mut countered = chain(1);
    countered.bundle.counter_anchor = Some("AAAA".to_string());
    cases.push(case("populated_counter_anchor", &countered, false));

    // ---- Refused: tampering, one field at a time.
    let mut bad_path = chain(1);
    bad_path.bundle.inclusion_proof.hashes[0] =
        flip_base64_byte(&bad_path.bundle.inclusion_proof.hashes[0]);
    cases.push(case("tampered_inclusion_path_node", &bad_path, false));

    let mut bad_receipt = chain(1);
    bad_receipt.bundle.links[0].receipt = flip_base64_byte(&bad_receipt.bundle.links[0].receipt);
    cases.push(case("tampered_receipt_byte", &bad_receipt, false));

    let mut bad_leaf = chain(1);
    bad_leaf.bundle.leaf = flip_base64_byte(&bad_leaf.bundle.leaf);
    cases.push(case("tampered_leaf", &bad_leaf, false));

    // A single flipped byte in link 0's checkpoint text breaks both the join
    // equality and the note signature over it.
    let mut bad_checkpoint = chain(1);
    bad_checkpoint.bundle.links[0].checkpoint = bad_checkpoint.bundle.links[0]
        .checkpoint
        .replace("child", "chxld");
    cases.push(case(
        "tampered_checkpoint_in_link_zero",
        &bad_checkpoint,
        false,
    ));

    // ---- Refused: container shape. A v1 bundle carrying a field v1 does not
    // define is not a v1 bundle, and an unrecognised format string is refused
    // before anything is parsed hopefully.
    let shaped = chain(1);
    cases.push(Case {
        name: "unknown_field_in_the_container",
        json: edited_json(&shaped, |value| {
            value
                .as_object_mut()
                .unwrap()
                .insert("extra".to_string(), serde_json::Value::Bool(true));
        }),
        log_key: shaped.log_key.clone(),
        anchors: shaped.anchors.clone(),
        accept: false,
    });
    cases.push(Case {
        name: "wrong_format_string",
        json: edited_json(&shaped, |value| {
            value.as_object_mut().unwrap().insert(
                "format".to_string(),
                serde_json::Value::String("lys/verification-bundle/v2".to_string()),
            );
        }),
        log_key: shaped.log_key.clone(),
        anchors: shaped.anchors.clone(),
        accept: false,
    });

    cases
}

/// Flips one bit inside a base64 payload, keeping it valid base64 of the same
/// length so the failure is the content rather than the encoding.
fn flip_base64_byte(encoded: &str) -> String {
    use base64::Engine;
    use base64::engine::general_purpose::STANDARD;
    let mut bytes = STANDARD.decode(encoded).unwrap();
    assert!(!bytes.is_empty(), "nothing to tamper with");
    bytes[0] ^= 0x01;
    STANDARD.encode(bytes)
}

// --------------------------------------------------------------- lys verdicts

/// lys's verdict on a case: `Some` with the evidence, or `None` for refused.
///
/// Deserialization failure counts as a refusal, which is the honest reading —
/// a container lys cannot parse is one it will not verify.
fn lys_verdict(case: &Case) -> Option<VerifiedBundle> {
    let bundle: VerificationBundle = serde_json::from_str(&case.json).ok()?;
    let log_key = NoteVerifierKey::from_spec(&case.log_key).ok()?;
    let anchors = case
        .anchors
        .iter()
        .map(|spec| NoteVerifierKey::from_spec(spec))
        .collect::<Result<Vec<_>, _>>()
        .ok()?;
    verify_bundle(&bundle, &log_key, &anchors).ok()
}

fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut acc, b| {
        let _ = write!(acc, "{b:02x}");
        acc
    })
}

/// The report the Go tool prints for an accepted bundle, derived here from
/// lys's own evidence. Comparing these is what makes the gate about *values*
/// rather than only about verdicts: both sides must have reconstructed the same
/// roots, at the same sizes and indices.
fn expected_report(verified: &VerifiedBundle) -> String {
    let checkpoint = verified.log_checkpoint();
    let mut report = format!(
        "{} {} {} {}\n",
        to_hex(verified.leaf()),
        checkpoint.origin(),
        checkpoint.tree_size(),
        to_hex(&checkpoint.root_hash()),
    );
    for notarization in verified.notarizations() {
        let _ = writeln!(
            report,
            "{} {} {}",
            to_hex(&notarization.anchor_root()),
            notarization.anchor_tree_size(),
            notarization.leaf_index(),
        );
    }
    report
}

// ------------------------------------------------------------------- the gates

/// Runs unconditionally, Go or no Go: every case's verdict is the one the wire
/// spec requires. A Go-less environment therefore never reduces what this file
/// covers — it only loses the second opinion.
#[test]
fn every_case_gets_the_verdict_the_spec_requires() {
    let cases = cases();
    assert_eq!(cases.len(), 23, "the case table lost or gained a case");
    for case in &cases {
        assert_eq!(
            lys_verdict(case).is_some(),
            case.accept,
            "lys disagreed with the spec on case {}",
            case.name
        );
    }
}

/// The interop gate: an independent implementation must agree, case by case.
#[test]
fn go_bundle_conformance_agrees_case_by_case() {
    let Some(go) = go_or_skip("verification-bundle conformance") else {
        return;
    };
    let workdir = tempfile::tempdir().unwrap();
    let gocache = workdir.path().join("gocache");
    let bin = workdir.path().join("cosetool");
    build_go_tool(&go, &gocache, &bin);

    let cases = cases();
    let mut checked = 0usize;
    let mut accepted = 0usize;
    for case in &cases {
        let mut args = vec!["bundle-verify".to_string(), case.log_key.clone()];
        args.extend(case.anchors.iter().cloned());
        let (go_ok, stdout) = run_built_tool(&bin, &args, case.json.as_bytes());

        assert_eq!(
            go_ok, case.accept,
            "the Go verifier disagreed with the spec on case {}",
            case.name
        );

        // Parity with lys, stated separately from parity with the spec so a
        // failure says which of the two broke.
        let verdict = lys_verdict(case);
        assert_eq!(
            go_ok,
            verdict.is_some(),
            "lys and the Go verifier disagreed on case {}",
            case.name
        );

        if let Some(verified) = verdict {
            accepted += 1;
            assert_eq!(
                String::from_utf8(stdout).unwrap(),
                expected_report(&verified),
                "the two implementations derived different values on case {}",
                case.name
            );
        }
        checked += 1;
    }

    // A loop that silently ran zero times would pass every assertion inside it,
    // and a table of nothing but refusals would too.
    assert_eq!(checked, cases.len(), "every case must be checked");
    assert_eq!(
        accepted, 6,
        "the positive controls must actually be accepted"
    );
}
