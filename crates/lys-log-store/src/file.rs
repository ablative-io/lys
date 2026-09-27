//! [`FileLeafStore`] — a [`LeafStore`] backed by a directory of files.
//!
//! # Layout
//!
//! - `log.json` — immutable identity (`format`, `origin`); written once at
//!   creation and never rewritten.
//! - `leaves/<20-digit zero-padded index>` — one file per leaf, raw bytes
//!   verbatim. **The leaf file IS the RFC 6962 preimage**, so
//!   `(printf '\x00'; cat leaf-file) | shasum -a 256` is the leaf hash. A
//!   stranger with `shasum` can check a leaf without lys. A leaf is written
//!   to a hidden temporary file (`leaves/.<pid>-<index>-<sequence>.tmp`),
//!   flushed, and only then linked to its final name by an operation that
//!   refuses to replace an existing leaf. A leaf name therefore never refers
//!   to a torn or unflushed file, and a leftover temporary file from a crash
//!   is never counted as a leaf.
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
//!
//! **Scope of that verification, stated rather than left to be assumed:** it was
//! established by *reading* the standard library's dispatch, on one platform. No
//! test here observes a power-loss outcome, and nothing has run these paths on a
//! filesystem that refuses `F_FULLFSYNC`. So the durability claim rests on the
//! platform contract plus error propagation, not on an experiment — which is the
//! honest strength for a property whose failure needs a crash to observe, and
//! which is a different and weaker axis of independence than the crate's Merkle
//! cross-checks, where two separately written implementations disagree or agree.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

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
    /// The leaf whose directory flush failed on this handle, if any. While set,
    /// every append is refused until the store is reopened.
    durability_uncertain: Option<u64>,
}

/// The two steps after a leaf is linked, kept as functions so that a test can
/// make either one fail.
struct AfterLink {
    /// Removes the temporary name the leaf was written under.
    remove_temp: fn(&Path) -> std::io::Result<()>,
    /// Flushes the leaves directory so the leaf's name is durable.
    flush_dir: fn(&Path) -> std::io::Result<()>,
}

/// The real steps after a link.
const AFTER_LINK: AfterLink = AfterLink {
    remove_temp: remove_file,
    flush_dir: sync_dir,
};

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
            durability_uncertain: None,
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
        // A leaf name linked just before a crash may not yet be durable; the
        // flush makes every name counted below one that survives.
        fsync_dir(&dir.join("leaves"))?;
        Ok(Self {
            dir: dir.to_path_buf(),
            origin: config.origin,
            extent: contiguous_extent(dir)?,
            pinned,
            durability_uncertain: None,
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

    /// Writes the leaf at `index` with `write_contents` supplying its bytes.
    ///
    /// The bytes go to a hidden temporary file in `leaves/`, which is flushed
    /// and only then linked to the leaf's final name, so no leaf name ever
    /// refers to a partly written or unflushed file. The link refuses to
    /// replace an existing leaf: that is the second, independent absent-check
    /// below the in-memory extent, catching a leaf this store never saw —
    /// another writer appending to the same directory — which the extent cached
    /// at open cannot know about. The directory is flushed after the link so
    /// the name itself is durable.
    ///
    /// The link is the commit point: once it succeeds the extent advances
    /// whatever happens next. A temporary name that cannot be removed is left
    /// for open to skip, and the append still succeeds. A directory that cannot
    /// be flushed is reported as [`StoreError::LeafDurabilityUncertain`] and
    /// halts this handle.
    ///
    /// `next_sequence` supplies the sequence numbers for temporary names, so
    /// that a test can say which names a write will try.
    fn put_leaf_with(
        &mut self,
        index: u64,
        write_contents: impl FnOnce(&mut std::fs::File) -> std::io::Result<()>,
        after_link: &AfterLink,
        next_sequence: &mut impl FnMut() -> u64,
    ) -> StoreResult<()> {
        if let Some(uncertain) = self.durability_uncertain {
            return Err(StoreError::ReopenRequired { index: uncertain });
        }
        if index < self.extent {
            return Err(StoreError::LeafAlreadyWritten { index });
        }
        if index > self.extent {
            return Err(StoreError::LeafWouldLeaveGap {
                index,
                next: self.extent,
            });
        }
        let leaves_dir = self.dir.join("leaves");
        let tmp_path = write_leaf_temp(&leaves_dir, index, write_contents, next_sequence)?;
        link_leaf(&tmp_path, &self.leaf_path(index), index)?;
        self.extent += 1;
        // The leaf is committed under its final name, and open skips a hidden
        // temporary name, so a name left behind here costs nothing but space.
        let left_behind = (after_link.remove_temp)(&tmp_path);
        drop(left_behind);
        (after_link.flush_dir)(&leaves_dir).map_err(|source| {
            self.durability_uncertain = Some(index);
            StoreError::LeafDurabilityUncertain { index, source }
        })
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
        self.put_leaf_with(
            index,
            |file| file.write_all(bytes),
            &AFTER_LINK,
            &mut next_process_sequence,
        )
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
        // Monotonicity is about the size and says nothing about the root, so a
        // second root at an already-pinned size passes it. That is equivocation
        // arriving through the one operation allowed to repeat.
        if pin.tree_size == self.pinned.tree_size && pin.root != self.pinned.root {
            return Err(StoreError::PinRootChanged {
                tree_size: pin.tree_size,
                held: STANDARD.encode(self.pinned.root),
                offered: STANDARD.encode(pin.root),
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

/// Distinguishes temporary leaf names made by one process.
static LEAF_TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// Takes the next sequence number from [`LEAF_TEMP_SEQUENCE`].
fn next_process_sequence() -> u64 {
    LEAF_TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed)
}

/// How many temporary names one leaf write tries before refusing.
///
/// A name is taken only by a leftover from an earlier process that had the
/// same process id and wrote the same index, so a write meets one such name
/// rarely and several in a row almost never. Each taken name costs one
/// sequence number and is left untouched. A write that finds every name in
/// this bound taken is not meeting leftovers but something else creating
/// names in `leaves/`, and refusing says so where trying further would hide it.
const LEAF_TEMP_ATTEMPTS: u32 = 16;

/// The hidden temporary name a leaf is written under before it is linked:
/// `.<pid>-<index padded to LEAF_NAME_WIDTH>-<sequence>.tmp`.
///
/// It begins with `.` so that opening the store never counts it, and carries
/// the process id, the leaf index and a per-process sequence number so that no
/// two writers share one.
fn leaf_temp_name(pid: u32, index: u64, sequence: u64) -> String {
    format!(".{pid}-{index:0LEAF_NAME_WIDTH$}-{sequence}.tmp")
}

/// Writes a leaf's bytes to a fresh hidden file in `leaves_dir` and flushes it.
///
/// The file is named by [`leaf_temp_name`] and created only if the name is
/// free. A name already taken is a leftover, which is never replaced: the
/// write takes the next sequence number from `next_sequence` and tries
/// again, up to
/// [`LEAF_TEMP_ATTEMPTS`] names. On any failure the file is removed: a write
/// that fails to flush can leave bytes in the page cache that read back as if
/// they were durable, and nothing may be linked from them.
fn write_leaf_temp(
    leaves_dir: &Path,
    index: u64,
    write_contents: impl FnOnce(&mut std::fs::File) -> std::io::Result<()>,
    next_sequence: &mut impl FnMut() -> u64,
) -> StoreResult<PathBuf> {
    let (tmp_path, mut file) = create_leaf_temp(leaves_dir, index, next_sequence)?;
    let written = write_contents(&mut file)
        .map_err(|source| StoreError::Io {
            context: format!("failed to write temporary leaf file {}", tmp_path.display()),
            source,
        })
        .and_then(|()| {
            file.sync_all().map_err(|source| StoreError::Io {
                context: format!(
                    "failed to flush temporary leaf file {} to disk",
                    tmp_path.display()
                ),
                source,
            })
        });
    drop(file);
    match written {
        Ok(()) => Ok(tmp_path),
        Err(err) => Err(discard_temp(&tmp_path, err)),
    }
}

/// Creates the temporary file for the leaf at `index` under the first free
/// name, never opening a name that already exists. Each name tried takes one
/// number from `next_sequence`.
fn create_leaf_temp(
    leaves_dir: &Path,
    index: u64,
    next_sequence: &mut impl FnMut() -> u64,
) -> StoreResult<(PathBuf, std::fs::File)> {
    let pid = std::process::id();
    for _ in 0..LEAF_TEMP_ATTEMPTS {
        let sequence = next_sequence();
        let tmp_path = leaves_dir.join(leaf_temp_name(pid, index, sequence));
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp_path)
        {
            Ok(file) => return Ok((tmp_path, file)),
            Err(source) if source.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(source) => {
                return Err(StoreError::Io {
                    context: format!(
                        "failed to create temporary leaf file {}",
                        tmp_path.display()
                    ),
                    source,
                });
            }
        }
    }
    Err(StoreError::LeafTempNamesTaken {
        index,
        attempts: LEAF_TEMP_ATTEMPTS,
        path: leaves_dir.to_path_buf(),
    })
}

/// Links a flushed temporary leaf file to its final name, never replacing one.
///
/// A hard link fails with `AlreadyExists` when the name is taken, which is the
/// refusal a second writer on the same index receives. On any failure the
/// temporary file is removed.
fn link_leaf(tmp_path: &Path, leaf_path: &Path, index: u64) -> StoreResult<()> {
    match std::fs::hard_link(tmp_path, leaf_path) {
        Ok(()) => Ok(()),
        Err(source) if source.kind() == std::io::ErrorKind::AlreadyExists => Err(discard_temp(
            tmp_path,
            StoreError::LeafAlreadyWritten { index },
        )),
        Err(source) => Err(discard_temp(
            tmp_path,
            StoreError::Io {
                context: format!(
                    "failed to link leaf file {} from {}",
                    leaf_path.display(),
                    tmp_path.display()
                ),
                source,
            },
        )),
    }
}

/// Removes an abandoned temporary leaf file, keeping the error that abandoned
/// it. A removal that also fails is reported alongside rather than dropped.
fn discard_temp(tmp_path: &Path, err: StoreError) -> StoreError {
    match std::fs::remove_file(tmp_path) {
        Ok(()) => err,
        Err(source) => StoreError::Io {
            context: format!(
                "{err}; the temporary leaf file {} could not be removed either",
                tmp_path.display()
            ),
            source,
        },
    }
}

/// Fsyncs a directory so that entries created or renamed inside it are durable.
///
/// Unix only: opening a directory as a file is not portable, and on platforms
/// where it is unavailable the file fsyncs above still apply while the
/// *directory entry* carries the platform's own weaker guarantee. Said plainly
/// rather than papered over, because a durability claim that quietly does not
/// hold on some target is worse than one scoped to where it does.
fn fsync_dir(dir: &Path) -> StoreResult<()> {
    sync_dir(dir).map_err(|source| StoreError::Io {
        context: format!("failed to flush directory {} to disk", dir.display()),
        source,
    })
}

/// Opens a directory and flushes it; see [`fsync_dir`] for the platform scope.
#[cfg(unix)]
fn sync_dir(dir: &Path) -> std::io::Result<()> {
    std::fs::File::open(dir).and_then(|handle| handle.sync_all())
}

#[cfg(not(unix))]
fn sync_dir(_dir: &Path) -> std::io::Result<()> {
    Ok(())
}

/// Removes a file by path.
fn remove_file(path: &Path) -> std::io::Result<()> {
    std::fs::remove_file(path)
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
