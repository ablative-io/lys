//! End-to-end integration tests for the `lys` binary.
//!
//! Each subcommand is exercised through a real process spawn of the compiled
//! binary (`CARGO_BIN_EXE_lys`), asserting on exit codes, stdout/stderr
//! content, and on-disk side effects. The attest/verify tests additionally
//! cross-check the CLI's JSON envelope against `lys-core` directly.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;
use std::process::{Command, Output};

use lys_core::Ed25519Identity;
use lys_core::attestation::{Attestation, verify_attestation};

/// Spawn the compiled `lys` binary with the given arguments.
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
        let _ = s.write_fmt(format_args!("{b:02x}"));
    }
    s
}

fn path_str(path: &Path) -> &str {
    path.to_str().expect("tempdir path was not UTF-8")
}

// ---------------------------------------------------------------- key generate

#[test]
fn key_generate_creates_key_file_and_prints_public_hex_only() {
    let dir = tempfile::tempdir().unwrap();
    let key_path = dir.path().join("agent.key");

    let output = run_lys(&["key", "generate", "--out", path_str(&key_path)]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr_of(&output));

    let stdout = stdout_of(&output);
    assert!(stdout.contains("generated new identity key"), "{stdout}");
    let pub_hex = field(&stdout, "public key (ed25519):");
    assert_eq!(pub_hex.len(), 64, "expected 32-byte hex, got: {pub_hex}");
    assert!(pub_hex.chars().all(|c| c.is_ascii_hexdigit()));

    // The key file holds exactly the 32-byte seed, and no encoding of that
    // seed ever appears in the command output.
    let seed = std::fs::read(&key_path).unwrap();
    assert_eq!(seed.len(), 32);
    let seed_hex = hex_lower(&seed);
    assert!(
        !stdout.contains(&seed_hex),
        "private seed leaked into stdout"
    );
    assert!(
        !stderr_of(&output).contains(&seed_hex),
        "private seed leaked into stderr"
    );

    // The printed hex is the real public key for the persisted seed.
    let identity = Ed25519Identity::load_or_generate(&key_path).unwrap();
    assert_eq!(pub_hex, hex_lower(&identity.public_key_bytes()));

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&key_path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600, "key file mode was {:o}", mode & 0o777);
    }
}

#[test]
fn key_generate_is_idempotent_and_reports_existing_key() {
    let dir = tempfile::tempdir().unwrap();
    let key_path = dir.path().join("agent.key");

    let first = run_lys(&["key", "generate", "--out", path_str(&key_path)]);
    assert_eq!(first.status.code(), Some(0), "{}", stderr_of(&first));
    let first_pub = field(&stdout_of(&first), "public key (ed25519):");
    let seed_before = std::fs::read(&key_path).unwrap();

    let second = run_lys(&["key", "generate", "--out", path_str(&key_path)]);
    assert_eq!(second.status.code(), Some(0), "{}", stderr_of(&second));
    let second_stdout = stdout_of(&second);
    assert!(
        second_stdout.contains("loaded existing identity key"),
        "{second_stdout}"
    );
    assert_eq!(field(&second_stdout, "public key (ed25519):"), first_pub);
    assert_eq!(
        std::fs::read(&key_path).unwrap(),
        seed_before,
        "second generate must not rewrite the key file"
    );
}

// ---------------------------------------------------------------- key inspect

#[test]
fn key_inspect_prints_ed25519_and_derived_x25519_public_keys() {
    let dir = tempfile::tempdir().unwrap();
    let key_path = dir.path().join("agent.key");
    let generate = run_lys(&["key", "generate", "--out", path_str(&key_path)]);
    assert_eq!(generate.status.code(), Some(0), "{}", stderr_of(&generate));
    let generated_pub = field(&stdout_of(&generate), "public key (ed25519):");

    let output = run_lys(&["key", "inspect", "--key", path_str(&key_path)]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr_of(&output));
    let stdout = stdout_of(&output);

    assert_eq!(field(&stdout, "public key (ed25519):"), generated_pub);

    let identity = Ed25519Identity::load_or_generate(&key_path).unwrap();
    assert_eq!(
        field(&stdout, "public key (x25519):"),
        hex_lower(&identity.x25519_public_key())
    );

    let seed_hex = hex_lower(&std::fs::read(&key_path).unwrap());
    assert!(
        !stdout.contains(&seed_hex),
        "private seed leaked into stdout"
    );
}

#[test]
fn key_inspect_missing_file_fails_without_creating_one() {
    let dir = tempfile::tempdir().unwrap();
    let key_path = dir.path().join("absent.key");

    let output = run_lys(&["key", "inspect", "--key", path_str(&key_path)]);
    assert_eq!(output.status.code(), Some(1));
    let stderr = stderr_of(&output);
    assert!(
        stderr.contains("identity key file not found"),
        "stderr: {stderr}"
    );
    assert!(
        !key_path.exists(),
        "inspect must never create a key file as a side effect"
    );
}

// --------------------------------------------------------------------- attest

#[test]
fn attest_writes_json_envelope_that_lys_core_verifies() {
    let dir = tempfile::tempdir().unwrap();
    let key_path = dir.path().join("agent.key");
    let payload_path = dir.path().join("payload.bin");
    let out_path = dir.path().join("attestation.json");
    let payload: &[u8] = b"execution receipt: task 42 completed";

    let generate = run_lys(&["key", "generate", "--out", path_str(&key_path)]);
    assert_eq!(generate.status.code(), Some(0), "{}", stderr_of(&generate));
    std::fs::write(&payload_path, payload).unwrap();

    let output = run_lys(&[
        "attest",
        "--key",
        path_str(&key_path),
        "--payload",
        path_str(&payload_path),
        "--out",
        path_str(&out_path),
    ]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr_of(&output));
    let stdout = stdout_of(&output);

    // The written envelope is valid JSON in the lys-core wire shape and
    // verifies against the payload through the library directly.
    let json = std::fs::read_to_string(&out_path).unwrap();
    let attestation: Attestation = serde_json::from_str(&json).unwrap();
    verify_attestation(&attestation, payload).unwrap();

    // Printed metadata matches the envelope on disk.
    let identity = Ed25519Identity::load_or_generate(&key_path).unwrap();
    assert_eq!(attestation.signer_public_key, identity.public_key_bytes());
    assert_eq!(
        field(&stdout, "payload hash (sha256):"),
        hex_lower(&attestation.payload_hash)
    );
    assert_eq!(
        field(&stdout, "signer public key (ed25519):"),
        hex_lower(&attestation.signer_public_key)
    );
    assert_eq!(
        field(&stdout, "signed at (unix ms):"),
        attestation.timestamp.to_string()
    );

    let seed_hex = hex_lower(&std::fs::read(&key_path).unwrap());
    assert!(
        !json.contains(&seed_hex),
        "private seed leaked into envelope"
    );
    assert!(
        !stdout.contains(&seed_hex),
        "private seed leaked into stdout"
    );
}

#[test]
fn attest_with_missing_key_fails_and_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let payload_path = dir.path().join("payload.bin");
    let out_path = dir.path().join("attestation.json");
    std::fs::write(&payload_path, b"payload").unwrap();

    let output = run_lys(&[
        "attest",
        "--key",
        path_str(&dir.path().join("absent.key")),
        "--payload",
        path_str(&payload_path),
        "--out",
        path_str(&out_path),
    ]);
    assert_eq!(output.status.code(), Some(1));
    let stderr = stderr_of(&output);
    assert!(
        stderr.contains("identity key file not found"),
        "stderr: {stderr}"
    );
    assert!(
        !out_path.exists(),
        "no attestation may be written on failure"
    );
    assert!(
        !dir.path().join("absent.key").exists(),
        "attest must never create a key file as a side effect"
    );
}

// --------------------------------------------------------------------- verify

/// Run the full generate → attest pipeline, returning the attestation path.
fn attest_fixture(dir: &Path, payload: &[u8]) -> std::path::PathBuf {
    let key_path = dir.join("agent.key");
    let payload_path = dir.join("payload.bin");
    let out_path = dir.join("attestation.json");

    let generate = run_lys(&["key", "generate", "--out", path_str(&key_path)]);
    assert_eq!(generate.status.code(), Some(0), "{}", stderr_of(&generate));
    std::fs::write(&payload_path, payload).unwrap();
    let attest = run_lys(&[
        "attest",
        "--key",
        path_str(&key_path),
        "--payload",
        path_str(&payload_path),
        "--out",
        path_str(&out_path),
    ]);
    assert_eq!(attest.status.code(), Some(0), "{}", stderr_of(&attest));
    out_path
}

#[test]
fn verify_accepts_valid_attestation_with_exit_zero() {
    let dir = tempfile::tempdir().unwrap();
    let out_path = attest_fixture(dir.path(), b"audit entry payload");

    let output = run_lys(&[
        "verify",
        "--attestation",
        path_str(&out_path),
        "--payload",
        path_str(&dir.path().join("payload.bin")),
    ]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr_of(&output));
    let stdout = stdout_of(&output);
    assert!(stdout.contains("attestation verified"), "{stdout}");
    assert_eq!(field(&stdout, "signer public key (ed25519):").len(), 64);
}

#[test]
fn verify_rejects_tampered_payload_with_exit_one() {
    let dir = tempfile::tempdir().unwrap();
    let out_path = attest_fixture(dir.path(), b"original payload");
    let payload_path = dir.path().join("payload.bin");
    std::fs::write(&payload_path, b"tampered payload").unwrap();

    let output = run_lys(&[
        "verify",
        "--attestation",
        path_str(&out_path),
        "--payload",
        path_str(&payload_path),
    ]);
    assert_eq!(output.status.code(), Some(1));
    let stderr = stderr_of(&output);
    assert!(
        stderr.contains("attestation verification failed"),
        "stderr: {stderr}"
    );
    let stdout = stdout_of(&output);
    assert!(
        !stdout.contains("attestation verified"),
        "must not claim success: {stdout}"
    );
}

#[test]
fn verify_rejects_tampered_timestamp_with_exit_one() {
    let dir = tempfile::tempdir().unwrap();
    let out_path = attest_fixture(dir.path(), b"timestamped payload");

    // Shift the (authenticated) timestamp by one millisecond in the JSON.
    let json = std::fs::read_to_string(&out_path).unwrap();
    let mut envelope: serde_json::Value = serde_json::from_str(&json).unwrap();
    let timestamp = envelope["timestamp"].as_i64().unwrap();
    envelope["timestamp"] = serde_json::Value::from(timestamp + 1);
    std::fs::write(&out_path, serde_json::to_string(&envelope).unwrap()).unwrap();

    let output = run_lys(&[
        "verify",
        "--attestation",
        path_str(&out_path),
        "--payload",
        path_str(&dir.path().join("payload.bin")),
    ]);
    assert_eq!(output.status.code(), Some(1));
    assert!(
        stderr_of(&output).contains("attestation verification failed"),
        "stderr: {}",
        stderr_of(&output)
    );
}

#[test]
fn verify_rejects_malformed_attestation_json_with_exit_one() {
    let dir = tempfile::tempdir().unwrap();
    let attestation_path = dir.path().join("attestation.json");
    let payload_path = dir.path().join("payload.bin");
    std::fs::write(&attestation_path, b"{ not json ]").unwrap();
    std::fs::write(&payload_path, b"payload").unwrap();

    let output = run_lys(&[
        "verify",
        "--attestation",
        path_str(&attestation_path),
        "--payload",
        path_str(&payload_path),
    ]);
    assert_eq!(output.status.code(), Some(1));
    let stderr = stderr_of(&output);
    assert!(
        stderr.contains("failed to parse attestation JSON"),
        "stderr: {stderr}"
    );
}
