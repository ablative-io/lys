#![cfg(test)]
//! End-to-end integration tests for the `lys log` command family.
//!
//! Every test drives the compiled binary through real process spawns
//! (`CARGO_BIN_EXE_lys`). The centrepiece is the third-party path: proofs
//! produced in one directory verify in a fresh directory holding ONLY the
//! artifacts, the proven leaf, and the verifier key string — never the log.
//! The tamper matrices assert the non-oracle discipline: every tamper class
//! within one artifact class produces the identical exit code AND stderr.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

#[path = "log_tests/proofs.rs"]
mod proofs;
#[path = "log_tests/vectors.rs"]
mod vectors;
#[path = "log_tests/writing.rs"]
mod writing;

/// Golden test seed from the design (32 ASCII bytes), shared with the
/// lys-core golden vectors and the Go conformance gate.
const GOLDEN_SEED: &[u8; 32] = b"lys-go-conformance-test-seed-01!";

/// Golden origin / note key name.
const GOLDEN_ORIGIN: &str = "example.com/lys/test";

/// Golden verifier key text form for (`GOLDEN_ORIGIN`, golden seed's pubkey).
const GOLDEN_VERIFIER_SPEC: &str =
    "example.com/lys/test+52580cd9+AQz9D9gbFqzLxSMM9Fy6nUuTfYJ8bI29RKFE5aulcbni";

/// Golden signed note over the size-3 checkpoint body (byte-identical to Go
/// `note.Sign` output; the primary copy is pinned in the lys-core tests).
const GOLDEN_NOTE: &str = "example.com/lys/test\n3\nz3Y6BByBzu8VeKYIP3XGG+8uABTyo+aDqX/Pylvn8Zo=\n\n\u{2014} example.com/lys/test UlgM2S4MVZwL9PUGADbPhidG6yKCC0hCE+sx7iXFboC6/rex00vtEy4d33ODa1g0afYmx36opQUAXnwdUl9E7eE28QU=\n";

/// Golden RFC 6962 leaf hash of the raw bytes `leaf-0`.
const GOLDEN_LEAF0_HASH: &str = "305df59f9590c3c9ac63d2b2743c388e3792449078cebf7fb3dbe6471643b2b7";

/// Golden root at size 3 over leaves `leaf-0`, `leaf-1`, `leaf-2`.
const GOLDEN_ROOT3_HEX: &str = "cf763a041c81ceef1578a6083f75c61bef2e0014f2a3e683a97fcfca5be7f19a";

/// The empty tree's root: SHA-256 of the empty string.
const EMPTY_ROOT_HEX: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

/// Exact stderr of a failed `lys log verify inclusion` (crypto class).
const INCLUSION_FAIL_STDERR: &str =
    "error: inclusion proof verification failed: invalid artifact, checkpoint, or leaf\n";

/// Exact stderr of a failed `lys log verify consistency` (crypto class).
const CONSISTENCY_FAIL_STDERR: &str =
    "error: consistency proof verification failed: invalid artifact or checkpoints\n";

fn run_lys(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_lys"))
        .args(args)
        .output()
        .expect("failed to spawn lys binary")
}

fn stdout_of(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("stdout was not UTF-8")
}

fn stderr_of(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("stderr was not UTF-8")
}

/// Extract the value following `label` on the matching stdout line.
fn field(stdout: &str, label: &str) -> String {
    stdout
        .lines()
        .find_map(|line| line.strip_prefix(label))
        .unwrap_or_else(|| panic!("no line starting with {label:?} in output:\n{stdout}"))
        .trim()
        .to_string()
}

/// Lowercase hex encoding, mirroring the CLI's output format.
fn hex_lower(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.write_fmt(format_args!("{b:02x}"))
            .expect("writing to a String cannot fail");
    }
    s
}

fn path_str(path: &Path) -> &str {
    path.to_str().expect("tempdir path was not UTF-8")
}

fn assert_success(output: &Output) {
    assert_eq!(output.status.code(), Some(0), "{}", stderr_of(output));
}

/// A fully built log: key, three leaves, checkpoint, and both artifacts.
struct ProvenLog {
    _tmp: tempfile::TempDir,
    dir: PathBuf,
    key: PathBuf,
    leaf_files: Vec<PathBuf>,
    verifier: String,
    checkpoint_file: PathBuf,
    checkpoint_stdout: String,
    append_stdouts: Vec<String>,
    inclusion_artifact: PathBuf,
    consistency_artifact: PathBuf,
    /// Every stdout/stderr captured while building, for leak checks.
    transcripts: Vec<String>,
}

/// Builds a log with three leaves and produces a checkpoint, an inclusion
/// artifact for leaf 1, and a consistency artifact from size 2 to 3.
///
/// `seed`: `Some(bytes)` writes that exact key file (golden vectors);
/// `None` runs `lys key generate`.
fn build_proven_log(origin: &str, seed: Option<&[u8; 32]>) -> ProvenLog {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("log");
    let key = tmp.path().join("operator.key");
    let mut transcripts = Vec::new();
    if let Some(bytes) = seed {
        std::fs::write(&key, bytes).unwrap();
    } else {
        let generate = run_lys(&["key", "generate", "--out", path_str(&key)]);
        assert_success(&generate);
        transcripts.push(stdout_of(&generate));
        transcripts.push(stderr_of(&generate));
    }
    let init = run_lys(&["log", "init", "--dir", path_str(&dir), "--origin", origin]);
    assert_success(&init);
    transcripts.push(stdout_of(&init));
    transcripts.push(stderr_of(&init));
    let mut leaf_files = Vec::new();
    let mut append_stdouts = Vec::new();
    for i in 0..3u32 {
        let leaf = tmp.path().join(format!("leaf-{i}.bin"));
        std::fs::write(&leaf, format!("leaf-{i}")).unwrap();
        let append = run_lys(&[
            "log",
            "append",
            "--dir",
            path_str(&dir),
            "--leaf",
            path_str(&leaf),
        ]);
        assert_success(&append);
        append_stdouts.push(stdout_of(&append));
        transcripts.push(stdout_of(&append));
        transcripts.push(stderr_of(&append));
        leaf_files.push(leaf);
    }
    let checkpoint_file = tmp.path().join("checkpoint.note");
    let checkpoint = run_lys(&[
        "log",
        "checkpoint",
        "--dir",
        path_str(&dir),
        "--key",
        path_str(&key),
        "--out",
        path_str(&checkpoint_file),
    ]);
    assert_success(&checkpoint);
    let checkpoint_stdout = stdout_of(&checkpoint);
    let verifier = field(&checkpoint_stdout, "verifier key (signed-note):");
    transcripts.push(stdout_of(&checkpoint));
    transcripts.push(stderr_of(&checkpoint));
    let inclusion_artifact = tmp.path().join("inclusion.json");
    let prove_inclusion = run_lys(&[
        "log",
        "prove",
        "inclusion",
        "--dir",
        path_str(&dir),
        "--key",
        path_str(&key),
        "--leaf-index",
        "1",
        "--out",
        path_str(&inclusion_artifact),
    ]);
    assert_success(&prove_inclusion);
    transcripts.push(stdout_of(&prove_inclusion));
    transcripts.push(stderr_of(&prove_inclusion));
    let consistency_artifact = tmp.path().join("consistency.json");
    let prove_consistency = run_lys(&[
        "log",
        "prove",
        "consistency",
        "--dir",
        path_str(&dir),
        "--key",
        path_str(&key),
        "--old-size",
        "2",
        "--out",
        path_str(&consistency_artifact),
    ]);
    assert_success(&prove_consistency);
    transcripts.push(stdout_of(&prove_consistency));
    transcripts.push(stderr_of(&prove_consistency));
    ProvenLog {
        _tmp: tmp,
        dir,
        key,
        leaf_files,
        verifier,
        checkpoint_file,
        checkpoint_stdout,
        append_stdouts,
        inclusion_artifact,
        consistency_artifact,
        transcripts,
    }
}

/// Writes `value` as JSON to a fresh file under `dir` and returns the path.
fn write_json(dir: &Path, name: &str, value: &serde_json::Value) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, serde_json::to_string_pretty(value).unwrap()).unwrap();
    path
}

fn load_json(path: &Path) -> serde_json::Value {
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}
