#![cfg(test)]

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

/// The names in `dir`, sorted, so a leftover temporary file shows up.
fn names_in(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

#[test]
fn a_staged_file_reaches_its_target_only_when_placed() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("agent.pem");
    let staged = StagedFile::stage(&target, b"certificate bytes", "certificate file").unwrap();
    assert!(!target.exists(), "staging must not touch the target");
    staged.place().unwrap();
    assert_eq!(std::fs::read(&target).unwrap(), b"certificate bytes");
    assert_eq!(names_in(dir.path()), ["agent.pem"]);
}

#[test]
fn a_staged_file_dropped_unplaced_leaves_nothing_behind() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("agent.pem");
    drop(StagedFile::stage(&target, b"certificate bytes", "certificate file").unwrap());
    assert!(names_in(dir.path()).is_empty());
}

#[test]
fn staging_into_a_missing_directory_names_the_output_and_its_path() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("absent").join("proof.json");
    let err = StagedFile::stage(&target, b"{}", "inclusion proof artifact").unwrap_err();
    let display = err.to_string();
    assert!(
        display.contains("inclusion proof artifact"),
        "got: {display}"
    );
    assert!(display.contains("proof.json"), "got: {display}");
}

#[test]
fn an_existing_output_is_refused_by_name() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("agent.pem");
    std::fs::write(&path, b"already here").unwrap();
    let err = refuse_existing(&path, "certificate file").unwrap_err();
    assert!(matches!(
        err,
        CliError::OutputExists {
            what: "certificate file",
            ..
        }
    ));
    assert!(err.to_string().contains("agent.pem"), "got: {err}");
    refuse_existing(&dir.path().join("absent.pem"), "certificate file").unwrap();
}

#[test]
fn two_outputs_naming_one_path_are_refused_by_name() {
    let err = refuse_shared_paths(&[
        ("certificate file", Path::new("out/agent.pem")),
        ("issuer certificate file", Path::new("out/issuer.pem")),
        ("leaf file", Path::new("out/./agent.pem")),
    ])
    .unwrap_err();
    assert!(matches!(
        err,
        CliError::OutputPathShared {
            first: "certificate file",
            second: "leaf file",
            ..
        }
    ));
    refuse_shared_paths(&[
        ("certificate file", Path::new("agent.pem")),
        ("leaf file", Path::new("agent.leaf")),
    ])
    .unwrap();
}
