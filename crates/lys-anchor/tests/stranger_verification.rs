#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! The stranger gate: an anchor's inclusion artifact, judged by a script that
//! has never heard of this workspace.
//!
//! # What this exists to stop happening again
//!
//! Four committed documents cite a "15-line independent verification script" —
//! `docs/ROADMAP.md`, and the anchor's `DECISIONS.md`, `STRAWMAN.md` and
//! `BUILD-PLAN.md`, one of which calls it *the strongest evidence this project
//! has that its claims are real*. **The script was never in the repository.**
//! `git log --all --diff-filter=A` over the whole history found no such file.
//! It was demonstrated once, in a session, in July; the demonstration was
//! recorded and the instrument was not, so "demonstrated, not claimed" decayed
//! back into a claim with no event marking the crossing.
//!
//! `scripts/verify_inclusion.py` is that instrument, committed. This file is
//! what keeps it one: a script nothing runs is a script nothing notices the
//! rotting of.
//!
//! # Which axis of independence this is, and which it is not
//!
//! **Algorithm and language.** The script's Merkle walk is transcribed from
//! RFC 6962 §2.1.1 — the recursive `PATH` definition, read backwards — by an
//! author who did not read `lys-core`'s Rust walk. It reads the artifact's
//! fields and the checkpoint's three-line body, both of which are wire formats
//! written down in `docs/design/WIRE-FORMATS.md`; those describe the container,
//! and the container is meant to be read. The hashing is the RFC's.
//!
//! **Not platform, and not custody.** One machine, one Python, one toolchain.
//! And the script does not verify the checkpoint's Ed25519 signature — there is
//! no Ed25519 in the Python standard library and it takes no dependencies. It
//! answers exactly one question: does this leaf hash into that root, at that
//! index. Whether the root was signed by the log you meant is a second question
//! with a second answer, and `checkpoint_note_conformance.rs` is where the
//! signature is judged by a second implementation.
//!
//! # The negative controls are the point, not the trimming
//!
//! A test made only of successes cannot tell a working verifier from one that
//! accepts everything, and this repository has already shipped the mirror-image
//! defect once — a Go conformance test that stayed green under a drifted
//! constant because every assertion in it asserted a *refusal*. So the script
//! is offered a corrupted leaf, a flipped node in the path, a changed index, a
//! contradicted tree size, an unknown field, a wrong `format`, and a supplied
//! root that is not the log's.
//!
//! Each refusal asserts the script's **verification-failure** exit status (2)
//! and its `VERIFICATION FAILED` marker on stderr, not merely a non-zero exit —
//! a missing script, a syntax error or an unreadable file all exit non-zero
//! too, and every one of those would let all seven refusals pass while nothing
//! was ever verified. The same trap sits under the positive case, so it is
//! keyed on the `INCLUSION VERIFIED` marker rather than on exit 0 alone, and
//! the script's presence on disk is asserted before anything is spawned.
//!
//! # What the injection sweep measured
//!
//! Each check in the script was removed or drifted in turn and this suite
//! re-run — eight injections. Reported case by case rather than summarized,
//! because the last one overturned a belief:
//!
//! - Interior-node prefix `0x01`, leaf prefix `0x00`, and the sidedness of the
//!   `m < k` branch: each drift fails the two positive tests. The walk is load
//!   bearing.
//! - A `verify` stubbed to accept everything: **all seven refusals fail**, and
//!   only the positive test passes. That is the result that says the refusals
//!   can tell a working verifier from a permissive one.
//! - Dropping the supplied-root check, the unknown-field check, the `format`
//!   check, or the `tree_size`-versus-checkpoint cross-check: **exactly one
//!   test fails each time, and it is the one built for that check.**
//!
//! # `tree_size` is not redundant with the proof, and the sweep is how that was learned
//!
//! The last of those was expected to fail nothing. The reasoning was that the
//! walk consumes the artifact's `tree_size`, so a lie about it should land the
//! recomputation on a different root and be refused anyway — making the
//! cross-check a nicer error message over a check that already existed.
//!
//! **That reasoning was wrong, and the injection said so.** Under RFC 6962 the
//! sidedness sequence for leaf `m` depends on `n` only through which power-of-two
//! split each level falls on, and whole ranges of `n` share one sequence: for
//! `m = 3`, tree sizes **5, 6, 7 and 8 all produce the identical path shape and
//! length**. So the fixture's five-leaf artifact, relabelled `tree_size: 6`,
//! recomputes the *same* root — the five-leaf one — and matches the five-leaf
//! checkpoint. A verifier without the cross-check accepts it and reports "leaf 3
//! of 6" about a log that has five leaves.
//!
//! Which means WIRE-FORMATS §3.3 is stronger than the heading it sits under.
//! "Redundancy is checked, not trusted" understates the rule: `tree_size` is
//! **not** redundant with `(leaf_index, hashes)`, and the embedded signed
//! checkpoint is the only thing that pins it. `lys-core` performs this
//! comparison; so must any third-party verifier, and that is now a rule with a
//! case behind it rather than an inference.
//!
//! # The skip policy
//!
//! A missing `python3` is a developer-machine skip. A missing `python3` with
//! `LYS_REQUIRE_PYTHON` set is a hard failure — the same contract
//! `lys-core/tests/harness/mod.rs` holds for Go, and for the same reason: this
//! gate must never quietly degrade to "passed", which is how the instrument
//! vanished the first time.
//!
//! # What produces the artifact, and what only stages the fixture
//!
//! The artifact is the anchor's own: [`Anchor::inclusion_artifact`], the
//! shipped entry point, over a file-backed store in a temporary directory. The
//! anchor also creates that store and writes leaf 0, and its signer signs the
//! embedded checkpoint.
//!
//! The four statements after genesis are appended through `Log` rather than
//! through `Anchor::submit`, because `submit` is behind the off-by-default
//! `unstable-anchor` feature and this gate must run in both feature shapes —
//! the default build is where a stranger's artifact actually comes from. The
//! staging path is the same one `checkpoint_note_conformance.rs` already uses:
//! the anchor is closed, the store reopened as a `Log`, appended to, and the
//! anchor reopened over it. Nothing about the leaf bytes or the tree differs
//! from what `submit` would have produced — `submit` appends the statement
//! verbatim — so what is being staged is the log's contents, not the artifact.

use std::path::{Path, PathBuf};
use std::process::Command;

use lys_anchor::{Anchor, AnchorConfig, FileSigner};
use lys_log_store::{FileLeafStore, Log};
use tempfile::TempDir;

/// The origin the store is created under, and the note key name it signs with.
const ORIGIN: &str = "example.com/lys/stranger-gate";

/// Leaf 0, written by [`Anchor::create`].
const GENESIS: &[u8] = b"lys-anchor stranger-gate genesis";

/// Leaves 1..=4. Five leaves is the smallest size whose inclusion paths are not
/// all the same shape: index 4 has a one-node path, index 0 a three-node one,
/// and the sweep therefore exercises both sides of the RFC's `m < k` branch at
/// the top of the tree as well as inside it.
const STATEMENTS: [&[u8]; 4] = [
    b"stranger-gate statement one",
    b"stranger-gate statement two",
    b"stranger-gate statement three",
    b"stranger-gate statement four",
];

/// Total leaves in the fixture log.
const TREE_SIZE: u64 = 5;

/// A fixed seed, so the fixture is the same log on every run.
const FIXTURE_SEED: &[u8; 32] = b"lys-stranger-gate-seed-0000000!!";

/// Exit status the script reserves for "a check ran and did not hold".
const EXIT_VERIFICATION_FAILED: i32 = 2;

/// The committed instrument. Its absence is a hard failure, never a skip.
fn script() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/verify_inclusion.py")
}

/// Candidates tried in order, after the `LYS_PYTHON_BIN` override.
///
/// **The pinned absolute paths are not belt-and-braces; they were earned.** On
/// the machine this gate was written on, `python3` first on `PATH` resolved to
/// a **Linux x86-64 ELF binary** installed under `~/.local/bin`, which macOS
/// cannot exec at all. A probe that stopped at the first `PATH` hit therefore
/// reported "no Python here" on a machine with two working interpreters, and
/// this whole gate skipped — silently, and looking exactly like five passes.
/// The lesson is the one the file already turns on: a control that never fires
/// is indistinguishable from one that passed, so the probe keeps looking until
/// something actually runs.
const PYTHON_CANDIDATES: [&str; 4] = [
    "python3",
    "/usr/bin/python3",
    "/opt/homebrew/bin/python3",
    "/usr/local/bin/python3",
];

/// Returns `path` if it executes and identifies itself as Python 3.
///
/// Checked by what the interpreter *said*, not by whether a process was
/// spawned: a name on `PATH` that exits 0 is not evidence of a Python.
fn usable_python(path: &str) -> Option<PathBuf> {
    let out = Command::new(path).arg("--version").output().ok()?;
    let version =
        String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr);
    (out.status.success() && version.starts_with("Python 3")).then(|| PathBuf::from(path))
}

/// Locates a Python 3: `LYS_PYTHON_BIN` override, then [`PYTHON_CANDIDATES`].
fn find_python() -> Option<PathBuf> {
    if let Ok(overridden) = std::env::var("LYS_PYTHON_BIN") {
        // An explicit override is obeyed, not second-guessed: an operator who
        // named an interpreter wants that one, and a silent fallback to another
        // would run the gate under something they did not choose.
        return Some(PathBuf::from(overridden));
    }
    PYTHON_CANDIDATES.iter().copied().find_map(usable_python)
}

/// Resolves Python, or returns `None` after announcing the skip.
///
/// # Panics
///
/// Panics if no interpreter is found while `LYS_REQUIRE_PYTHON` is set — in
/// that environment the gate is required, and skipping it would report a pass
/// for a cross-check that never ran. Panics too if the script is missing: a
/// present interpreter and an absent script is the exact combination under
/// which every refusal below would still "pass".
fn python_or_skip(gate: &str) -> Option<PathBuf> {
    assert!(
        script().is_file(),
        "{} is missing — the stranger gate has no instrument to run",
        script().display()
    );
    if let Some(python) = find_python() {
        return Some(python);
    }
    assert!(
        std::env::var_os("LYS_REQUIRE_PYTHON").is_none(),
        "LYS_REQUIRE_PYTHON is set but no python3 was found — \
         the {gate} gate must not skip in this environment"
    );
    eprintln!("skipping {gate}: no python3 found");
    None
}

/// One case on disk: the artifact JSON, the leaf bytes, and the root a stranger
/// would have been told out of band.
struct Case {
    /// Kept alive for the lifetime of the case; dropping it deletes the files.
    _tmp: TempDir,
    artifact_path: PathBuf,
    leaf_path: PathBuf,
    /// The parsed artifact, for the tests that mutate a field before rewriting.
    artifact: serde_json::Value,
    leaf: Vec<u8>,
    /// Line 3 of the embedded checkpoint, taken by splitting the note here
    /// rather than by asking `lys-core` to parse it.
    root_b64: String,
}

/// Stages the fixture log and asks the anchor for the artifact at `leaf_index`.
fn build_case(leaf_index: u64) -> Case {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();

    let key_path = dir.join("anchor.key");
    std::fs::write(&key_path, FIXTURE_SEED).unwrap();
    let signer = || FileSigner::load(&key_path).unwrap();

    let store = FileLeafStore::create(dir, ORIGIN).unwrap();
    Anchor::create(store, GENESIS, signer(), AnchorConfig::unconfigured()).unwrap();

    let mut log = Log::open(FileLeafStore::open(dir).unwrap()).unwrap();
    for statement in STATEMENTS {
        log.append(statement).unwrap();
    }
    assert_eq!(log.tree().len(), TREE_SIZE, "fixture log is the wrong size");
    let leaf = log
        .leaf_bytes(leaf_index)
        .unwrap_or_else(|| panic!("no leaf at index {leaf_index}"))
        .to_vec();
    drop(log);
    // An empty fixture makes a positive control inert: hashing nothing produces
    // a correct-looking digest whether the file was read or the read failed.
    assert!(!leaf.is_empty(), "the leaf under test must have bytes");

    let anchor = Anchor::open(
        FileLeafStore::open(dir).unwrap(),
        signer(),
        AnchorConfig::unconfigured(),
    )
    .unwrap();
    let artifact = anchor.inclusion_artifact(leaf_index).unwrap();
    let root_b64 = artifact
        .checkpoint
        .split('\n')
        .nth(2)
        .expect("a checkpoint body has three lines")
        .to_string();
    let value = serde_json::to_value(&artifact).unwrap();

    let artifact_path = dir.join("inclusion.json");
    let leaf_path = dir.join("leaf.bin");
    std::fs::write(&artifact_path, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
    std::fs::write(&leaf_path, &leaf).unwrap();

    Case {
        _tmp: tmp,
        artifact_path,
        leaf_path,
        artifact: value,
        leaf,
        root_b64,
    }
}

/// Runs the script; returns `(exit code, stdout, stderr)`.
fn run(python: &Path, case: &Case, expected_root: Option<&str>) -> (Option<i32>, String, String) {
    let mut command = Command::new(python);
    command
        .arg(script())
        .arg(&case.artifact_path)
        .arg(&case.leaf_path);
    if let Some(root) = expected_root {
        command.arg(root);
    }
    let out = command.output().expect("failed to spawn python3");
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// Asserts the script refused *because a check failed*, not because it broke.
fn assert_refused(python: &Path, case: &Case, expected_root: Option<&str>, what: &str) {
    let (code, stdout, stderr) = run(python, case, expected_root);
    assert_eq!(
        code,
        Some(EXIT_VERIFICATION_FAILED),
        "{what}: expected a verification failure, got {code:?}\n\
         stdout: {stdout}\nstderr: {stderr}"
    );
    assert!(
        stderr.contains("VERIFICATION FAILED"),
        "{what}: the script exited {code:?} without reporting a failed check — \
         a broken or missing script exits non-zero too\nstderr: {stderr}"
    );
    assert!(
        !stdout.contains("INCLUSION VERIFIED"),
        "{what}: the script both verified and refused"
    );
}

/// Rewrites the case's artifact file from `value`, and proves the bytes moved.
fn rewrite_artifact(case: &Case, value: &serde_json::Value) {
    let before = std::fs::read(&case.artifact_path).unwrap();
    let after = serde_json::to_vec_pretty(value).unwrap();
    assert_ne!(before, after, "the mutation did not change the artifact");
    std::fs::write(&case.artifact_path, after).unwrap();
}

#[test]
fn the_script_recomputes_the_root_for_every_leaf_in_the_log() {
    let Some(python) = python_or_skip("lys-anchor stranger verification") else {
        return;
    };

    let mut verified = 0;
    let mut path_lengths = Vec::new();
    for leaf_index in 0..TREE_SIZE {
        let case = build_case(leaf_index);
        let (code, stdout, stderr) = run(&python, &case, None);
        assert_eq!(
            code,
            Some(0),
            "the script refused a genuine proof for leaf {leaf_index}\n\
             stdout: {stdout}\nstderr: {stderr}"
        );
        // Keyed on what the script printed, not on its exit status: exit 0 is
        // what a script that did nothing at all also returns.
        assert!(
            stdout.contains("INCLUSION VERIFIED"),
            "the script exited 0 for leaf {leaf_index} without verifying anything\n\
             stdout: {stdout}"
        );
        path_lengths.push(case.artifact["hashes"].as_array().unwrap().len());
        verified += 1;
    }

    // Count what fired: a loop that ran zero times satisfies every assertion in
    // it, and a gate that never spawned looks exactly like one that passed.
    assert_eq!(verified, TREE_SIZE);

    // And the sweep covered paths of different depths rather than one shape
    // five times, which five agreements on a one-node path would also satisfy.
    path_lengths.sort_unstable();
    path_lengths.dedup();
    assert!(
        path_lengths.len() > 1,
        "every proof in the sweep had the same path length: {path_lengths:?}"
    );
}

#[test]
fn the_script_refuses_a_leaf_with_one_byte_changed() {
    let Some(python) = python_or_skip("lys-anchor stranger verification") else {
        return;
    };
    let case = build_case(3);

    // Keyed on the leaf's own first byte rather than on a literal this test
    // chose, so it stays a corruption whatever the fixture holds.
    let mut corrupted = case.leaf.clone();
    corrupted[0] ^= 0x01;
    assert_ne!(corrupted, case.leaf, "the corruption must change a byte");
    assert_eq!(corrupted.len(), case.leaf.len(), "only the byte may change");
    std::fs::write(&case.leaf_path, &corrupted).unwrap();

    assert_refused(&python, &case, None, "a leaf with one byte flipped");
}

#[test]
fn the_script_refuses_a_flipped_node_in_the_inclusion_path() {
    let Some(python) = python_or_skip("lys-anchor stranger verification") else {
        return;
    };
    let case = build_case(3);

    let mut value = case.artifact.clone();
    let hashes = value["hashes"].as_array_mut().unwrap();
    assert!(
        hashes.len() > 1,
        "leaf 3 of a five-leaf tree must have a multi-node path"
    );
    let node = hashes[0].as_str().unwrap();
    // One base64 character, which is six bits of the first byte. The character
    // is chosen against the node's own leading character, so this is a change
    // for any node the fixture happens to produce.
    let head = if node.starts_with('A') { 'B' } else { 'A' };
    let flipped = format!("{head}{}", &node[1..]);
    assert_ne!(flipped, node, "the flip must change the node");
    hashes[0] = serde_json::Value::String(flipped);
    rewrite_artifact(&case, &value);

    assert_refused(&python, &case, None, "a flipped node in the inclusion path");
}

#[test]
fn the_script_refuses_a_changed_leaf_index() {
    let Some(python) = python_or_skip("lys-anchor stranger verification") else {
        return;
    };
    let case = build_case(3);

    let mut value = case.artifact.clone();
    assert_eq!(value["leaf_index"].as_u64(), Some(3));
    // Index 2 is leaf 3's sibling, so the path is the same length and the same
    // nodes: only the sidedness of the innermost step changes. A verifier that
    // ignored the index entirely — hashing the path in a fixed order — would
    // still accept this, which is why the sibling is the case to use rather
    // than an index from another part of the tree.
    value["leaf_index"] = serde_json::Value::from(2u64);
    rewrite_artifact(&case, &value);

    assert_refused(&python, &case, None, "a changed leaf_index");
}

#[test]
fn the_script_refuses_a_tree_size_that_contradicts_the_checkpoint() {
    let Some(python) = python_or_skip("lys-anchor stranger verification") else {
        return;
    };
    let case = build_case(3);

    let mut value = case.artifact.clone();
    assert_eq!(value["tree_size"].as_u64(), Some(TREE_SIZE));
    // Six rather than four, and this is the whole case rather than a detail:
    // for leaf 3, tree sizes 5, 6, 7 and 8 share one RFC 6962 sidedness
    // sequence, so a five-leaf artifact relabelled `tree_size: 6` recomputes
    // the five-leaf root and matches the five-leaf checkpoint. Nothing in the
    // Merkle walk can notice. Only the comparison against the signed
    // checkpoint's own size refuses it — see the module docs. Four would have
    // been caught by the path length instead, and would have proven nothing
    // about this rule.
    value["tree_size"] = serde_json::Value::from(TREE_SIZE + 1);
    rewrite_artifact(&case, &value);

    assert_refused(
        &python,
        &case,
        None,
        "a tree_size the checkpoint contradicts",
    );
}

#[test]
fn the_script_refuses_an_artifact_carrying_an_unknown_field() {
    let Some(python) = python_or_skip("lys-anchor stranger verification") else {
        return;
    };
    let case = build_case(3);

    // `InclusionProofArtifact` is `deny_unknown_fields` (D2: unknown fields in a
    // v1 artifact are not valid v1). A stranger's verifier that shrugged at an
    // extra field would accept a shape `lys-core` itself rejects, and the two
    // would disagree about what a v1 artifact is.
    let mut value = case.artifact.clone();
    value["smuggled"] = serde_json::Value::from("not part of v1");
    rewrite_artifact(&case, &value);

    assert_refused(&python, &case, None, "an artifact with an unknown field");
}

#[test]
fn the_script_refuses_an_artifact_labelled_with_the_wrong_format() {
    let Some(python) = python_or_skip("lys-anchor stranger verification") else {
        return;
    };
    let case = build_case(3);

    // The consistency format, on an inclusion artifact: everything else about
    // the file is genuine and recomputes correctly, so only the kind check can
    // catch this. Cross-kind confusion is the attack the `format` field exists
    // to make impossible.
    let mut value = case.artifact.clone();
    value["format"] = serde_json::Value::from("lys/log-consistency-proof/v1");
    rewrite_artifact(&case, &value);

    assert_refused(
        &python,
        &case,
        None,
        "an artifact labelled as the wrong kind",
    );
}

#[test]
fn the_supplied_root_argument_decides_in_both_directions() {
    let Some(python) = python_or_skip("lys-anchor stranger verification") else {
        return;
    };
    let case = build_case(3);

    // 32 zero bytes: a root no SHA-256 tree produces, and demonstrably not this
    // log's. An argument the script silently ignored would pass this.
    let zero_root = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";
    assert_ne!(
        case.root_b64, zero_root,
        "the fixture root is not all zeros"
    );
    assert_refused(
        &python,
        &case,
        Some(zero_root),
        "a root that is not the log's",
    );

    // And the same argument accepts the real root, so the refusal above is the
    // check working rather than the argument always rejecting.
    let (code, stdout, stderr) = run(&python, &case, Some(&case.root_b64));
    assert_eq!(
        code,
        Some(0),
        "the script refused the log's own root\nstdout: {stdout}\nstderr: {stderr}"
    );
    assert!(stdout.contains("INCLUSION VERIFIED"), "stdout: {stdout}");
}
