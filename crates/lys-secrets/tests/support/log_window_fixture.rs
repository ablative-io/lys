#![cfg(test)]

use std::error::Error;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;

use lys_log_store::{FileLeafStore, FrontierLog, LeafStore};
use lys_secrets::{Broker, LocalGrants, StoreKey, to_hex};

use super::{AT_MS, Folders, LINES};

#[path = "log_window_pack.rs"]
mod pack;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

fn write_new(path: &Path, bytes: &[u8], mode: u32) -> Result<()> {
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(mode)
        .open(path)
        .map_err(|error| format!("FixtureCreateFailed: {}: {error}", path.display()))?
        .write_all(bytes)
        .map_err(|error| format!("FixtureWriteFailed: {}: {error}", path.display()))?;
    Ok(())
}

pub(super) fn copy() -> Result<Folders> {
    let (archive, manifest) = pack::read()?;
    let (manifest, frames) = pack::validate(&archive, &manifest)?;
    if u64::try_from(pack::RECORDS)? != LINES {
        return Err("FixtureCountMismatch: test and recorded history differ".into());
    }
    let folders = Folders {
        dir: tempfile::tempdir()?,
    };
    fs::create_dir(folders.root())
        .map_err(|error| format!("FixtureCreateFailed: broker directory: {error}"))?;
    fs::create_dir(folders.keys())
        .map_err(|error| format!("FixtureCreateFailed: keys directory: {error}"))?;
    let paths = folders.paths();
    drop(Broker::create(
        &paths,
        LocalGrants::new(),
        Box::new(|| AT_MS),
    )?);
    // Only the fresh empty audit files are replaced; each test keeps its own store.
    fs::remove_dir_all(&paths.log_dir)
        .map_err(|error| format!("FixtureReplaceFailed: empty audit log: {error}"))?;
    fs::remove_file(&paths.audit_key)
        .map_err(|error| format!("FixtureReplaceFailed: fresh audit key: {error}"))?;
    fs::remove_file(&paths.anchor)
        .map_err(|error| format!("FixtureReplaceFailed: empty anchor: {error}"))?;
    // The log is rebuilt through the store itself (LYSLOGSTORE-008 R1: the
    // layout under the directory is the store's own): created at the audit
    // origin, then every recorded line appended in one act (R3), which gives
    // byte-for-byte the store that one append at a time gives. The pack's
    // log.json and state.json frames belong to the per-leaf layout and are
    // not written; its snapshot and anchor frames are.
    drop(FileLeafStore::create(
        &paths.log_dir,
        "lys.local/secrets-audit",
    )?);
    write_new(
        &paths.audit_key,
        include_bytes!("../fixtures/log-window.key"),
        0o600,
    )?;
    let guarded = [paths.store_dir.as_path(), paths.log_dir.as_path()];
    let key = StoreKey::load(&paths.audit_key, &guarded)?;
    if key.id().as_str() != manifest.audit_key_fingerprint {
        return Err("FixtureKeyMismatch: manifest names another audit key".into());
    }
    for (path, bytes) in [paths.log_dir.join("snapshot.bin"), paths.anchor.clone()]
        .iter()
        .zip(&frames[2..4])
    {
        write_new(path, bytes, 0o600)?;
    }
    let lines: &[&[u8]] = &frames[4..];
    let (mut log, _tail) = FrontierLog::open(FileLeafStore::open(&paths.log_dir)?)?;
    let appended = log.append_batch(lines)?;
    if appended.len() != lines.len() {
        return Err("FixtureAppendIncomplete: the batch did not answer every line".into());
    }
    drop(log);
    let store = FileLeafStore::open_read_only(&paths.log_dir)?;
    let pin = store.pinned();
    if store.origin() != "lys.local/secrets-audit"
        || pin.tree_size != LINES
        || to_hex(&pin.root) != manifest.root
    {
        return Err("FixturePinMismatch: manifest and recorded log differ".into());
    }
    Ok(folders)
}
