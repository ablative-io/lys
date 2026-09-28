#![cfg(test)]
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

#[path = "../harness/mod.rs"]
mod harness;

use std::fmt::Write as _;

use harness::{build_go_tool, go_or_skip, run_built_tool};
use lys_core::Ed25519Identity;
use lys_core::bundle::{BundleLink, VerificationBundle, VerifiedBundle, verify_bundle};
use lys_core::checkpoint::{CheckpointBody, NoteVerifierKey, sign_note};
use lys_core::merkle::tree::{AppendOnlyTree, RawLeaf};
use lys_core::receipt::sign_receipt;
use lys_core::tlog::build_inclusion_artifact;

mod cases;
mod scenarios;

use cases::*;
use scenarios::*;

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

/// One complete scenario: the parties, the bundle, and the keys a verifier needs.
struct Scenario {
    bundle: VerificationBundle,
    log_key: String,
    anchors: Vec<String>,
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
        write!(acc, "{b:02x}").expect("writing to a String cannot fail");
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
        writeln!(
            report,
            "{} {} {}",
            to_hex(&notarization.anchor_root()),
            notarization.anchor_tree_size(),
            notarization.leaf_index(),
        )
        .expect("writing to a String cannot fail");
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
