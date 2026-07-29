#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;

#[test]
fn read_file_round_trips_written_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("data.bin");
    write_file(&path, b"payload bytes", "test file").unwrap();
    assert_eq!(read_file(&path, "test file").unwrap(), b"payload bytes");
}

#[test]
fn read_file_error_names_role_and_path() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("missing.bin");
    let err = read_file(&path, "payload file").unwrap_err();
    let display = err.to_string();
    assert!(display.contains("payload file"), "got: {display}");
    assert!(display.contains("missing.bin"), "got: {display}");
}

#[test]
fn write_file_private_round_trips_and_is_owner_only_on_unix() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("plain.bin");
    write_file_private(&path, b"recovered plaintext", "opened payload file").unwrap();
    assert_eq!(
        read_file(&path, "opened payload file").unwrap(),
        b"recovered plaintext"
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600, "private file must be 0600");
    }
}

/// Recovered plaintext must not inherit a pre-existing file's permissions.
///
/// `.mode(0o600)` on `OpenOptions` applies only when the file is created.
/// An attacker — or an ordinary earlier command — who leaves a
/// world-readable file at the output path would otherwise have `lys open`
/// truncate it and fill it with decrypted plaintext at mode 0644, and the
/// command reports success either way. lys-core's key-write path already
/// force-tightens after writing; this is the plaintext analog.
#[cfg(unix)]
#[test]
fn write_file_private_tightens_a_pre_existing_world_readable_file() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("plain.bin");

    // Pre-existing, world-readable, with content that must be replaced.
    std::fs::write(&path, b"stale").unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
    assert_eq!(
        std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o644,
        "precondition: the file starts world-readable"
    );

    write_file_private(&path, b"recovered plaintext", "opened payload file").unwrap();

    assert_eq!(
        read_file(&path, "opened payload file").unwrap(),
        b"recovered plaintext"
    );
    let mode = std::fs::metadata(&path).unwrap().permissions().mode();
    assert_eq!(
        mode & 0o777,
        0o600,
        "plaintext must not keep the pre-existing file's permissions"
    );
}

#[test]
fn write_file_private_error_names_role_and_path() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("no-such-dir").join("plain.bin");
    let err = write_file_private(&path, b"x", "opened payload file").unwrap_err();
    let display = err.to_string();
    assert!(display.contains("opened payload file"), "got: {display}");
    assert!(display.contains("plain.bin"), "got: {display}");
}

#[test]
fn write_file_error_names_role_and_path() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("no-such-dir").join("out.json");
    let err = write_file(&path, b"x", "attestation file").unwrap_err();
    let display = err.to_string();
    assert!(display.contains("attestation file"), "got: {display}");
    assert!(display.contains("out.json"), "got: {display}");
}
