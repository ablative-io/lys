//! `lys inspect attestation` and `lys inspect cert`: fields shown unverified.

use base64::Engine;
use lys_core::attestation::Attestation;
use lys_core::ca::{CertificateAuthority, encode_extension};

use super::{
    CLAIMS_JSON, CLAIMS_OID, attest_fixture, ca_issue_fixture, field, hex_lower, path_str, run_lys,
    stderr_of, stdout_of,
};

// -------------------------------------------------------- inspect attestation

#[test]
fn inspect_attestation_prints_unverified_fields_matching_attest() {
    let dir = tempfile::tempdir().unwrap();
    let key_path = dir.path().join("agent.key");
    let payload_path = dir.path().join("payload.bin");
    let cose_path = dir.path().join("attestation.cose");

    let generate = run_lys(&["key", "generate", "--out", path_str(&key_path)]);
    assert_eq!(generate.status.code(), Some(0), "{}", stderr_of(&generate));
    std::fs::write(&payload_path, b"inspect me: task 7 completed").unwrap();
    let attest = run_lys(&[
        "attest",
        "--key",
        path_str(&key_path),
        "--payload",
        path_str(&payload_path),
        "--out",
        path_str(&cose_path),
    ]);
    assert_eq!(attest.status.code(), Some(0), "{}", stderr_of(&attest));
    let attest_stdout = stdout_of(&attest);

    let output = run_lys(&[
        "inspect",
        "attestation",
        "--attestation",
        path_str(&cose_path),
    ]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr_of(&output));
    let stdout = stdout_of(&output);

    // The first line is the unverified banner, and it names the command that
    // does verify.
    let first_line = stdout.lines().next().expect("inspect printed nothing");
    assert!(first_line.contains("UNVERIFIED"), "{first_line}");
    assert!(
        first_line.contains("signature was NOT checked"),
        "{first_line}"
    );
    assert!(first_line.contains("lys verify"), "{first_line}");
    assert!(
        !stdout.contains("attestation verified"),
        "inspect must never claim verification: {stdout}"
    );

    // Every field matches what `lys attest` printed for the same artifact.
    for label in [
        "signer public key (ed25519):",
        "payload hash (sha256):",
        "signed at (unix ms):",
    ] {
        assert_eq!(
            field(&stdout, label),
            field(&attest_stdout, label),
            "{label}"
        );
    }

    // ...and what the library reads back out of the artifact on disk.
    let artifact = std::fs::read(&cose_path).unwrap();
    let attestation = Attestation::from_cose_bytes(&artifact).unwrap();
    assert_eq!(
        field(&stdout, "signer public key (ed25519):"),
        hex_lower(&attestation.signer_public_key)
    );
    assert_eq!(
        field(&stdout, "payload hash (sha256):"),
        hex_lower(&attestation.payload_hash)
    );
    assert_eq!(
        field(&stdout, "signed at (unix ms):"),
        attestation.timestamp.to_string()
    );
    let rendered = chrono::DateTime::<chrono::Utc>::from_timestamp_millis(attestation.timestamp)
        .expect("attestation timestamp outside the representable range")
        .to_rfc3339();
    assert_eq!(field(&stdout, "signed at (rfc3339):"), rendered);
}

#[test]
fn inspect_attestation_reads_an_artifact_that_verify_rejects() {
    let dir = tempfile::tempdir().unwrap();
    let cose_path = attest_fixture(dir.path(), b"corruptible payload");

    // Overwrite the 64 detached signature bytes with different ones. The
    // signature bstr head (`58 40`) and every other byte of the encoding are
    // untouched, so the artifact stays canonical — only the signature is now
    // wrong. `inspect` reads it; `verify` must not.
    let mut artifact = std::fs::read(&cose_path).unwrap();
    let len = artifact.len();
    assert_eq!(
        &artifact[len - 66..len - 64],
        &[0x58, 0x40],
        "signature bstr head not at its fixed wire position"
    );
    for byte in &mut artifact[len - 64..] {
        *byte ^= 0x01;
    }
    std::fs::write(&cose_path, &artifact).unwrap();

    let inspect = run_lys(&[
        "inspect",
        "attestation",
        "--attestation",
        path_str(&cose_path),
    ]);
    assert_eq!(inspect.status.code(), Some(0), "{}", stderr_of(&inspect));
    let stdout = stdout_of(&inspect);
    assert!(stdout.starts_with("UNVERIFIED"), "{stdout}");
    assert_eq!(field(&stdout, "signer public key (ed25519):").len(), 64);

    let verify = run_lys(&[
        "verify",
        "--attestation",
        path_str(&cose_path),
        "--payload",
        path_str(&dir.path().join("payload.bin")),
    ]);
    assert_eq!(
        verify.status.code(),
        Some(1),
        "a corrupted signature must not verify"
    );
    assert!(
        stderr_of(&verify).contains("attestation verification failed"),
        "stderr: {}",
        stderr_of(&verify)
    );
}

#[test]
fn inspect_attestation_collapses_garbage_into_the_generic_failure() {
    let dir = tempfile::tempdir().unwrap();
    let garbage_path = dir.path().join("garbage.cose");
    std::fs::write(&garbage_path, b"not a cose artifact").unwrap();

    let output = run_lys(&[
        "inspect",
        "attestation",
        "--attestation",
        path_str(&garbage_path),
    ]);
    assert_eq!(output.status.code(), Some(1));
    let stderr = stderr_of(&output);
    assert!(
        stderr.contains("attestation verification failed"),
        "stderr: {stderr}"
    );
    assert!(
        !stderr.contains("parse"),
        "undecodable artifacts must not get a distinct parse error: {stderr}"
    );
    assert!(
        stdout_of(&output).is_empty(),
        "no field may be printed for an undecodable artifact: {}",
        stdout_of(&output)
    );
}

// --------------------------------------------------------------- inspect cert

#[test]
fn inspect_cert_prints_unverified_subject_and_claims_without_an_issuer_key() {
    let dir = tempfile::tempdir().unwrap();
    let (cert_path, _issuer_pub) = ca_issue_fixture(dir.path(), "1");

    // No issuer key is supplied anywhere in this invocation.
    let output = run_lys(&["inspect", "cert", "--cert", path_str(&cert_path)]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr_of(&output));
    let stdout = stdout_of(&output);

    let first_line = stdout.lines().next().expect("inspect printed nothing");
    assert!(
        first_line.contains("NOT been checked against any issuer"),
        "{first_line}"
    );
    assert!(first_line.contains("lys ca verify"), "{first_line}");
    assert!(
        !stdout.contains("certificate verified"),
        "inspect must never claim verification: {stdout}"
    );

    // Ordering by byte offset: banner first, then the claims marker, then the
    // claims themselves — a reader can never meet unverified claims before
    // being told they are unverified.
    let banner_offset = stdout.find("UNVERIFIED").expect("no unverified banner");
    assert_eq!(banner_offset, 0, "banner must open the output: {stdout}");
    let marker_offset = stdout
        .find("UNVERIFIED CLAIMS")
        .expect("no unverified-claims marker");
    let claims_offset = stdout.find(CLAIMS_JSON).expect("claims were not echoed");
    assert!(
        banner_offset < marker_offset,
        "banner must precede the claims marker: {stdout}"
    );
    assert!(
        marker_offset < claims_offset,
        "claims marker must precede the claims JSON: {stdout}"
    );

    assert_eq!(field(&stdout, "subject:"), "agent-under-test");
    assert_eq!(field(&stdout, "subject public key (ed25519):").len(), 64);
    assert!(
        !field(&stdout, "not before (rfc3339):").is_empty(),
        "{stdout}"
    );
    assert!(
        !field(&stdout, "not after (rfc3339):").is_empty(),
        "{stdout}"
    );
}

#[test]
fn inspect_cert_echoes_control_character_claims_as_hex_never_raw() {
    // The viewer reads certificates from anywhere — nothing has vouched for
    // them — so claims carrying raw terminal escapes must be hex-encoded
    // exactly as `ca verify` hex-encodes them, never replayed.
    let dir = tempfile::tempdir().unwrap();
    let key_path = dir.path().join("issuer.key");
    let cert_path = dir.path().join("hostile-claims.pem");

    let generate = run_lys(&["key", "generate", "--out", path_str(&key_path)]);
    assert_eq!(generate.status.code(), Some(0), "{}", stderr_of(&generate));

    let identity = lys_core::Ed25519Identity::load(&key_path).unwrap();
    let authority = CertificateAuthority::new(identity);
    let hostile_claims = b"{\"role\":\"\x1b[2K spoofed\"}".to_vec();
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

    let output = run_lys(&["inspect", "cert", "--cert", path_str(&cert_path)]);
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
    assert!(
        stdout.contains("UNVERIFIED CLAIMS"),
        "claims must still be marked unverified: {stdout}"
    );
}
