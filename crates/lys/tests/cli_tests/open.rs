//! `lys open`, and the one generic failure a malformed attestation collapses into.

use super::{
    attest_fixture, field, hex_lower, path_str, run_lys, seal_fixture, stderr_of, stdout_of,
};

#[cfg(unix)]
#[test]
fn open_writes_plaintext_owner_readable_only() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::tempdir().unwrap();
    let fixture = seal_fixture(dir.path(), b"confidential payload");
    let out_path = dir.path().join("opened.bin");

    let output = run_lys(&[
        "open",
        "--key",
        path_str(&fixture.recipient_key),
        "--sender-public-key",
        &fixture.sender_pub,
        "--envelope",
        path_str(&fixture.envelope_path),
        "--attestation",
        path_str(&fixture.attestation_path),
        "--out",
        path_str(&out_path),
    ]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr_of(&output));

    let mode = std::fs::metadata(&out_path).unwrap().permissions().mode();
    assert_eq!(
        mode & 0o777,
        0o600,
        "recovered plaintext must be owner-readable only"
    );
}

#[test]
fn open_recovers_payload_without_printing_plaintext() {
    let dir = tempfile::tempdir().unwrap();
    let payload: &[u8] = b"sealed secret: hunter2";
    let fixture = seal_fixture(dir.path(), payload);
    let out_path = dir.path().join("opened.bin");

    let output = run_lys(&[
        "open",
        "--key",
        path_str(&fixture.recipient_key),
        "--sender-public-key",
        &fixture.sender_pub,
        "--envelope",
        path_str(&fixture.envelope_path),
        "--attestation",
        path_str(&fixture.attestation_path),
        "--out",
        path_str(&out_path),
    ]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr_of(&output));

    // Exact plaintext recovered on disk; stdout carries only metadata.
    assert_eq!(std::fs::read(&out_path).unwrap(), payload);
    let stdout = stdout_of(&output);
    assert!(stdout.contains("sealed envelope opened"), "{stdout}");
    assert!(
        !stdout.contains("hunter2"),
        "plaintext leaked into stdout: {stdout}"
    );
    assert_eq!(
        field(&stdout, "sender public key (ed25519):"),
        fixture.sender_pub
    );
    assert_eq!(field(&stdout, "payload bytes:"), payload.len().to_string());
    let seed_hex = hex_lower(&std::fs::read(&fixture.recipient_key).unwrap());
    assert!(
        !stdout.contains(&seed_hex),
        "private seed leaked into stdout"
    );
}

#[test]
fn open_failures_collapse_to_one_generic_message() {
    let dir = tempfile::tempdir().unwrap();
    let fixture = seal_fixture(dir.path(), b"non-oracle payload");
    let out_path = dir.path().join("opened.bin");

    // An unrelated identity: wrong recipient key, and wrong expected sender.
    let other_key = dir.path().join("other.key");
    let generate = run_lys(&["key", "generate", "--out", path_str(&other_key)]);
    assert_eq!(generate.status.code(), Some(0), "{}", stderr_of(&generate));
    let other_pub = field(&stdout_of(&generate), "public key (ed25519):");

    // A tampered envelope: flip one ciphertext byte, keeping the JSON shape.
    let tampered_envelope = dir.path().join("tampered-envelope.json");
    let mut envelope: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&fixture.envelope_path).unwrap()).unwrap();
    let byte = envelope["ciphertext"][0].as_u64().unwrap();
    envelope["ciphertext"][0] = serde_json::Value::from(byte ^ 0x01);
    std::fs::write(
        &tampered_envelope,
        serde_json::to_string(&envelope).unwrap(),
    )
    .unwrap();

    // A tampered attestation: flip one signature byte in the raw COSE
    // artifact (the final byte is inside the 64-byte signature), keeping
    // the artifact canonical so the rejection is cryptographic.
    let tampered_attestation = dir.path().join("tampered-attestation.cose");
    let mut attestation_bytes = std::fs::read(&fixture.attestation_path).unwrap();
    let last = attestation_bytes.len() - 1;
    attestation_bytes[last] ^= 0x01;
    std::fs::write(&tampered_attestation, &attestation_bytes).unwrap();

    let cases: Vec<Vec<&str>> = vec![
        // Wrong recipient key (decryption would fail).
        vec![
            "open",
            "--key",
            path_str(&other_key),
            "--sender-public-key",
            &fixture.sender_pub,
            "--envelope",
            path_str(&fixture.envelope_path),
            "--attestation",
            path_str(&fixture.attestation_path),
            "--out",
            path_str(&out_path),
        ],
        // Wrong expected sender (attestation binding fails).
        vec![
            "open",
            "--key",
            path_str(&fixture.recipient_key),
            "--sender-public-key",
            &other_pub,
            "--envelope",
            path_str(&fixture.envelope_path),
            "--attestation",
            path_str(&fixture.attestation_path),
            "--out",
            path_str(&out_path),
        ],
        // Tampered ciphertext (signature over envelope bytes fails).
        vec![
            "open",
            "--key",
            path_str(&fixture.recipient_key),
            "--sender-public-key",
            &fixture.sender_pub,
            "--envelope",
            path_str(&tampered_envelope),
            "--attestation",
            path_str(&fixture.attestation_path),
            "--out",
            path_str(&out_path),
        ],
        // Tampered attestation signature.
        vec![
            "open",
            "--key",
            path_str(&fixture.recipient_key),
            "--sender-public-key",
            &fixture.sender_pub,
            "--envelope",
            path_str(&fixture.envelope_path),
            "--attestation",
            path_str(&tampered_attestation),
            "--out",
            path_str(&out_path),
        ],
    ];

    let mut messages = Vec::new();
    for args in &cases {
        let output = run_lys(args);
        assert_eq!(output.status.code(), Some(1), "args: {args:?}");
        let stderr = stderr_of(&output);
        assert!(
            stderr.contains("sealed envelope open failed"),
            "stderr: {stderr}"
        );
        assert!(
            !stdout_of(&output).contains("sealed envelope opened"),
            "must not claim success"
        );
        assert!(!out_path.exists(), "no plaintext may be written on failure");
        messages.push(stderr);
    }
    // Non-oracle: wrong recipient key, wrong sender, tampered envelope, and
    // tampered attestation must all be indistinguishable to the caller.
    for message in &messages[1..] {
        assert_eq!(&messages[0], message);
    }
}

#[test]
fn open_with_missing_key_fails_and_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let fixture = seal_fixture(dir.path(), b"payload");
    let out_path = dir.path().join("opened.bin");
    let absent_key = dir.path().join("absent.key");

    let output = run_lys(&[
        "open",
        "--key",
        path_str(&absent_key),
        "--sender-public-key",
        &fixture.sender_pub,
        "--envelope",
        path_str(&fixture.envelope_path),
        "--attestation",
        path_str(&fixture.attestation_path),
        "--out",
        path_str(&out_path),
    ]);
    assert_eq!(output.status.code(), Some(1));
    let stderr = stderr_of(&output);
    assert!(
        stderr.contains("identity key file not found"),
        "stderr: {stderr}"
    );
    assert!(!out_path.exists(), "no plaintext may be written on failure");
    assert!(
        !absent_key.exists(),
        "open must never create a key file as a side effect"
    );
}

#[test]
fn open_rejects_malformed_envelope_json() {
    let dir = tempfile::tempdir().unwrap();
    let fixture = seal_fixture(dir.path(), b"payload");
    let out_path = dir.path().join("opened.bin");
    let bogus_envelope = dir.path().join("bogus.json");
    std::fs::write(&bogus_envelope, b"{ not json ]").unwrap();

    let output = run_lys(&[
        "open",
        "--key",
        path_str(&fixture.recipient_key),
        "--sender-public-key",
        &fixture.sender_pub,
        "--envelope",
        path_str(&bogus_envelope),
        "--attestation",
        path_str(&fixture.attestation_path),
        "--out",
        path_str(&out_path),
    ]);
    assert_eq!(output.status.code(), Some(1));
    let stderr = stderr_of(&output);
    assert!(
        stderr.contains("failed to parse sealed envelope JSON"),
        "stderr: {stderr}"
    );
    assert!(!out_path.exists(), "no plaintext may be written on failure");
}

#[test]
fn verify_collapses_malformed_attestation_into_the_generic_failure() {
    // Non-oracle widening (design V7): a file that is not a COSE artifact
    // at all produces the SAME generic message as a cryptographic
    // rejection — no distinct parse error exists for attestations.
    let dir = tempfile::tempdir().unwrap();
    let attestation_path = dir.path().join("attestation.cose");
    let payload_path = dir.path().join("payload.bin");
    std::fs::write(&attestation_path, b"not a cose artifact").unwrap();
    std::fs::write(&payload_path, b"payload").unwrap();

    let output = run_lys(&[
        "verify",
        "--attestation",
        path_str(&attestation_path),
        "--payload",
        path_str(&payload_path),
    ]);
    assert_eq!(output.status.code(), Some(1));
    let malformed_stderr = stderr_of(&output);
    assert!(
        malformed_stderr.contains("attestation verification failed"),
        "stderr: {malformed_stderr}"
    );
    assert!(
        !malformed_stderr.contains("parse"),
        "malformed artifacts must not get a distinct parse error: {malformed_stderr}"
    );

    // Same message as a signature-level rejection: attest a payload, then
    // verify against a different payload and compare stderr byte-for-byte.
    let out_path = attest_fixture(dir.path(), b"real payload");
    std::fs::write(dir.path().join("payload.bin"), b"different payload").unwrap();
    let output = run_lys(&[
        "verify",
        "--attestation",
        path_str(&out_path),
        "--payload",
        path_str(&dir.path().join("payload.bin")),
    ]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr_of(&output),
        malformed_stderr,
        "malformed-artifact and wrong-payload failures must be indistinguishable"
    );
}

#[test]
fn open_collapses_malformed_attestation_into_the_generic_failure() {
    // The seal-open path widens identically: a malformed attestation file
    // collapses into the existing generic open failure, not a parse error.
    let dir = tempfile::tempdir().unwrap();
    let fixture = seal_fixture(dir.path(), b"payload");
    let out_path = dir.path().join("opened.bin");
    let bogus_attestation = dir.path().join("bogus.cose");
    std::fs::write(&bogus_attestation, b"not a cose artifact").unwrap();

    let output = run_lys(&[
        "open",
        "--key",
        path_str(&fixture.recipient_key),
        "--sender-public-key",
        &fixture.sender_pub,
        "--envelope",
        path_str(&fixture.envelope_path),
        "--attestation",
        path_str(&bogus_attestation),
        "--out",
        path_str(&out_path),
    ]);
    assert_eq!(output.status.code(), Some(1));
    let stderr = stderr_of(&output);
    assert!(
        stderr.contains("sealed envelope open failed"),
        "stderr: {stderr}"
    );
    assert!(
        !stderr.contains("parse"),
        "malformed attestations must not get a distinct parse error: {stderr}"
    );
    assert!(!out_path.exists(), "no plaintext may be written on failure");
}
