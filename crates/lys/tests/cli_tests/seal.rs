//! `lys seal`.

use lys_core::Ed25519Identity;
use lys_core::attestation::Attestation;
use lys_core::seal::{SealedEnvelope, open_and_verify};

use super::{field, hex_lower, path_str, run_lys, seal_fixture, stderr_of, stdout_of};

#[test]
fn seal_writes_envelope_and_attestation_that_lys_core_opens() {
    let dir = tempfile::tempdir().unwrap();
    let payload: &[u8] = b"credential bundle: api token hunter2";
    let fixture = seal_fixture(dir.path(), payload);

    // Both files are the exact lys-core wire artifacts — the envelope as
    // JSON, the attestation as raw COSE bytes — and the pair opens through
    // the library directly with the recipient's key.
    let envelope_json = std::fs::read_to_string(&fixture.envelope_path).unwrap();
    let envelope: SealedEnvelope = serde_json::from_str(&envelope_json).unwrap();
    let attestation_bytes = std::fs::read(&fixture.attestation_path).unwrap();
    let attestation = Attestation::from_cose_bytes(&attestation_bytes).unwrap();

    let sender = Ed25519Identity::load_or_generate(&fixture.sender_key).unwrap();
    let recipient = Ed25519Identity::load_or_generate(&fixture.recipient_key).unwrap();
    assert_eq!(attestation.signer_public_key, sender.public_key_bytes());
    let opened = open_and_verify(
        &envelope,
        &attestation,
        &sender.public_key_bytes(),
        &recipient.x25519_static_secret(),
    )
    .unwrap();
    assert_eq!(opened.as_slice(), payload);

    // The ciphertext is not the plaintext, and neither the plaintext nor
    // either private seed appears in any output.
    assert_ne!(envelope.ciphertext.as_slice(), payload);
    let seal_output = run_lys(&[
        "seal",
        "--key",
        path_str(&fixture.sender_key),
        "--recipient-public-key",
        &fixture.recipient_x25519_pub,
        "--payload",
        path_str(&fixture.payload_path),
        "--out",
        path_str(&fixture.envelope_path),
        "--attestation-out",
        path_str(&fixture.attestation_path),
    ]);
    assert_eq!(seal_output.status.code(), Some(0));
    let stdout = stdout_of(&seal_output);
    assert!(
        !stdout.contains("hunter2"),
        "plaintext leaked into stdout: {stdout}"
    );
    for key_path in [&fixture.sender_key, &fixture.recipient_key] {
        let seed_hex = hex_lower(&std::fs::read(key_path).unwrap());
        assert!(
            !stdout.contains(&seed_hex),
            "private seed leaked into stdout"
        );
        assert!(
            !envelope_json.contains(&seed_hex),
            "private seed leaked into envelope"
        );
    }
    assert_eq!(field(&stdout, "sender public key (ed25519):").len(), 64);
    assert_eq!(
        field(&stdout, "recipient public key (x25519):"),
        fixture.recipient_x25519_pub
    );
}

#[test]
fn seal_with_missing_key_fails_and_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let payload_path = dir.path().join("payload.bin");
    let envelope_path = dir.path().join("envelope.json");
    let attestation_path = dir.path().join("seal-attestation.cose");
    std::fs::write(&payload_path, b"payload").unwrap();

    let output = run_lys(&[
        "seal",
        "--key",
        path_str(&dir.path().join("absent.key")),
        "--recipient-public-key",
        &"ab".repeat(32),
        "--payload",
        path_str(&payload_path),
        "--out",
        path_str(&envelope_path),
        "--attestation-out",
        path_str(&attestation_path),
    ]);
    assert_eq!(output.status.code(), Some(1));
    let stderr = stderr_of(&output);
    assert!(
        stderr.contains("identity key file not found"),
        "stderr: {stderr}"
    );
    assert!(!envelope_path.exists(), "no envelope on failure");
    assert!(!attestation_path.exists(), "no attestation on failure");
    assert!(
        !dir.path().join("absent.key").exists(),
        "seal must never create a key file as a side effect"
    );
}

#[test]
fn seal_rejects_invalid_recipient_public_key_hex() {
    let dir = tempfile::tempdir().unwrap();
    let sender_key = dir.path().join("sender.key");
    let payload_path = dir.path().join("payload.bin");
    let envelope_path = dir.path().join("envelope.json");
    let attestation_path = dir.path().join("seal-attestation.cose");

    let generate = run_lys(&["key", "generate", "--out", path_str(&sender_key)]);
    assert_eq!(generate.status.code(), Some(0), "{}", stderr_of(&generate));
    std::fs::write(&payload_path, b"payload").unwrap();

    for bad in ["zz", "abc123", &"ab".repeat(33)] {
        let output = run_lys(&[
            "seal",
            "--key",
            path_str(&sender_key),
            "--recipient-public-key",
            bad,
            "--payload",
            path_str(&payload_path),
            "--out",
            path_str(&envelope_path),
            "--attestation-out",
            path_str(&attestation_path),
        ]);
        assert_eq!(output.status.code(), Some(1), "input: {bad}");
        assert!(
            stderr_of(&output).contains("invalid recipient public key"),
            "stderr: {}",
            stderr_of(&output)
        );
        assert!(!envelope_path.exists(), "no envelope on failure");
    }
}

#[test]
fn seal_attestation_write_failure_leaves_no_partial_envelope() {
    let dir = tempfile::tempdir().unwrap();
    let sender_key = dir.path().join("sender.key");
    let recipient_key = dir.path().join("recipient.key");
    let payload_path = dir.path().join("payload.bin");
    let envelope_path = dir.path().join("envelope.json");
    // Unwritable attestation destination: parent directory does not exist.
    let attestation_path = dir.path().join("no-such-dir").join("attestation.cose");

    let generate_sender = run_lys(&["key", "generate", "--out", path_str(&sender_key)]);
    assert_eq!(
        generate_sender.status.code(),
        Some(0),
        "{}",
        stderr_of(&generate_sender)
    );
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
    std::fs::write(&payload_path, b"payload").unwrap();

    let output = run_lys(&[
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
    assert_eq!(output.status.code(), Some(1));
    assert!(
        stderr_of(&output).contains("seal attestation file"),
        "stderr: {}",
        stderr_of(&output)
    );
    // The envelope written before the attestation failure must be cleaned
    // up: an envelope without its attestation is unopenable, and failed
    // commands leave no partial outputs.
    assert!(
        !envelope_path.exists(),
        "orphaned envelope left behind after attestation write failure"
    );
}
