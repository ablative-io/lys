//! End-to-end gates over the `lys-anchor` binary as an operator runs it.
//!
//! # Where the second party comes from
//!
//! These tests do not call this crate's functions — a binary crate has no
//! library target, so there is nothing to call. They spawn the built binary and
//! read what it writes, which puts a process boundary between the assertions and
//! the implementation.
//!
//! That alone would still be one party. The artifacts are therefore handed to
//! **`lys-core`'s verifiers**, over the same path a stranger uses:
//! `verify_inclusion_artifact` for the JSON proof and `verify_receipt_bytes` for
//! the receipt. Neither knows a CLI produced the bytes, and both refuse on any
//! discrepancy. The axis is *algorithm* — producer and verifier are different
//! code — and explicitly not *platform*: one machine, one toolchain, one
//! dependency resolution.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;
use std::process::{Command, Output};

use lys_core::Ed25519Identity;
use lys_core::checkpoint::NoteVerifierKey;
use lys_core::tlog::{InclusionProofArtifact, verify_inclusion_artifact};
use serde_json::Value;

/// The binary under test, as cargo built it.
const BIN: &str = env!("CARGO_BIN_EXE_lys-anchor");
const ORIGIN: &str = "example.com/lys/anchor-cli-test";
const GENESIS: &[u8] = b"genesis for the anchor CLI gate";

/// A temporary anchor directory, key file and genesis file.
struct Fixture {
    _tmp: tempfile::TempDir,
    dir: PathBuf,
    key: PathBuf,
    genesis: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let tmp = tempfile::tempdir().unwrap();
        let key = tmp.path().join("anchor.key");
        Ed25519Identity::load_or_generate(&key).unwrap();
        let genesis = tmp.path().join("genesis.bin");
        std::fs::write(&genesis, GENESIS).unwrap();
        Self {
            dir: tmp.path().join("anchor"),
            key,
            genesis,
            _tmp: tmp,
        }
    }

    fn path(&self, name: &str) -> PathBuf {
        self.dir.parent().unwrap().join(name)
    }

    /// `init --admit accept-all`, the starting state most gates need.
    fn init(&self) -> Value {
        ok_json(&[
            "init",
            "--dir",
            self.dir.to_str().unwrap(),
            "--origin",
            ORIGIN,
            "--key",
            self.key.to_str().unwrap(),
            "--genesis",
            self.genesis.to_str().unwrap(),
            "--admit",
            "accept-all",
        ])
    }
}

fn run(args: &[&str]) -> Output {
    Command::new(BIN)
        .args(args)
        .output()
        .expect("the binary must be runnable")
}

/// Runs with `--json`, asserts success, and returns the parsed object.
fn ok_json(args: &[&str]) -> Value {
    let mut with_json = args.to_vec();
    with_json.push("--json");
    let output = run(&with_json);
    assert!(
        output.status.success(),
        "{args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: Value = serde_json::from_slice(&output.stdout)
        .unwrap_or_else(|err| panic!("{args:?} did not emit one JSON object: {err}"));
    assert_eq!(value["ok"], Value::Bool(true), "{value}");
    value
}

/// Only the receipt gate needs a raw public key, and the receipt gate only
/// exists with `unstable-anchor` — so this helper does too, rather than sitting
/// behind an `allow(dead_code)` in the default build.
#[cfg(feature = "unstable-anchor")]
fn hex32(text: &str) -> [u8; 32] {
    let bytes = text.as_bytes();
    assert_eq!(bytes.len(), 64, "not a 32-byte hex string: {text}");
    let mut out = [0u8; 32];
    for (slot, pair) in out.iter_mut().zip(bytes.chunks_exact(2)) {
        let hi = char::from(pair[0]).to_digit(16).unwrap();
        let lo = char::from(pair[1]).to_digit(16).unwrap();
        *slot = u8::try_from((hi << 4) | lo).unwrap();
    }
    out
}

fn read_artifact(path: &PathBuf) -> InclusionProofArtifact {
    let text = std::fs::read_to_string(path).expect("the artifact must have been written");
    serde_json::from_str(&text).expect("the artifact must be valid JSON of its own type")
}

/// The whole core path an operator runs, and the proof it produces is checked by
/// `lys-core`'s own third-party verifier rather than by this file.
///
/// The verifier is given only what a stranger has: the artifact, the verifier
/// key string the CLI printed, and the genesis bytes. It never sees the anchor
/// directory.
#[test]
fn init_status_checkpoint_and_prove_produce_a_verifiable_proof() {
    let fixture = Fixture::new();

    let init = fixture.init();
    assert_eq!(init["origin"], Value::String(ORIGIN.into()));
    assert_eq!(init["tree_size"], Value::from(1u64));
    assert_eq!(init["genesis_leaf_index"], Value::from(0u64));
    assert_eq!(init["recovered_to"], Value::Null, "a fresh anchor is clean");
    // Delivery only, deliberately not wording. What the sentence must SAY is
    // pinned against the build plan by `open_tests`; asserting the words here
    // as well would mean one reworded constant broke two tests, and a rule
    // guarded twice is a rule neither test proves on its own.
    let disclosure = init["standalone_disclosure"].as_str().unwrap();
    assert!(
        !disclosure.is_empty(),
        "the disclosure must reach a --json consumer, not only a terminal"
    );

    let status = ok_json(&[
        "status",
        "--dir",
        fixture.dir.to_str().unwrap(),
        "--key",
        fixture.key.to_str().unwrap(),
        "--admit",
        "accept-all",
    ]);
    assert_eq!(status["tree_size"], Value::from(1u64));
    assert_eq!(status["origin"], init["origin"]);
    assert_eq!(
        status["verifier_key"], init["verifier_key"],
        "the identity an operator hands out must not change between commands"
    );

    let note_path = fixture.path("checkpoint.note");
    let checkpoint = ok_json(&[
        "checkpoint",
        "--dir",
        fixture.dir.to_str().unwrap(),
        "--key",
        fixture.key.to_str().unwrap(),
        "--out",
        note_path.to_str().unwrap(),
        "--admit",
        "accept-all",
    ]);
    let note = std::fs::read_to_string(&note_path).expect("the note must have been written");
    assert!(note.starts_with(ORIGIN), "signed under the origin: {note}");
    assert_eq!(
        checkpoint["root_hash"].as_str().unwrap().len(),
        64,
        "a SHA-256 root is 64 hex characters"
    );

    let artifact_path = fixture.path("genesis-inclusion.json");
    let proof = ok_json(&[
        "prove",
        "--dir",
        fixture.dir.to_str().unwrap(),
        "--key",
        fixture.key.to_str().unwrap(),
        "--leaf-index",
        "0",
        "--out",
        artifact_path.to_str().unwrap(),
        "--admit",
        "accept-all",
    ]);
    assert_eq!(
        proof["artifact_format"],
        Value::String("lys/log-inclusion-proof/v1".into())
    );
    assert_eq!(
        proof["inclusion_path_nodes"],
        Value::from(0u64),
        "a one-leaf tree's inclusion path is empty"
    );

    // The stranger's check: lys-core's verifier, the printed verifier key, the
    // raw leaf bytes, and nothing else.
    let verifier = NoteVerifierKey::from_spec(proof["verifier_key"].as_str().unwrap()).unwrap();
    let body = verify_inclusion_artifact(&read_artifact(&artifact_path), GENESIS, &verifier)
        .expect("the artifact the CLI wrote must verify for a third party");
    assert_eq!(body.origin(), ORIGIN);
    assert_eq!(body.tree_size(), 1);
}

/// A directory holding no anchor is refused, and the refusal names the command
/// that creates one — this binary's, not `lys log init`'s.
#[test]
fn an_uninitialized_directory_is_refused_with_a_remedy() {
    let fixture = Fixture::new();
    let output = run(&[
        "status",
        "--dir",
        fixture.dir.to_str().unwrap(),
        "--key",
        fixture.key.to_str().unwrap(),
        "--admit",
        "accept-all",
    ]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("not initialized"), "{stderr}");
    assert!(stderr.contains("lys-anchor init"), "{stderr}");
}

/// A `--json` caller receives JSON on the failure path too.
///
/// The caller asked for parseable output; handing it a bare stderr line at the
/// moment a pipeline is deciding whether to proceed is the silent-failure shape
/// this repository refuses elsewhere.
#[test]
fn a_failure_under_json_is_still_json() {
    let fixture = Fixture::new();
    let output = run(&[
        "status",
        "--dir",
        fixture.dir.to_str().unwrap(),
        "--key",
        fixture.key.to_str().unwrap(),
        "--admit",
        "accept-all",
        "--json",
    ]);
    assert!(!output.status.success());
    let value: Value =
        serde_json::from_slice(&output.stdout).expect("a --json failure must still parse");
    assert_eq!(value["ok"], Value::Bool(false));
    assert!(value["error"].as_str().unwrap().contains("not initialized"));
    assert!(
        !String::from_utf8_lossy(&output.stderr).is_empty(),
        "the operator's diagnostic must still reach stderr"
    );
}

/// A flag the chosen policy does not read stops the command rather than being
/// ignored, and the anchor is not touched.
#[test]
fn an_unread_admission_flag_stops_the_command() {
    let fixture = Fixture::new();
    fixture.init();
    let output = run(&[
        "status",
        "--dir",
        fixture.dir.to_str().unwrap(),
        "--key",
        fixture.key.to_str().unwrap(),
        "--admit",
        "accept-all",
        "--max-bytes",
        "10",
    ]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("does not read"), "{stderr}");
    assert!(stderr.contains("--max-bytes"), "{stderr}");
}

/// Without `unstable-anchor` the subcommand is absent, not merely refused.
///
/// This is the shape consumers get by default, and the gate is the binary's own
/// argument parser: `submit` is compiled out because the receipt it writes is a
/// draft format, and a build that did not ask for a draft must not be able to
/// sign one.
#[cfg(not(feature = "unstable-anchor"))]
#[test]
fn submit_does_not_exist_in_a_default_build() {
    let output = run(&["submit", "--help"]);
    assert!(!output.status.success(), "submit must not be a subcommand");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("submit"), "{stderr}");

    // The positive control: a subcommand that IS present prints help and exits
    // zero, so the failure above is about `submit` and not about `--help`.
    let control = run(&["status", "--help"]);
    assert!(control.status.success(), "the control must succeed");
}

/// The receipt and the proof a submission produces both verify, for a third
/// party holding only the statement bytes and the anchor's public key.
#[cfg(feature = "unstable-anchor")]
#[test]
fn a_submitted_statement_yields_a_verifiable_receipt_and_proof() {
    use lys_core::receipt::verify_receipt_bytes;

    let fixture = Fixture::new();
    fixture.init();

    let statement_path = fixture.path("statement.bin");
    let statement = b"a statement the anchor does not interpret";
    std::fs::write(&statement_path, statement).unwrap();
    let receipt_path = fixture.path("receipt.cose");
    let artifact_path = fixture.path("statement-inclusion.json");

    let submitted = ok_json(&[
        "submit",
        "--dir",
        fixture.dir.to_str().unwrap(),
        "--key",
        fixture.key.to_str().unwrap(),
        "--statement",
        statement_path.to_str().unwrap(),
        "--receipt-out",
        receipt_path.to_str().unwrap(),
        "--artifact-out",
        artifact_path.to_str().unwrap(),
        "--admit",
        "accept-all",
    ]);
    assert_eq!(submitted["leaf_index"], Value::from(1u64));
    assert_eq!(submitted["tree_size"], Value::from(2u64));

    let anchor_key = hex32(submitted["signer_public_key"].as_str().unwrap());
    let receipt_bytes = std::fs::read(&receipt_path).expect("the receipt must have been written");
    let receipt = verify_receipt_bytes(&receipt_bytes, statement, &anchor_key)
        .expect("the receipt the CLI wrote must verify for a third party");
    assert_eq!(receipt.leaf_index, 1);
    assert_eq!(receipt.tree_size, 2);

    let verifier = NoteVerifierKey::from_spec(submitted["verifier_key"].as_str().unwrap()).unwrap();
    let artifact = read_artifact(&artifact_path);
    let body = verify_inclusion_artifact(&artifact, statement, &verifier)
        .expect("the JSON proof written beside the receipt must verify too");

    // The agreement the artifact module says a caller must check rather than
    // assume. It holds because the CLI takes both against one in-memory tree
    // with no append between them; a change that reopened the anchor in between
    // would break exactly this assertion.
    assert_eq!(artifact.tree_size, receipt.tree_size);
    assert_eq!(artifact.leaf_index, receipt.leaf_index);
    assert_eq!(body.tree_size(), receipt.tree_size);
    assert_eq!(
        body.root_hash(),
        receipt.reconstructed_root(statement).unwrap(),
        "the checkpoint's root and the receipt's reconstructed root are one tree"
    );
}

/// A refused submission appends nothing, and the refusal discloses no rule.
///
/// Keyed on the tree size the anchor itself reports afterwards — a value this
/// test did not supply — rather than on the absence of an output file, which a
/// command that appended and then failed to write would also produce.
#[cfg(feature = "unstable-anchor")]
#[test]
fn a_refused_submission_leaves_the_log_untouched() {
    let fixture = Fixture::new();
    fixture.init();

    let statement_path = fixture.path("too-long.bin");
    std::fs::write(&statement_path, vec![b'x'; 100]).unwrap();
    let receipt_path = fixture.path("refused.cose");
    let artifact_path = fixture.path("refused.json");

    let submit = |limit: &str| {
        run(&[
            "submit",
            "--dir",
            fixture.dir.to_str().unwrap(),
            "--key",
            fixture.key.to_str().unwrap(),
            "--statement",
            statement_path.to_str().unwrap(),
            "--receipt-out",
            receipt_path.to_str().unwrap(),
            "--artifact-out",
            artifact_path.to_str().unwrap(),
            "--admit",
            "max-size",
            "--max-bytes",
            limit,
        ])
    };

    let refused = submit("1");
    assert!(
        !refused.status.success(),
        "100 bytes must exceed a 1-byte limit"
    );
    let stderr = String::from_utf8_lossy(&refused.stderr);
    assert!(stderr.contains("did not admit"), "{stderr}");
    assert!(
        !stderr.contains("100") && !stderr.contains("max-bytes"),
        "a refusal must not read the rule back out: {stderr}"
    );
    assert!(
        !receipt_path.exists() && !artifact_path.exists(),
        "a refusal writes nothing"
    );

    let after = ok_json(&[
        "status",
        "--dir",
        fixture.dir.to_str().unwrap(),
        "--key",
        fixture.key.to_str().unwrap(),
        "--admit",
        "accept-all",
    ]);
    assert_eq!(
        after["tree_size"],
        Value::from(1u64),
        "a refused submission must occupy no index"
    );

    // The positive control: the same bytes under a limit that permits them are
    // admitted, so the refusal above was the limit and not the plumbing.
    let admitted = submit("100");
    assert!(
        admitted.status.success(),
        "{}",
        String::from_utf8_lossy(&admitted.stderr)
    );
    let grown = ok_json(&[
        "status",
        "--dir",
        fixture.dir.to_str().unwrap(),
        "--key",
        fixture.key.to_str().unwrap(),
        "--admit",
        "accept-all",
    ]);
    assert_eq!(grown["tree_size"], Value::from(2u64));
}
