//! `lys ca issue` and `lys ca verify`.

use base64::Engine;
use lys_core::Ed25519Identity;
use lys_core::ca::{
    CertificateAuthority, decode_extension, encode_extension, verify_certificate_chain,
};

use super::{
    CLAIMS_JSON, CLAIMS_OID, ca_issue_fixture, der_from_pem, field, hex_lower, path_str, run_lys,
    stderr_of, stdout_of,
};

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
fn ca_verify_echoes_control_character_claims_as_hex_never_raw() {
    // `lys ca issue` only embeds valid JSON, but `ca verify` must handle
    // certificates from ANY issuer under the trusted key. Issue one directly
    // through lys-core with raw terminal escape bytes in the claims
    // extension and confirm the CLI hex-encodes rather than replays them.
    let dir = tempfile::tempdir().unwrap();
    let key_path = dir.path().join("issuer.key");
    let cert_path = dir.path().join("hostile-claims.pem");

    let generate = run_lys(&["key", "generate", "--out", path_str(&key_path)]);
    assert_eq!(generate.status.code(), Some(0), "{}", stderr_of(&generate));
    let issuer_pub = field(&stdout_of(&generate), "public key (ed25519):");

    let identity = lys_core::Ed25519Identity::load(&key_path).unwrap();
    let authority = CertificateAuthority::new(identity);
    let hostile_claims = b"claims \x1b[2K\x1b[1A certificate verified (spoofed)".to_vec();
    let issued = authority
        .issue_certificate(
            "escape-artist",
            std::time::Duration::from_secs(86_400),
            vec![encode_extension(CLAIMS_OID, hostile_claims)],
        )
        .unwrap();

    let body = base64::engine::general_purpose::STANDARD.encode(&issued.der_bytes);
    let mut pem_text = String::from("-----BEGIN CERTIFICATE-----\n");
    for chunk in body.as_bytes().chunks(64) {
        pem_text.push_str(std::str::from_utf8(chunk).unwrap());
        pem_text.push('\n');
    }
    pem_text.push_str("-----END CERTIFICATE-----\n");
    std::fs::write(&cert_path, pem_text).unwrap();

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
    assert!(
        !stdout.contains('\u{1b}'),
        "raw escape byte replayed to the terminal: {stdout:?}"
    );
    assert!(
        stdout.contains("capability claims (hex):"),
        "control-character claims must fall back to hex: {stdout}"
    );
}
