#![cfg(test)]
//! End-to-end integration tests for the `lys` binary.
//!
//! Each subcommand is exercised through a real process spawn of the compiled
//! binary (`CARGO_BIN_EXE_lys`), asserting on exit codes, stdout/stderr
//! content, and on-disk side effects. The attest/verify tests additionally
//! cross-check the CLI's `COSE_Sign1` artifact against `lys-core` directly.

use std::path::Path;
use std::process::{Command, Output};

use base64::Engine;

#[path = "cli_tests/attest.rs"]
mod attest;
#[path = "cli_tests/ca.rs"]
mod ca;
#[path = "cli_tests/inspect.rs"]
mod inspect;
#[path = "cli_tests/key.rs"]
mod key;
#[path = "cli_tests/open.rs"]
mod open;
#[path = "cli_tests/seal.rs"]
mod seal;

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
        s.write_fmt(format_args!("{b:02x}"))
            .expect("writing to a String cannot fail");
    }
    s
}

fn path_str(path: &Path) -> &str {
    path.to_str().expect("tempdir path was not UTF-8")
}

// --------------------------------------------------------------------- verify

/// Run the full generate → attest pipeline, returning the attestation path.
fn attest_fixture(dir: &Path, payload: &[u8]) -> std::path::PathBuf {
    let key_path = dir.join("agent.key");
    let payload_path = dir.join("payload.bin");
    let out_path = dir.join("attestation.cose");

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

// ------------------------------------------------------------------- ca issue

/// Capability claims used across the CA tests.
const CLAIMS_JSON: &str = r#"{"capabilities":["deploy","sign"],"scope":"ci"}"#;

/// The OID the CLI documents for capability-claims extensions
/// (`LYS_OID_ARC` + `1`).
const CLAIMS_OID: &[u64] = &[1, 3, 6, 1, 4, 1, 66364, 1];

/// Strip PEM framing and base64-decode the certificate body.
fn der_from_pem(pem_text: &str) -> Vec<u8> {
    let body: String = pem_text
        .lines()
        .filter(|line| !line.starts_with("-----"))
        .collect();
    base64::engine::general_purpose::STANDARD
        .decode(body)
        .expect("PEM body was not valid base64")
}

/// Initialize a test log at `dir/log`, the log every `lys ca issue` enters
/// its certificate in, and return its path.
fn init_test_log(dir: &Path) -> std::path::PathBuf {
    let log_dir = dir.join("log");
    let init = run_lys(&[
        "log",
        "init",
        "--dir",
        path_str(&log_dir),
        "--origin",
        "example.com/lys/issuance",
    ]);
    assert_eq!(init.status.code(), Some(0), "{}", stderr_of(&init));
    log_dir
}

/// The number of leaves in the test log at `log_dir`.
fn log_leaf_count(log_dir: &Path) -> usize {
    std::fs::read_dir(log_dir.join("leaves")).unwrap().count()
}

/// Generate an issuer key and issue a certificate with the standard claims,
/// returning the cert path and the issuer public key hex.
fn ca_issue_fixture(dir: &Path, validity_days: &str) -> (std::path::PathBuf, String) {
    let key_path = dir.join("issuer.key");
    let claims_path = dir.join("claims.json");
    let cert_path = dir.join("subject.pem");
    let log_dir = init_test_log(dir);
    let leaf_path = dir.join("subject.leaf");

    let generate = run_lys(&["key", "generate", "--out", path_str(&key_path)]);
    assert_eq!(generate.status.code(), Some(0), "{}", stderr_of(&generate));
    let issuer_pub = field(&stdout_of(&generate), "public key (ed25519):");
    std::fs::write(&claims_path, CLAIMS_JSON).unwrap();

    let issue = run_lys(&[
        "ca",
        "issue",
        "--key",
        path_str(&key_path),
        "--subject",
        "agent-under-test",
        "--claims",
        path_str(&claims_path),
        "--validity-days",
        validity_days,
        "--out",
        path_str(&cert_path),
        "--log",
        path_str(&log_dir),
        "--leaf-out",
        path_str(&leaf_path),
    ]);
    assert_eq!(issue.status.code(), Some(0), "{}", stderr_of(&issue));
    (cert_path, issuer_pub)
}

// ----------------------------------------------------------------- seal / open

/// Everything `seal_fixture` produces, so tests can pick what they need.
struct SealFixture {
    sender_key: std::path::PathBuf,
    recipient_key: std::path::PathBuf,
    payload_path: std::path::PathBuf,
    envelope_path: std::path::PathBuf,
    attestation_path: std::path::PathBuf,
    sender_pub: String,
    recipient_x25519_pub: String,
}

/// Generate sender and recipient keys, then run `lys seal` end to end.
fn seal_fixture(dir: &Path, payload: &[u8]) -> SealFixture {
    let sender_key = dir.join("sender.key");
    let recipient_key = dir.join("recipient.key");
    let payload_path = dir.join("payload.bin");
    let envelope_path = dir.join("envelope.json");
    let attestation_path = dir.join("seal-attestation.cose");

    let generate_sender = run_lys(&["key", "generate", "--out", path_str(&sender_key)]);
    assert_eq!(
        generate_sender.status.code(),
        Some(0),
        "{}",
        stderr_of(&generate_sender)
    );
    let sender_pub = field(&stdout_of(&generate_sender), "public key (ed25519):");

    let generate_recipient = run_lys(&["key", "generate", "--out", path_str(&recipient_key)]);
    assert_eq!(
        generate_recipient.status.code(),
        Some(0),
        "{}",
        stderr_of(&generate_recipient)
    );
    let inspect = run_lys(&["key", "inspect", "--key", path_str(&recipient_key)]);
    assert_eq!(inspect.status.code(), Some(0), "{}", stderr_of(&inspect));
    let recipient_x25519_pub = field(&stdout_of(&inspect), "public key (x25519):");

    std::fs::write(&payload_path, payload).unwrap();
    let seal = run_lys(&[
        "seal",
        "--key",
        path_str(&sender_key),
        "--recipient-public-key",
        &recipient_x25519_pub,
        "--payload",
        path_str(&payload_path),
        "--out",
        path_str(&envelope_path),
        "--attestation-out",
        path_str(&attestation_path),
    ]);
    assert_eq!(seal.status.code(), Some(0), "{}", stderr_of(&seal));

    SealFixture {
        sender_key,
        recipient_key,
        payload_path,
        envelope_path,
        attestation_path,
        sender_pub,
        recipient_x25519_pub,
    }
}
