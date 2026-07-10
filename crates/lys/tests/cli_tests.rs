//! End-to-end integration tests for the `lys` binary.
//!
//! Each subcommand is exercised through a real process spawn of the compiled
//! binary (`CARGO_BIN_EXE_lys`), asserting on exit codes, stdout/stderr
//! content, and on-disk side effects. The attest/verify tests additionally
//! cross-check the CLI's JSON envelope against `lys-core` directly.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;
use std::process::{Command, Output};

use base64::Engine;
use lys_core::Ed25519Identity;
use lys_core::attestation::{Attestation, verify_attestation};
use lys_core::ca::{decode_extension, verify_certificate_chain};

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

// ------------------------------------------------------------------- ca issue

/// Capability claims used across the CA tests.
const CLAIMS_JSON: &str = r#"{"capabilities":["deploy","sign"],"scope":"ci"}"#;

/// The OID the CLI documents for capability-claims extensions
/// (`LYS_OID_ARC` + `1`).
const CLAIMS_OID: &[u64] = &[1, 3, 6, 1, 4, 1, 58888, 1];

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

/// Generate an issuer key and issue a certificate with the standard claims,
/// returning the cert path and the issuer public key hex.
fn ca_issue_fixture(dir: &Path, validity_days: &str) -> (std::path::PathBuf, String) {
    let key_path = dir.join("issuer.key");
    let claims_path = dir.join("claims.json");
    let cert_path = dir.join("subject.pem");

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
    ]);
    assert_eq!(issue.status.code(), Some(0), "{}", stderr_of(&issue));
    (cert_path, issuer_pub)
}

#[test]
fn ca_issue_writes_pem_certificate_that_lys_core_verifies() {
    let dir = tempfile::tempdir().unwrap();
    let key_path = dir.path().join("issuer.key");
    let claims_path = dir.path().join("claims.json");
    let cert_path = dir.path().join("subject.pem");

    let generate = run_lys(&["key", "generate", "--out", path_str(&key_path)]);
    assert_eq!(generate.status.code(), Some(0), "{}", stderr_of(&generate));
    std::fs::write(&claims_path, CLAIMS_JSON).unwrap();

    let output = run_lys(&[
        "ca",
        "issue",
        "--key",
        path_str(&key_path),
        "--subject",
        "agent-under-test",
        "--claims",
        path_str(&claims_path),
        "--validity-days",
        "1",
        "--out",
        path_str(&cert_path),
    ]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr_of(&output));
    let stdout = stdout_of(&output);

    // The written file is PEM whose DER verifies through lys-core directly
    // against the issuer key, and carries the claims byte-for-byte under the
    // documented OID.
    let pem_text = std::fs::read_to_string(&cert_path).unwrap();
    assert!(pem_text.starts_with("-----BEGIN CERTIFICATE-----"));
    let der = der_from_pem(&pem_text);
    let identity = Ed25519Identity::load_or_generate(&key_path).unwrap();
    verify_certificate_chain(&der, &identity.public_key_bytes()).unwrap();
    assert_eq!(
        decode_extension(&der, CLAIMS_OID).unwrap(),
        Some(CLAIMS_JSON.as_bytes().to_vec())
    );

    // Printed metadata is public-only and consistent with the issuer key.
    assert_eq!(
        field(&stdout, "issuer public key (ed25519):"),
        hex_lower(&identity.public_key_bytes())
    );
    assert_eq!(field(&stdout, "subject public key (ed25519):").len(), 64);
    assert_eq!(field(&stdout, "fingerprint (sha256):").len(), 64);

    // The issuer seed never leaks, and no subject key file is minted — the
    // only files in the directory are the ones this test created plus the
    // certificate.
    let seed_hex = hex_lower(&std::fs::read(&key_path).unwrap());
    assert!(
        !stdout.contains(&seed_hex),
        "private seed leaked into stdout"
    );
    assert!(
        !pem_text.contains(&seed_hex),
        "private seed leaked into certificate"
    );
    let mut entries: Vec<String> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    entries.sort();
    assert_eq!(
        entries,
        vec!["claims.json", "issuer.key", "subject.pem"],
        "ca issue must not create extra files (e.g. a subject key)"
    );
}

#[test]
fn ca_issue_with_missing_key_fails_and_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let key_path = dir.path().join("absent.key");
    let cert_path = dir.path().join("subject.pem");

    let output = run_lys(&[
        "ca",
        "issue",
        "--key",
        path_str(&key_path),
        "--subject",
        "agent-under-test",
        "--validity-days",
        "1",
        "--out",
        path_str(&cert_path),
    ]);
    assert_eq!(output.status.code(), Some(1));
    let stderr = stderr_of(&output);
    assert!(
        stderr.contains("identity key file not found"),
        "stderr: {stderr}"
    );
    assert!(
        !cert_path.exists(),
        "no certificate may be written on failure"
    );
    assert!(
        !key_path.exists(),
        "ca issue must never create a key file as a side effect"
    );
}

#[test]
fn ca_issue_rejects_malformed_claims_json_and_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let key_path = dir.path().join("issuer.key");
    let claims_path = dir.path().join("claims.json");
    let cert_path = dir.path().join("subject.pem");

    let generate = run_lys(&["key", "generate", "--out", path_str(&key_path)]);
    assert_eq!(generate.status.code(), Some(0), "{}", stderr_of(&generate));
    std::fs::write(&claims_path, b"{ not json ]").unwrap();

    let output = run_lys(&[
        "ca",
        "issue",
        "--key",
        path_str(&key_path),
        "--subject",
        "agent-under-test",
        "--claims",
        path_str(&claims_path),
        "--validity-days",
        "1",
        "--out",
        path_str(&cert_path),
    ]);
    assert_eq!(output.status.code(), Some(1));
    let stderr = stderr_of(&output);
    assert!(
        stderr.contains("capability claims JSON"),
        "stderr: {stderr}"
    );
    assert!(
        !cert_path.exists(),
        "no certificate may be written on failure"
    );
}

// ------------------------------------------------------------------ ca verify

#[test]
fn ca_verify_accepts_valid_certificate_with_exit_zero() {
    let dir = tempfile::tempdir().unwrap();
    let (cert_path, issuer_pub) = ca_issue_fixture(dir.path(), "1");

    let output = run_lys(&[
        "ca",
        "verify",
        "--cert",
        path_str(&cert_path),
        "--issuer-public-key",
        &issuer_pub,
    ]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr_of(&output));
    let stdout = stdout_of(&output);
    assert!(stdout.contains("certificate verified"), "{stdout}");
    assert_eq!(field(&stdout, "issuer public key (ed25519):"), issuer_pub);
    assert_eq!(field(&stdout, "capability claims:"), CLAIMS_JSON);
}

#[test]
fn ca_verify_accepts_explicit_instant_inside_the_window() {
    let dir = tempfile::tempdir().unwrap();
    let (cert_path, issuer_pub) = ca_issue_fixture(dir.path(), "2");
    let inside = (chrono::Utc::now() + chrono::Duration::days(1)).to_rfc3339();

    let output = run_lys(&[
        "ca",
        "verify",
        "--cert",
        path_str(&cert_path),
        "--issuer-public-key",
        &issuer_pub,
        "--at",
        &inside,
    ]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr_of(&output));
    assert!(
        stdout_of(&output).contains("certificate verified"),
        "{}",
        stdout_of(&output)
    );
}

#[test]
fn ca_verify_failures_collapse_to_one_generic_message() {
    let dir = tempfile::tempdir().unwrap();
    let (cert_path, issuer_pub) = ca_issue_fixture(dir.path(), "1");

    // A different (untrusted) issuer key.
    let other_key = dir.path().join("other.key");
    let generate = run_lys(&["key", "generate", "--out", path_str(&other_key)]);
    assert_eq!(generate.status.code(), Some(0), "{}", stderr_of(&generate));
    let wrong_pub = field(&stdout_of(&generate), "public key (ed25519):");

    let before_window = "2000-01-01T00:00:00Z".to_string();
    let after_window = (chrono::Utc::now() + chrono::Duration::days(400)).to_rfc3339();

    let cases: Vec<Vec<&str>> = vec![
        // Wrong issuer key at a valid instant.
        vec![
            "ca",
            "verify",
            "--cert",
            path_str(&cert_path),
            "--issuer-public-key",
            &wrong_pub,
        ],
        // Right issuer key, before the validity window.
        vec![
            "ca",
            "verify",
            "--cert",
            path_str(&cert_path),
            "--issuer-public-key",
            &issuer_pub,
            "--at",
            &before_window,
        ],
        // Right issuer key, after the validity window.
        vec![
            "ca",
            "verify",
            "--cert",
            path_str(&cert_path),
            "--issuer-public-key",
            &issuer_pub,
            "--at",
            &after_window,
        ],
    ];

    let mut messages = Vec::new();
    for args in &cases {
        let output = run_lys(args);
        assert_eq!(output.status.code(), Some(1), "args: {args:?}");
        let stderr = stderr_of(&output);
        assert!(
            stderr.contains("certificate verification failed"),
            "stderr: {stderr}"
        );
        assert!(
            !stdout_of(&output).contains("certificate verified"),
            "must not claim success"
        );
        messages.push(stderr);
    }
    // Non-oracle: wrong key, not-yet-valid, and expired must all be
    // indistinguishable from the caller's side.
    assert_eq!(messages[0], messages[1]);
    assert_eq!(messages[1], messages[2]);
}

#[test]
fn ca_verify_rejects_malformed_at_timestamp() {
    let dir = tempfile::tempdir().unwrap();
    let (cert_path, issuer_pub) = ca_issue_fixture(dir.path(), "1");

    let output = run_lys(&[
        "ca",
        "verify",
        "--cert",
        path_str(&cert_path),
        "--issuer-public-key",
        &issuer_pub,
        "--at",
        "yesterday at noon",
    ]);
    assert_eq!(output.status.code(), Some(1));
    let stderr = stderr_of(&output);
    assert!(stderr.contains("invalid timestamp"), "stderr: {stderr}");
    assert!(stderr.contains("RFC 3339"), "stderr: {stderr}");
}

#[test]
fn ca_verify_rejects_invalid_issuer_public_key_hex() {
    let dir = tempfile::tempdir().unwrap();
    let (cert_path, _) = ca_issue_fixture(dir.path(), "1");

    for bad in ["zz", "abc123", &"ab".repeat(33)] {
        let output = run_lys(&[
            "ca",
            "verify",
            "--cert",
            path_str(&cert_path),
            "--issuer-public-key",
            bad,
        ]);
        assert_eq!(output.status.code(), Some(1), "input: {bad}");
        assert!(
            stderr_of(&output).contains("invalid issuer public key"),
            "stderr: {}",
            stderr_of(&output)
        );
    }
}

#[test]
fn ca_verify_rejects_non_pem_certificate_file() {
    let dir = tempfile::tempdir().unwrap();
    let cert_path = dir.path().join("bogus.pem");
    std::fs::write(&cert_path, b"this is not a certificate").unwrap();

    let output = run_lys(&[
        "ca",
        "verify",
        "--cert",
        path_str(&cert_path),
        "--issuer-public-key",
        &"ab".repeat(32),
    ]);
    assert_eq!(output.status.code(), Some(1));
    let stderr = stderr_of(&output);
    assert!(
        stderr.contains("failed to parse PEM certificate"),
        "stderr: {stderr}"
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
