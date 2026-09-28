//! `lys key generate` and `lys key inspect`, with the signed-note verifier name.

use lys_core::Ed25519Identity;

use super::{field, hex_lower, path_str, run_lys, stderr_of, stdout_of};

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

// ------------------------------------------------- key inspect --note-name

/// Golden test seed from the design (32 ASCII bytes), shared with the
/// lys-core golden vectors and the Go conformance gate.
const GOLDEN_SEED: &[u8; 32] = b"lys-go-conformance-test-seed-01!";

/// Golden verifier key text form for (example.com/lys/test, golden pubkey).
const GOLDEN_VERIFIER_SPEC: &str =
    "example.com/lys/test+52580cd9+AQz9D9gbFqzLxSMM9Fy6nUuTfYJ8bI29RKFE5aulcbni";

#[test]
fn key_inspect_note_name_prints_golden_verifier_key() {
    let dir = tempfile::tempdir().unwrap();
    let key_path = dir.path().join("golden.key");
    std::fs::write(&key_path, GOLDEN_SEED).unwrap();

    let output = run_lys(&[
        "key",
        "inspect",
        "--key",
        path_str(&key_path),
        "--note-name",
        "example.com/lys/test",
    ]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr_of(&output));
    let stdout = stdout_of(&output);
    let spec = field(&stdout, "verifier key (signed-note):");
    assert_eq!(spec, GOLDEN_VERIFIER_SPEC);

    // Cross-check against the library's own derivation.
    let identity = Ed25519Identity::load_or_generate(&key_path).unwrap();
    let expected = lys_core::checkpoint::NoteVerifierKey::new(
        "example.com/lys/test",
        identity.public_key_bytes(),
    )
    .unwrap();
    assert_eq!(spec, expected.to_spec());

    // Still never prints private material.
    let seed_hex = hex_lower(GOLDEN_SEED);
    assert!(!stdout.contains(&seed_hex), "private seed leaked");
}

#[test]
fn key_inspect_without_note_name_prints_no_verifier_line() {
    let dir = tempfile::tempdir().unwrap();
    let key_path = dir.path().join("golden.key");
    std::fs::write(&key_path, GOLDEN_SEED).unwrap();

    let output = run_lys(&["key", "inspect", "--key", path_str(&key_path)]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr_of(&output));
    assert!(
        !stdout_of(&output).contains("verifier key"),
        "no verifier line without --note-name: {}",
        stdout_of(&output)
    );
}

#[test]
fn key_inspect_note_name_rejects_invalid_names() {
    let dir = tempfile::tempdir().unwrap();
    let key_path = dir.path().join("golden.key");
    std::fs::write(&key_path, GOLDEN_SEED).unwrap();

    for bad in ["has space", "has+plus", ""] {
        let output = run_lys(&[
            "key",
            "inspect",
            "--key",
            path_str(&key_path),
            "--note-name",
            bad,
        ]);
        assert_eq!(output.status.code(), Some(1), "name {bad:?} was accepted");
        let stderr = stderr_of(&output);
        assert!(stderr.contains("invalid note verifier key"), "{stderr}");
    }
}
