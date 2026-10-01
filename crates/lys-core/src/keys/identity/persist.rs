//! The identity key file on disk: loading an existing seed, and generating
//! and publishing a new one so that concurrent generators agree on a single
//! persisted seed.

use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use rand::RngCore;
use zeroize::Zeroizing;

use super::Ed25519Identity;
use crate::error::{TrustError, TrustResult};

#[cfg(unix)]
fn warn_if_loose_permissions(path: &Path) {
    use std::os::unix::fs::PermissionsExt;

    let mode = match std::fs::metadata(path) {
        Ok(m) => m.permissions().mode(),
        Err(e) => {
            tracing::warn!(
                path = %path.display(),
                error = %e,
                "could not stat identity key file to check permissions"
            );
            return;
        }
    };
    if mode & 0o077 != 0 {
        tracing::warn!(
            path = %path.display(),
            mode = format!("{:o}", mode & 0o777),
            "identity key file has loose permissions; expected 0600"
        );
    }
}

#[cfg(not(unix))]
fn warn_if_loose_permissions(_path: &Path) {}

/// Loads an identity from an existing key file (raw 32-byte seed).
///
/// Warns (but still loads) on loose Unix permissions. Shared by
/// [`Ed25519Identity::load_or_generate`] and the lost-publish-race path of
/// [`generate_and_persist`].
pub(super) fn load_existing(path: &Path) -> TrustResult<Ed25519Identity> {
    warn_if_loose_permissions(path);
    let bytes = Zeroizing::new(std::fs::read(path).map_err(|e| TrustError::KeyManagement {
        reason: format!("failed to read identity key: {e}"),
    })?);
    if bytes.len() != 32 {
        return Err(TrustError::KeyManagement {
            reason: format!(
                "identity key file has invalid length: expected 32 bytes, got {}",
                bytes.len()
            ),
        });
    }
    let mut seed = Zeroizing::new([0u8; 32]);
    seed.copy_from_slice(&bytes);
    Ok(Ed25519Identity::from_seed(&seed))
}

/// Monotonic per-process counter mixed into temp key-file names so
/// concurrent generators within the same process never share a temp path.
/// The process id in the name covers cross-process uniqueness.
static TMP_NAME_COUNTER: AtomicU64 = AtomicU64::new(0);

pub(super) fn generate_and_persist(path: &Path) -> TrustResult<Ed25519Identity> {
    let file_name = path.file_name().ok_or_else(|| TrustError::KeyManagement {
        reason: format!(
            "identity key path has no filename component: {}",
            path.display()
        ),
    })?;
    let parent = path.parent().unwrap_or_else(|| Path::new(""));

    let mut seed = Zeroizing::new([0u8; 32]);
    rand::rng().fill_bytes(&mut *seed);

    if !parent.as_os_str().is_empty() {
        std::fs::create_dir_all(parent).map_err(|e| TrustError::KeyManagement {
            reason: format!("failed to write identity key: {e}"),
        })?;
    }

    // The tmp suffix embeds the process id (cross-process uniqueness) and a
    // per-process counter (same-process uniqueness), so concurrent
    // generators never clobber each other's in-flight tmp file.
    let unique = TMP_NAME_COUNTER.fetch_add(1, Ordering::Relaxed);
    let mut tmp_name = file_name.to_os_string();
    tmp_name.push(format!(".{}.{unique}.tmp", std::process::id()));
    let tmp_path = if parent.as_os_str().is_empty() {
        std::path::PathBuf::from(tmp_name)
    } else {
        parent.join(tmp_name)
    };

    if let Err(e) = write_identity_file(&tmp_path, &seed) {
        remove_tmp_file(&tmp_path);
        return Err(e);
    }

    // Publish with a no-clobber `hard_link` rather than `rename`: linking
    // fails with `AlreadyExists` if the destination exists, so the first
    // generator to publish wins permanently and the key file never changes
    // once created. A `rename` here would let a later generator overwrite
    // the winner, leaving the earlier caller holding an identity whose seed
    // is no longer the one on disk.
    match std::fs::hard_link(&tmp_path, path) {
        Ok(()) => {
            remove_tmp_file(&tmp_path);
            #[cfg(unix)]
            sync_parent(parent)?;
            tracing::info!(
                path = %path.display(),
                "generated and persisted identity key"
            );
            Ok(Ed25519Identity::from_seed(&seed))
        }
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            // Lost the publish race: a concurrent generator persisted its
            // key first. First writer wins — discard our candidate seed and
            // load the persisted one so we return the identity that is
            // actually on disk.
            remove_tmp_file(&tmp_path);
            tracing::info!(
                path = %path.display(),
                "identity key was persisted concurrently; loading the persisted key"
            );
            load_existing(path)
        }
        Err(e) => {
            remove_tmp_file(&tmp_path);
            Err(TrustError::KeyManagement {
                reason: format!("failed to write identity key: {e}"),
            })
        }
    }
}

/// Flushes the key's folder so the published name survives a crash.
#[cfg(unix)]
fn sync_parent(parent: &Path) -> TrustResult<()> {
    let folder = if parent.as_os_str().is_empty() {
        Path::new(".")
    } else {
        parent
    };
    std::fs::File::open(folder)
        .and_then(|handle| handle.sync_all())
        .map_err(|e| TrustError::KeyManagement {
            reason: format!("failed to flush identity key folder to disk: {e}"),
        })
}

/// Best-effort removal of an in-flight tmp key file after a failed write or
/// rename. A missing file is fine (the failure may have preceded creation);
/// any other removal error is logged so the orphaned file — which may contain
/// key material — is never silently left behind.
fn remove_tmp_file(tmp_path: &Path) {
    if let Err(e) = std::fs::remove_file(tmp_path)
        && e.kind() != std::io::ErrorKind::NotFound
    {
        tracing::warn!(
            path = %tmp_path.display(),
            error = %e,
            "failed to remove temporary identity key file after write failure"
        );
    }
}

#[cfg(unix)]
fn write_identity_file(path: &Path, seed: &[u8; 32]) -> TrustResult<()> {
    use std::io::Write;
    use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)
        .map_err(|e| TrustError::KeyManagement {
            reason: format!("failed to write identity key: {e}"),
        })?;
    file.write_all(seed)
        .map_err(|e| TrustError::KeyManagement {
            reason: format!("failed to write identity key: {e}"),
        })?;
    file.sync_all().map_err(|e| TrustError::KeyManagement {
        reason: format!("failed to flush identity key to disk: {e}"),
    })?;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)).map_err(|e| {
        TrustError::KeyManagement {
            reason: format!("failed to write identity key: {e}"),
        }
    })?;
    Ok(())
}

#[cfg(not(unix))]
fn write_identity_file(path: &Path, seed: &[u8; 32]) -> TrustResult<()> {
    use std::io::Write;

    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)
        .map_err(|e| TrustError::KeyManagement {
            reason: format!("failed to write identity key: {e}"),
        })?;
    file.write_all(seed)
        .map_err(|e| TrustError::KeyManagement {
            reason: format!("failed to write identity key: {e}"),
        })?;
    file.sync_all().map_err(|e| TrustError::KeyManagement {
        reason: format!("failed to flush identity key to disk: {e}"),
    })?;
    Ok(())
}
