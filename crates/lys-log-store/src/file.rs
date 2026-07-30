//! [`FileLeafStore`] — a [`LeafStore`] backed by a directory of files.
//!
//! # Layout
//!
//! - `log.json` — immutable identity (`format`, `origin`); written once at
//!   creation and never rewritten.
//! - `leaves/<20-digit zero-padded index>` — one file per leaf, raw bytes
//!   verbatim. **The leaf file IS the RFC 6962 preimage**, so
//!   `(printf '\x00'; cat leaf-file) | shasum -a 256` is the leaf hash. A
//!   stranger with `shasum` can check a leaf without lys.
//! - `state.json` — the pinned `(tree_size, root)`, rewritten atomically after
//!   every append.
//!
//! This layout is **local state, not a wire contract**: nothing durable is
//! signed under it and it may change between lys versions. The format marker
//! and the filename width are nonetheless kept byte-identical to the layout
//! the `lys` CLI wrote before this crate existed, because gratuitously
//! orphaning working log directories is not an improvement.
//!
//! # Durability
//!
//! Every acknowledged write is fsynced before it returns, **and so is the
//! containing directory** — on POSIX a freshly created or renamed file is not
//! durable until its parent directory's entry is too, so syncing only the file
//! would leave `Ok` meaning "probably". The trait requires durable-on-return
//! and a storage layer that quietly means otherwise is the exact defect that
//! makes a log unrepairable rather than merely wrong.
//!
//! **On macOS the claim is stronger than plain `fsync(2)`, and this was checked
//! rather than assumed.** A bare `fsync` on Apple platforms returns once the
//! data has reached the drive, without waiting for the drive to flush its own
//! write cache; `fcntl(F_FULLFSYNC)` is the call that waits. Rust's
//! `File::sync_all` dispatches to `F_FULLFSYNC` under `#[cfg(target_vendor =
//! "apple")]` and to `libc::fsync` elsewhere, and it propagates the result
//! through `cvt_r` — so an unsupported operation surfaces as an error rather
//! than degrading silently into the weaker guarantee. Verified against the
//! toolchain's own `std` source, because "cannot determine" resolving quietly to
//! the reassuring answer is the failure mode this whole module is arranged
//! against.

use std::io::Write;
use std::path::{Path, PathBuf};

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use lys_core::merkle::{AppendOnlyTree, RawLeaf};
use serde::{Deserialize, Serialize};

use crate::error::{StoreError, StoreResult};
use crate::store::{LeafStore, PinnedRoot};

/// Detection marker in `log.json`. A local-state version tag, not a wire
/// contract.
const LOG_DIR_FORMAT: &str = "lys/log-dir/v1";

/// Width of a leaf filename: `u64::MAX` has 20 decimal digits.
const LEAF_NAME_WIDTH: usize = 20;

/// `log.json` — the store's immutable identity.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct LogConfig {
    /// Always [`LOG_DIR_FORMAT`]; a detection marker only.
    format: String,
    /// The log's origin, pinned at creation.
    origin: String,
}

/// `state.json` — the pinned `(tree_size, root)`.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct LogState {
    /// Number of leaves the pinned root covers.
    tree_size: u64,
    /// Standard base64 (with padding) of the 32-byte RFC 6962 root hash.
    root_hash: String,
}

/// A directory-backed [`LeafStore`].
///
/// Holds no leaf bytes in memory: `extent` and `pinned` are established when
/// the store opens, and leaves are read on demand. Callers that need every
/// leaf (to rebuild a tree) hold that memory themselves, where its cost is
/// visible.
pub struct FileLeafStore {
    dir: PathBuf,
    origin: String,
    extent: u64,
    pinned: PinnedRoot,
}

impl std::fmt::Debug for FileLeafStore {
    /// Summarizes the store without reading or dumping leaf content.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FileLeafStore")
            .field("dir", &self.dir)
            .field("origin", &self.origin)
            .field("extent", &self.extent)
            .finish_non_exhaustive()
    }
}

impl FileLeafStore {
    /// Creates a store at `dir` with the given origin, pinning the empty tree.
    ///
    /// # Errors
    ///
    /// [`StoreError::Trust`] if the origin violates the checkpoint-origin
    /// rules, [`StoreError::AlreadyInitialized`] if `dir` is already a store
    /// (the origin is fixed at creation, so this never runs twice), and
    /// [`StoreError::Io`] on filesystem failure.
    pub fn create(dir: &Path, origin: &str) -> StoreResult<Self> {
        crate::validate_origin(origin)?;
        let config_path = dir.join("log.json");
        if config_path.exists() {
            return Err(StoreError::AlreadyInitialized {
                path: dir.to_path_buf(),
            });
        }
        std::fs::create_dir_all(dir.join("leaves")).map_err(|source| StoreError::Io {
            context: format!("failed to create log store directory {}", dir.display()),
            source,
        })?;
        let config = LogConfig {
            format: LOG_DIR_FORMAT.to_string(),
            origin: origin.to_string(),
        };
        write_durably(&config_path, &json_bytes(&config, "log config")?)?;
        // The one Merkle fact storage knows: zero leaves have exactly one
        // possible root, so computing it here cannot go wrong, whereas taking
        // it from the caller invites being handed a different one.
        let (root, tree_size) = AppendOnlyTree::<RawLeaf>::new().root().to_parts();
        let pinned = PinnedRoot { tree_size, root };
        write_state(dir, pinned)?;
        Ok(Self {
            dir: dir.to_path_buf(),
            origin: config.origin,
            extent: 0,
            pinned,
        })
    }

    /// Opens the store at `dir`, establishing contiguity and reading the pin.
    ///
    /// Leaf *contents* are not read here — that is the caller's business — but
    /// the index set is enumerated and required to be exactly `0..n`, because
    /// [`LeafStore::extent`] promises contiguity and a promise checked later is
    /// not a promise.
    ///
    /// # Errors
    ///
    /// [`StoreError::NotInitialized`] if `dir` is not a store,
    /// [`StoreError::Corrupt`] with the specific discrepancy for a malformed
    /// `log.json`/`state.json`, an unexpected entry in `leaves/`, or a gap in
    /// the index set, and [`StoreError::Io`] on filesystem failure.
    pub fn open(dir: &Path) -> StoreResult<Self> {
        let config_path = dir.join("log.json");
        if !config_path.exists() {
            return Err(StoreError::NotInitialized {
                path: dir.to_path_buf(),
            });
        }
        let config: LogConfig = parse_state_file(dir, &config_path, "log.json")?;
        if config.format != LOG_DIR_FORMAT {
            return Err(StoreError::Corrupt {
                path: dir.to_path_buf(),
                reason: format!(
                    "log.json format is {:?}, expected {LOG_DIR_FORMAT:?}",
                    config.format
                ),
            });
        }
        let state: LogState = parse_state_file(dir, &dir.join("state.json"), "state.json")?;
        let pinned = PinnedRoot {
            tree_size: state.tree_size,
            root: decode_pinned_root(dir, &state.root_hash)?,
        };
        Ok(Self {
            dir: dir.to_path_buf(),
            origin: config.origin,
            extent: contiguous_extent(dir)?,
            pinned,
        })
    }

    /// The directory this store occupies.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Path of the leaf file for `index`.
    fn leaf_path(&self, index: u64) -> PathBuf {
        self.dir
            .join("leaves")
            .join(format!("{index:0LEAF_NAME_WIDTH$}"))
    }
}

impl LeafStore for FileLeafStore {
    fn origin(&self) -> &str {
        &self.origin
    }

    fn extent(&self) -> u64 {
        self.extent
    }

    fn leaf(&self, index: u64) -> StoreResult<Option<Vec<u8>>> {
        if index >= self.extent {
            return Ok(None);
        }
        let path = self.leaf_path(index);
        std::fs::read(&path)
            .map(Some)
            .map_err(|source| StoreError::Io {
                context: format!("failed to read leaf file {}", path.display()),
                source,
            })
    }

    fn put_leaf(&mut self, index: u64, bytes: &[u8]) -> StoreResult<()> {
        if index < self.extent {
            return Err(StoreError::LeafAlreadyWritten { index });
        }
        if index > self.extent {
            return Err(StoreError::LeafWouldLeaveGap {
                index,
                next: self.extent,
            });
        }
        let path = self.leaf_path(index);
        // `create_new` is a SECOND, independent absent-check, below the
        // in-memory extent: it catches a leaf file this store never saw —
        // another process appending to the same directory — which the extent
        // it cached at open cannot possibly know about.
        let mut file = match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(file) => file,
            Err(source) if source.kind() == std::io::ErrorKind::AlreadyExists => {
                return Err(StoreError::LeafAlreadyWritten { index });
            }
            Err(source) => {
                return Err(StoreError::Io {
                    context: format!("failed to create leaf file {}", path.display()),
                    source,
                });
            }
        };
        file.write_all(bytes).map_err(|source| StoreError::Io {
            context: format!("failed to write leaf file {}", path.display()),
            source,
        })?;
        file.sync_all().map_err(|source| StoreError::Io {
            context: format!("failed to flush leaf file {} to disk", path.display()),
            source,
        })?;
        fsync_dir(&self.dir.join("leaves"))?;
        self.extent += 1;
        Ok(())
    }

    fn pinned(&self) -> PinnedRoot {
        self.pinned
    }

    fn pin(&mut self, pin: PinnedRoot) -> StoreResult<()> {
        if pin.tree_size < self.pinned.tree_size {
            return Err(StoreError::PinWentBackwards {
                pinned: self.pinned.tree_size,
                requested: pin.tree_size,
            });
        }
        write_state(&self.dir, pin)?;
        self.pinned = pin;
        Ok(())
    }
}

/// Serializes a local-state struct as pretty JSON with a trailing newline.
fn json_bytes<T: Serialize>(value: &T, what: &'static str) -> StoreResult<Vec<u8>> {
    let mut json = serde_json::to_string_pretty(value)
        .map_err(|source| StoreError::Serialize { what, source })?;
    json.push('\n');
    Ok(json.into_bytes())
}

/// Parses a local-state JSON file into an actionable [`StoreError::Corrupt`].
fn parse_state_file<T: serde::de::DeserializeOwned>(
    dir: &Path,
    path: &Path,
    what: &str,
) -> StoreResult<T> {
    let bytes = std::fs::read(path).map_err(|source| StoreError::Io {
        context: format!("failed to read {what} {}", path.display()),
        source,
    })?;
    serde_json::from_slice(&bytes).map_err(|e| StoreError::Corrupt {
        path: dir.to_path_buf(),
        reason: format!("{what} is malformed: {e}"),
    })
}

/// Decodes the pinned root: canonical standard base64 of exactly 32 bytes.
fn decode_pinned_root(dir: &Path, root_b64: &str) -> StoreResult<[u8; 32]> {
    STANDARD
        .decode(root_b64)
        .ok()
        .and_then(|bytes| <[u8; 32]>::try_from(bytes).ok())
        .ok_or_else(|| StoreError::Corrupt {
            path: dir.to_path_buf(),
            reason: "state.json root_hash is not standard base64 of exactly 32 bytes".to_string(),
        })
}

/// Durably replaces `state.json`: write a sibling temp file, fsync it, rename
/// over the target (atomic on POSIX), then fsync the directory so the rename
/// itself survives a crash.
fn write_state(dir: &Path, pin: PinnedRoot) -> StoreResult<()> {
    let state = LogState {
        tree_size: pin.tree_size,
        root_hash: STANDARD.encode(pin.root),
    };
    let tmp_path = dir.join("state.json.tmp");
    write_durably(&tmp_path, &json_bytes(&state, "log state")?)?;
    let state_path = dir.join("state.json");
    std::fs::rename(&tmp_path, &state_path).map_err(|source| StoreError::Io {
        context: format!(
            "failed to atomically replace log state file {}",
            state_path.display()
        ),
        source,
    })?;
    fsync_dir(dir)
}

/// Writes `contents` to `path` and fsyncs the file before returning.
fn write_durably(path: &Path, contents: &[u8]) -> StoreResult<()> {
    let mut file = std::fs::File::create(path).map_err(|source| StoreError::Io {
        context: format!("failed to create {}", path.display()),
        source,
    })?;
    file.write_all(contents).map_err(|source| StoreError::Io {
        context: format!("failed to write {}", path.display()),
        source,
    })?;
    file.sync_all().map_err(|source| StoreError::Io {
        context: format!("failed to flush {} to disk", path.display()),
        source,
    })
}

/// Fsyncs a directory so that entries created or renamed inside it are durable.
///
/// Unix only: opening a directory as a file is not portable, and on platforms
/// where it is unavailable the file fsyncs above still apply while the
/// *directory entry* carries the platform's own weaker guarantee. Said plainly
/// rather than papered over, because a durability claim that quietly does not
/// hold on some target is worse than one scoped to where it does.
#[cfg(unix)]
fn fsync_dir(dir: &Path) -> StoreResult<()> {
    std::fs::File::open(dir)
        .and_then(|handle| handle.sync_all())
        .map_err(|source| StoreError::Io {
            context: format!("failed to flush directory {} to disk", dir.display()),
            source,
        })
}

#[cfg(not(unix))]
fn fsync_dir(_dir: &Path) -> StoreResult<()> {
    Ok(())
}

/// Enumerates `leaves/` and returns the contiguous extent.
///
/// Entries beginning with `.` are ignored — they can never be leaf names,
/// which are exactly 20 digits, so ignoring them cannot mask a missing or
/// extra leaf. Any other unexpected entry is corruption. The index set must be
/// exactly `0..n`.
fn contiguous_extent(dir: &Path) -> StoreResult<u64> {
    let leaves_dir = dir.join("leaves");
    let entries = std::fs::read_dir(&leaves_dir).map_err(|source| StoreError::Io {
        context: format!("failed to read leaves directory {}", leaves_dir.display()),
        source,
    })?;
    let mut indices: Vec<u64> = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|source| StoreError::Io {
            context: format!("failed to read leaves directory {}", leaves_dir.display()),
            source,
        })?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            return Err(invalid_leaf_entry(dir, &name.to_string_lossy()));
        };
        if name.starts_with('.') {
            continue;
        }
        if name.len() != LEAF_NAME_WIDTH || !name.bytes().all(|b| b.is_ascii_digit()) {
            return Err(invalid_leaf_entry(dir, name));
        }
        let Ok(index) = name.parse::<u64>() else {
            return Err(invalid_leaf_entry(dir, name));
        };
        if !entry.path().is_file() {
            return Err(invalid_leaf_entry(dir, name));
        }
        indices.push(index);
    }
    indices.sort_unstable();
    for (expected, &index) in (0u64..).zip(indices.iter()) {
        if index != expected {
            return Err(StoreError::Corrupt {
                path: dir.to_path_buf(),
                reason: format!(
                    "leaves are not contiguous: expected leaf index {expected}, found {index}"
                ),
            });
        }
    }
    u64::try_from(indices.len()).map_err(|_source| StoreError::Corrupt {
        path: dir.to_path_buf(),
        reason: format!("{} leaves is more than u64 can index", indices.len()),
    })
}

/// Builds the corruption error for an unexpected `leaves/` entry.
fn invalid_leaf_entry(dir: &Path, name: &str) -> StoreError {
    StoreError::Corrupt {
        path: dir.to_path_buf(),
        reason: format!(
            "unexpected entry {name:?} in leaves/: leaf names are exactly {LEAF_NAME_WIDTH} \
             decimal digits"
        ),
    }
}

#[cfg(test)]
#[path = "file_tests.rs"]
mod tests;
