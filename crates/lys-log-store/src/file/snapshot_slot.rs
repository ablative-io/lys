//! `snapshot.bin` — the store's one snapshot slot, replaced whole.
//!
//! Written the way `state.json` is: to a sibling temporary file, flushed,
//! renamed over the slot, and the directory flushed so the rename survives a
//! crash. A reader therefore sees the previous snapshot or the new one, never
//! a torn one, and a leftover temporary file is never read as a snapshot.

use std::path::Path;

use crate::error::{StoreError, StoreResult};

use super::leaves::fsync_dir;
use super::write_durably;

/// The slot's file name.
const SNAPSHOT_FILE: &str = "snapshot.bin";

/// The temporary name a snapshot is written under before the rename.
const SNAPSHOT_TEMP: &str = "snapshot.bin.tmp";

/// The snapshot in `dir`, or `None` if none was ever written.
pub(super) fn read(dir: &Path) -> StoreResult<Option<Vec<u8>>> {
    let path = dir.join(SNAPSHOT_FILE);
    match std::fs::read(&path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(source) => Err(StoreError::Io {
            context: format!("failed to read snapshot {}", path.display()),
            source,
        }),
    }
}

/// Durably replaces the snapshot in `dir` with `bytes`.
pub(super) fn write(dir: &Path, bytes: &[u8]) -> StoreResult<()> {
    let tmp_path = dir.join(SNAPSHOT_TEMP);
    write_durably(&tmp_path, bytes)?;
    let path = dir.join(SNAPSHOT_FILE);
    std::fs::rename(&tmp_path, &path).map_err(|source| StoreError::Io {
        context: format!("failed to atomically replace snapshot {}", path.display()),
        source,
    })?;
    fsync_dir(dir)
}
