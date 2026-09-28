//! `lys attest` and `lys verify`.

use lys_core::Ed25519Identity;
use lys_core::attestation::verify_attestation_bytes;

use super::{attest_fixture, field, hex_lower, path_str, run_lys, stderr_of, stdout_of};

// --------------------------------------------------------------------- attest

#[test]
fn attest_writes_cose_artifact_that_lys_core_verifies() {
    let dir = tempfile::tempdir().unwrap();
    let key_path = dir.path().join("agent.key");
    let payload_path = dir.path().join("payload.bin");
    let out_path = dir.path().join("attestation.cose");
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

    // The written file is the raw tagged COSE_Sign1 artifact — canonical
    // size window, required tag byte — and verifies against the payload
    // through the library directly.
    let artifact = std::fs::read(&out_path).unwrap();
    assert!(
        (191..=199).contains(&artifact.len()),
        "artifact length {} outside the canonical window",
        artifact.len()
    );
    assert_eq!(artifact[0], 0xd2, "artifact must carry CBOR tag 18");
    let attestation = verify_attestation_bytes(&artifact, payload).unwrap();

    // Printed metadata matches the artifact on disk, and the output notes
    // the standard format.
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
    assert!(
        field(&stdout, "attestation written:").contains("COSE_Sign1, application/cose"),
        "{stdout}"
    );

    // Neither the raw seed bytes nor their hex encoding appear in the
    // artifact, and the hex never appears in stdout.
    let seed = std::fs::read(&key_path).unwrap();
    let seed_hex = hex_lower(&seed);
    assert!(
        !artifact
            .windows(seed.len())
            .any(|window| window == seed.as_slice()),
        "private seed bytes leaked into the artifact"
    );
    assert!(
        !hex_lower(&artifact).contains(&seed_hex),
        "private seed hex leaked into the artifact"
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
    let out_path = dir.path().join("attestation.cose");
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

    // Flip the low byte of the (authenticated) timestamp inside the CBOR
    // claims. A present-day unix-ms timestamp exceeds u32::MAX, so the
    // claim is encoded `02 1b <8 bytes>`; it closes the claims map and the
    // signature bstr (`58 40` + 64 bytes) closes the artifact, so the claim
    // head sits at a fixed offset from the end. Assert the head bytes, then
    // flip the final value byte — the encoding stays canonical while the
    // signed claim changes.
    let mut artifact = std::fs::read(&out_path).unwrap();
    let ts_head = artifact.len() - 76;
    assert_eq!(
        &artifact[ts_head..ts_head + 2],
        &[0x02, 0x1b],
        "timestamp claim head not at its fixed wire position"
    );
    artifact[ts_head + 9] ^= 0x01;
    std::fs::write(&out_path, &artifact).unwrap();

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
