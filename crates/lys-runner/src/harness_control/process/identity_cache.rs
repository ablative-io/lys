//! What a start learned about an executable file, kept so the next start of
//! the same file does not learn it again.
//!
//! A managed start fingerprints the harness binary (a SHA-256 of every byte,
//! about 200 MB for Claude Code) and runs it with `--version` to check it
//! against the qualified list; it also fingerprints this runner's own binary.
//! None of that can change while the file does not, so each answer is kept
//! against the file's identity on disk (device, inode, size, modification
//! and change times, read from the opened file). A file replaced or written
//! since moves its inode or its change time, which no user can set, and is
//! learned again. A record that cannot be read or does not match is learned
//! again, never trusted; a record that cannot be written costs only the next
//! start. Nothing is kept until an owner names the folder ([`keep_in`]).

use std::fs::Metadata;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

static FOLDER: OnceLock<PathBuf> = OnceLock::new();

/// Keep what is learned in `folder`, said once per process.
pub fn keep_in(folder: PathBuf) {
    if FOLDER.set(folder).is_err() {
        crate::error::said("executable_cache_repeated: the cache folder was said twice; the first stands");
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Stamp {
    path: String,
    dev: u64,
    ino: u64,
    size: u64,
    mtime: i64,
    mtime_nsec: i64,
    ctime: i64,
    ctime_nsec: i64,
}

impl Stamp {
    fn of(path: &Path, metadata: &Metadata) -> Option<Self> {
        Some(Self {
            path: path.to_str()?.to_owned(),
            dev: metadata.dev(),
            ino: metadata.ino(),
            size: metadata.size(),
            mtime: metadata.mtime(),
            mtime_nsec: metadata.mtime_nsec(),
            ctime: metadata.ctime(),
            ctime_nsec: metadata.ctime_nsec(),
        })
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Known {
    stamp: Option<Stamp>,
    sha256: Option<String>,
    /// What `--version` printed, kept only for a probe that exited zero.
    version: Option<String>,
}

fn record_for(path: &Path) -> Option<PathBuf> {
    let folder = FOLDER.get()?;
    let name = crate::protocol::hex(&Sha256::digest(path.as_os_str().as_encoded_bytes()));
    Some(folder.join(format!("{name}.json")))
}

/// What is known of `path` while it still has `metadata`'s identity.
fn known(path: &Path, metadata: &Metadata) -> Option<Known> {
    let stamp = Stamp::of(path, metadata)?;
    let bytes = std::fs::read(record_for(path)?).ok()?;
    let known: Known = serde_json::from_slice(&bytes).ok()?;
    (known.stamp.as_ref() == Some(&stamp)).then_some(known)
}

fn remember(path: &Path, metadata: &Metadata, change: impl FnOnce(&mut Known)) {
    let (Some(record), Some(stamp)) = (record_for(path), Stamp::of(path, metadata)) else {
        return;
    };
    let mut kept = known(path, metadata).unwrap_or_default();
    kept.stamp = Some(stamp);
    change(&mut kept);
    let written = (|| -> std::io::Result<()> {
        if let Some(folder) = record.parent() {
            std::fs::create_dir_all(folder)?;
        }
        let bytes = serde_json::to_vec(&kept).map_err(std::io::Error::other)?;
        let temporary = record.with_extension(format!("{}.tmp", std::process::id()));
        std::fs::write(&temporary, bytes)?;
        std::fs::rename(&temporary, &record)
    })();
    if let Err(error) = written {
        crate::error::said(&format!(
            "executable_cache_unwritten: {}: {error}; the next start learns it again",
            record.display()
        ));
    }
}

/// The SHA-256 of the opened file at `path`, if it was learned while the file
/// had this identity.
pub fn sha256(path: &Path, metadata: &Metadata) -> Option<String> {
    known(path, metadata)?.sha256
}

/// Keep `sha256` for the file at `path` as it is now.
pub fn remember_sha256(path: &Path, metadata: &Metadata, sha256: &str) {
    remember(path, metadata, |known| known.sha256 = Some(sha256.to_owned()));
}

/// What `--version` printed for the file at `path`, if it was asked while the
/// file had its present identity and the probe exited zero.
pub fn version(path: &Path) -> Option<String> {
    let metadata = std::fs::metadata(path).ok()?;
    known(path, &metadata)?.version
}

/// Keep what a probe that exited zero printed for the file at `path`.
pub fn remember_version(path: &Path, reported: &str) {
    if let Ok(metadata) = std::fs::metadata(path) {
        remember(path, &metadata, |known| known.version = Some(reported.to_owned()));
    }
}
