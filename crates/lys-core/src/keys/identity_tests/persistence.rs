#![cfg(test)]
//! Loading and generating the identity file: missing, valid, wrong-length
//! and oversized files, permissions, parent directories, and the race
//! between concurrent generators.

use super::*;

// ─── load_or_generate ─────────────────────────────────────────────

#[test]
fn load_or_generate_creates_file_when_missing() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("identity.key");
    assert!(!path.exists());

    let id = Ed25519Identity::load_or_generate(&path).unwrap();
    assert!(path.exists());

    let contents = std::fs::read(&path).unwrap();
    assert_eq!(contents.len(), 32, "file must be exactly 32 bytes");

    let reloaded = Ed25519Identity::load(&path).unwrap();
    assert_eq!(
        reloaded.public_key_bytes(),
        id.public_key_bytes(),
        "the generated file must load back as the same identity"
    );

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600, "generated key file must be 0600");
    }
}

#[test]
fn load_or_generate_loads_existing_valid_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("identity.key");
    let seed = [5u8; 32];
    std::fs::write(&path, seed).unwrap();
    #[cfg(unix)]
    chmod(&path, 0o600);

    let id = Ed25519Identity::load_or_generate(&path).unwrap();
    let expected_pk = ed25519_dalek::SigningKey::from_bytes(&seed)
        .verifying_key()
        .to_bytes();
    assert_eq!(id.public_key_bytes(), expected_pk);
}

// ─── load (load-only) ─────────────────────────────────────────────

#[test]
fn load_reads_existing_valid_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("identity.key");
    let seed = [5u8; 32];
    std::fs::write(&path, seed).unwrap();
    #[cfg(unix)]
    chmod(&path, 0o600);

    let id = Ed25519Identity::load(&path).unwrap();
    let expected_pk = ed25519_dalek::SigningKey::from_bytes(&seed)
        .verifying_key()
        .to_bytes();
    assert_eq!(id.public_key_bytes(), expected_pk);
}

#[test]
fn load_refuses_missing_file_and_creates_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("identity.key");
    assert!(!path.exists());

    let err = Ed25519Identity::load(&path).unwrap_err();
    assert!(
        matches!(err, TrustError::KeyManagement { .. }),
        "got: {err:?}"
    );
    assert!(
        !path.exists(),
        "load-only constructor must never create a key file"
    );
}

#[test]
fn load_rejects_wrong_length() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("identity.key");
    std::fs::write(&path, [1u8; 16]).unwrap();

    let err = Ed25519Identity::load(&path).unwrap_err();
    assert!(
        matches!(err, TrustError::KeyManagement { .. }),
        "got: {err:?}"
    );
}

#[test]
fn load_or_generate_idempotent() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("identity.key");

    let id1 = Ed25519Identity::load_or_generate(&path).unwrap();
    let id2 = Ed25519Identity::load_or_generate(&path).unwrap();
    assert_eq!(id1.public_key_bytes(), id2.public_key_bytes());
}

#[test]
fn load_or_generate_rejects_wrong_length() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("identity.key");
    std::fs::write(&path, [0u8; 16]).unwrap();
    #[cfg(unix)]
    chmod(&path, 0o600);

    let err = Ed25519Identity::load_or_generate(&path).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("invalid length"), "got: {msg}");
    assert!(msg.contains("expected 32 bytes, got 16"), "got: {msg}");
}

#[test]
fn load_or_generate_rejects_oversized() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("identity.key");
    std::fs::write(&path, [0u8; 64]).unwrap();
    #[cfg(unix)]
    chmod(&path, 0o600);

    let err = Ed25519Identity::load_or_generate(&path).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("invalid length"), "got: {msg}");
    assert!(msg.contains("got 64"), "got: {msg}");
}

#[cfg(unix)]
#[test]
fn load_or_generate_warns_but_loads_on_loose_permissions() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("identity.key");
    let seed = [9u8; 32];
    std::fs::write(&path, seed).unwrap();
    chmod(&path, 0o644);

    let id = Ed25519Identity::load_or_generate(&path).unwrap();
    let expected_pk = ed25519_dalek::SigningKey::from_bytes(&seed)
        .verifying_key()
        .to_bytes();
    assert_eq!(id.public_key_bytes(), expected_pk);
}

#[test]
fn load_or_generate_creates_parent_dir() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nested").join("deep").join("identity.key");
    assert!(!path.parent().unwrap().exists());

    let _id = Ed25519Identity::load_or_generate(&path).unwrap();
    assert!(path.exists());
    assert!(path.parent().unwrap().exists());
}

#[test]
fn load_or_generate_rejects_no_filename_path() {
    let err = Ed25519Identity::load_or_generate(Path::new("")).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("no filename component"), "got: {msg}");
}

#[test]
fn load_or_generate_concurrent_threads_agree_on_persisted_seed() {
    // The concurrency invariant: racing generators in the same process must
    // all return the identity of the single seed that ends up on disk. No
    // caller may hold an identity that diverges from the persisted file, and
    // no tmp files may be left behind.
    const THREADS: usize = 8;

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("identity.key");

    let barrier = std::sync::Barrier::new(THREADS);
    let public_keys: Vec<[u8; 32]> = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..THREADS)
            .map(|_| {
                scope.spawn(|| {
                    barrier.wait();
                    Ed25519Identity::load_or_generate(&path)
                        .expect("concurrent load_or_generate must succeed")
                        .public_key_bytes()
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().expect("thread panicked"))
            .collect()
    });

    // Every thread must agree with the seed that was actually persisted.
    let persisted = std::fs::read(&path).unwrap();
    assert_eq!(persisted.len(), 32);
    let mut persisted_seed = [0u8; 32];
    persisted_seed.copy_from_slice(&persisted);
    let expected_pk = ed25519_dalek::SigningKey::from_bytes(&persisted_seed)
        .verifying_key()
        .to_bytes();
    for (i, pk) in public_keys.iter().enumerate() {
        assert_eq!(
            *pk, expected_pk,
            "thread {i} returned an identity that diverges from the persisted seed"
        );
    }

    // No temp files may survive the race.
    let leftovers: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .filter(|name| *name != *"identity.key")
        .collect();
    assert!(
        leftovers.is_empty(),
        "tmp files left behind after concurrent generation: {leftovers:?}"
    );
}

#[test]
fn generate_and_persist_lost_race_loads_winner_seed() {
    // Deterministic replay of the publish race: the key file appears after
    // the `path.exists()` check in load_or_generate but before the
    // no-clobber publish. Calling the private generate_and_persist with the
    // file already present exercises exactly that window — the caller must
    // get the WINNER's identity back, and the file must not be clobbered.
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("identity.key");
    let winner_seed = [42u8; 32];
    std::fs::write(&path, winner_seed).unwrap();
    #[cfg(unix)]
    chmod(&path, 0o600);

    let id = generate_and_persist(&path).unwrap();
    let expected_pk = ed25519_dalek::SigningKey::from_bytes(&winner_seed)
        .verifying_key()
        .to_bytes();
    assert_eq!(
        id.public_key_bytes(),
        expected_pk,
        "loser of the publish race must return the persisted (winner) identity"
    );

    // The winner's file must be untouched and no tmp files left behind.
    assert_eq!(std::fs::read(&path).unwrap(), winner_seed);
    let leftovers: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .filter(|name| *name != *"identity.key")
        .collect();
    assert!(
        leftovers.is_empty(),
        "tmp files left behind after lost publish race: {leftovers:?}"
    );
}

#[test]
fn generate_and_persist_lost_race_surfaces_invalid_winner_file() {
    // If the concurrently persisted file is corrupt (wrong length), the
    // losing generator must surface that loudly instead of silently
    // returning its own unpersisted identity.
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("identity.key");
    std::fs::write(&path, [7u8; 16]).unwrap();
    #[cfg(unix)]
    chmod(&path, 0o600);

    let err = generate_and_persist(&path).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("invalid length"), "got: {msg}");
    assert!(msg.contains("expected 32 bytes, got 16"), "got: {msg}");
}

// ─── IO failure paths ─────────────────────────────────────────────

#[cfg(unix)]
#[test]
fn load_or_generate_read_permission_denied() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("identity.key");
    std::fs::write(&path, [1u8; 32]).unwrap();
    chmod(&path, 0o000);

    let err = Ed25519Identity::load_or_generate(&path).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("failed to read"), "got: {msg}");

    chmod(&path, 0o600);
}

#[cfg(unix)]
#[test]
fn load_or_generate_write_permission_denied() {
    let dir = tempfile::tempdir().unwrap();
    let restricted = dir.path().join("noaccess");
    std::fs::create_dir(&restricted).unwrap();
    chmod(&restricted, 0o000);

    let path = restricted.join("identity.key");
    let err = Ed25519Identity::load_or_generate(&path).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("failed to write"), "got: {msg}");

    chmod(&restricted, 0o755);

    // The failed generation must not leave an orphaned tmp key file behind.
    let leftovers: Vec<_> = std::fs::read_dir(&restricted)
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    assert!(
        leftovers.is_empty(),
        "orphaned files left after failed key generation: {leftovers:?}"
    );
}

// ─── test helpers ─────────────────────────────────────────────────

#[cfg(unix)]
fn chmod(path: &Path, mode: u32) {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = std::fs::metadata(path).unwrap().permissions();
    perms.set_mode(mode);
    std::fs::set_permissions(path, perms).unwrap();
}
