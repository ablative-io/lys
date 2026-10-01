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
//! - `snapshot.bin` — the log owner's signed snapshot, if one was written,
//!   replaced atomically in the same way.
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
//!
//! # A named leaf is whole
//!
//! A file under a 20-digit leaf name is always a whole, flushed leaf. Only
//! files whose names begin with `.` may be partial, and those are never counted
//! as leaves and never changed by an open.
//!
//! - A write that fails before a successful link removes its own temporary
//!   file, and no other.
//! - The no-replace link is the **commit point**: once it succeeds the leaf is
//!   this writer's and the extent advances, whatever happens after it.
//! - A failure to flush `leaves/` after the link is
//!   [`StoreError::LeafDurabilityUncertain`], and the handle refuses further
//!   appends until the store is reopened.
//! - A writable open ([`FileLeafStore::open`]) flushes `leaves/` before
//!   counting, so a leaf named at a writable open is durable, and the open
//!   fails when that flush fails.
//! - A read-only open ([`FileLeafStore::open_read_only`]) counts the named
//!   leaves without flushing, and its handle refuses to write a leaf, a pin or
//!   a snapshot with [`StoreError::ReadOnly`].
//! - **Open never deletes a leftover temporary file**, and neither open
//!   deletes, renames, truncates or writes any file in the store's directory.
//!   Clearing leftover temporary files that a crash stranded is not done here:
//!   it is a maintenance act of its own.
//!
//! The directory flushes above hold on unix targets only; elsewhere the flush
//! of a directory is a no-op, as the Durability section says.
//!
//! # What open does and never does
//!
//! [`FileLeafStore::open`] reads `log.json` and `state.json`, flushes
//! `leaves/` (on the targets where `fsync_dir` flushes a directory; see
//! Durability), counts the 20-digit leaf names and returns a writable handle.
//! The count starts at the pin: the leaf just below it must be a file, each
//! name past it is one lookup, and a name one past the last is a gap and
//! refused as [`StoreError::Corrupt`]. When the leaf below the pin is not a
//! file, the whole folder is listed and the names must be contiguous from 0.
//! [`FileLeafStore::audit_leaves`] lists the whole folder on request.
//!
//! [`FileLeafStore::open_read_only`] performs the same checks in the same order
//! without the flush, and creates, writes, renames, links and removes nothing.
//! It refuses a store exactly one leaf past its pin with
//! [`StoreError::RepairPending`], and otherwise returns a handle whose
//! `put_leaf`, `pin` and `put_snapshot` refuse with [`StoreError::ReadOnly`].
//!
//! Neither open reads leaf bytes, repairs a store or advances the pin: the
//! one-leaf repair of an interrupted append is [`Log::open`]'s, over a writable
//! handle. Neither open counts a dot-prefixed name as a leaf. An open **never
//! deletes**, renames or changes a leftover temporary file — it is skipped,
//! not tidied away — and the store's own are named by
//! [`FileLeafStore::leftover_temporaries`], which lists them when asked so
//! that neither open pays for a listing.
//!
//! [`FileLeafStore::leaf`](LeafStore::leaf) serves bytes it has not checked. A
//! leaf is proven whole only through [`Log::open`], which rebuilds the tree
//! from every stored leaf and compares it against the pinned root, so every
//! reader that wants a proven leaf goes through [`Log::open`].
//!
//! [`Log::open`]: crate::Log::open

use std::io::Write;
use std::path::{Path, PathBuf};

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use lys_core::merkle::{AppendOnlyTree, RawLeaf};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::{StoreError, StoreResult};
use crate::store::{LeafStore, PinnedRoot};

mod leaves;

#[cfg(test)]
use leaves::{LEAF_TEMP_ATTEMPTS, leaf_temp_name};
use leaves::{
    contiguous_extent, fsync_dir, link_leaf, next_process_sequence, probed_extent, remove_file,
    sync_dir, write_leaf_temp,
};

mod left_behind;
mod snapshot_slot;

pub use left_behind::LeftBehind;

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
    /// The temporary names this handle could not remove after a link, in the
    /// order it met them.
    left_behind: Vec<LeftBehind>,
    /// Set on a handle from [`FileLeafStore::open_read_only`], whose
    /// `put_leaf`, `pin` and `put_snapshot` refuse with
    /// [`StoreError::ReadOnly`].
    read_only: bool,
    /// Digest of the last snapshot this handle flushed, including its name.
    durable_snapshot: Option<[u8; 32]>,
    /// An opened or failed pin must be flushed before this handle can skip it.
    pin_uncertain: bool,
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
            left_behind: Vec::new(),
            read_only: false,
            durable_snapshot: None,
            pin_uncertain: false,
        })
    }

    /// Opens the store at `dir`, establishing contiguity and reading the pin.
    ///
    /// Leaf *contents* are not read here — that is the caller's business. The
    /// extent is found from the pin forward, so opening costs the same however
    /// long the log is; [`FileLeafStore::audit_leaves`] enumerates every name
    /// when a whole-folder check is wanted.
    ///
    /// # Errors
    ///
    /// [`StoreError::NotInitialized`] if `dir` is not a store,
    /// [`StoreError::Corrupt`] with the specific discrepancy for a malformed
    /// `log.json`/`state.json`, an unexpected entry in `leaves/`, or a gap in
    /// the index set, and [`StoreError::Io`] on filesystem failure.
    pub fn open(dir: &Path) -> StoreResult<Self> {
        let (config, pinned) = read_identity(dir)?;
        // A leaf name linked just before a crash may not yet be durable; the
        // flush makes every name counted below one that survives.
        fsync_dir(&dir.join("leaves"))?;
        Ok(Self {
            dir: dir.to_path_buf(),
            origin: config.origin,
            extent: probed_extent(dir, pinned.tree_size)?,
            pinned,
            durability_uncertain: None,
            left_behind: Vec::new(),
            read_only: false,
            durable_snapshot: None,
            pin_uncertain: true,
        })
    }

    /// Opens the store at `dir` for a reader: the checks of
    /// [`FileLeafStore::open`] in the same order, without flushing `leaves/`
    /// and without creating, writing, renaming, linking or removing any file.
    ///
    /// The handle's `put_leaf`, `pin` and `put_snapshot` refuse with [`StoreError::ReadOnly`].
    /// A store holding exactly one leaf past its pin is an interrupted append
    /// that only a writable open repairs, so it is refused rather than served
    /// at the pinned head. That is decided from the count and `state.json`
    /// alone; no leaf bytes are read. Over any other count, `Log::open`
    /// accepts a tree that matches the pin and refuses any other with
    /// [`StoreError::PinMismatch`] without pinning.
    ///
    /// # Errors
    ///
    /// [`StoreError::RepairPending`] if the store holds exactly one leaf past
    /// its pin, and otherwise the errors of [`FileLeafStore::open`]:
    /// [`StoreError::NotInitialized`], [`StoreError::Corrupt`] and
    /// [`StoreError::Io`].
    pub fn open_read_only(dir: &Path) -> StoreResult<Self> {
        let (config, pinned) = read_identity(dir)?;
        let extent = probed_extent(dir, pinned.tree_size)?;
        if pinned.tree_size.checked_add(1) == Some(extent) {
            return Err(StoreError::RepairPending {
                path: dir.to_path_buf(),
                pinned_size: pinned.tree_size,
                extent,
            });
        }
        Ok(Self {
            dir: dir.to_path_buf(),
            origin: config.origin,
            extent,
            pinned,
            durability_uncertain: None,
            left_behind: Vec::new(),
            read_only: true,
            durable_snapshot: None,
            pin_uncertain: false,
        })
    }

    /// The store's own leftover temporary leaf files in `leaves/`, by name in
    /// lexical order: each a `.<pid>-<20-digit index>-<sequence>.tmp` that a
    /// write left behind. Listed when asked, not at open, so that opening
    /// costs the same however long the log is.
    ///
    /// None is counted toward the extent and none is removed, renamed or
    /// changed. A dot-prefixed name of any other form is not reported.
    ///
    /// # Errors
    ///
    /// [`StoreError::Io`] if `leaves/` cannot be read.
    pub fn leftover_temporaries(&self) -> StoreResult<Vec<String>> {
        list_leftover_temporaries(&self.dir)
    }

    /// Refuses `operation` with [`StoreError::ReadOnly`] on a read-only handle.
    fn refuse_if_read_only(&self, operation: &'static str) -> StoreResult<()> {
        if self.read_only {
            return Err(StoreError::ReadOnly {
                path: self.dir.clone(),
                operation,
            });
        }
        Ok(())
    }

    /// Reads every name in `leaves/` and checks it is exactly `0..extent`.
    ///
    /// # Errors
    ///
    /// [`StoreError::Corrupt`] naming the unexpected entry, the gap, or an
    /// extent other than the one this handle holds, and [`StoreError::Io`] on
    /// filesystem failure.
    pub fn audit_leaves(&self) -> StoreResult<()> {
        let listed = contiguous_extent(&self.dir)?;
        if listed == self.extent {
            return Ok(());
        }
        Err(StoreError::Corrupt {
            path: self.dir.clone(),
            reason: format!(
                "the leaves folder holds {listed} leaves but this store's extent is {}",
                self.extent
            ),
        })
    }

    /// The directory this store occupies.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// The temporary names this handle could not remove after linking a
    /// leaf, each with the leaf it held and what the removal answered. Every
    /// leaf named here is stored: only its temporary name is still there.
    pub fn left_behind(&self) -> &[LeftBehind] {
        &self.left_behind
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
        // temporary name, so a name left behind here fails no append. It is
        // kept by name with its reason.
        if let Err(source) = (after_link.remove_temp)(&tmp_path) {
            self.left_behind.push(LeftBehind {
                index,
                path: tmp_path,
                source,
            });
        }
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
        match std::fs::read(&path) {
            Ok(bytes) => Ok(Some(bytes)),
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(source) => Err(StoreError::Io {
                context: format!("failed to read leaf file {}", path.display()),
                source,
            }),
        }
    }

    fn put_leaf(&mut self, index: u64, bytes: &[u8]) -> StoreResult<()> {
        self.refuse_if_read_only("write a leaf")?;
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
        self.refuse_if_read_only("pin")?;
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
        if pin == self.pinned && !self.pin_uncertain {
            return Ok(());
        }
        self.pin_uncertain = true;
        write_state(&self.dir, pin)?;
        self.pin_uncertain = false;
        self.pinned = pin;
        Ok(())
    }

    fn snapshot(&self) -> StoreResult<Option<Vec<u8>>> {
        snapshot_slot::read(&self.dir)
    }

    fn put_snapshot(&mut self, bytes: &[u8]) -> StoreResult<()> {
        self.refuse_if_read_only("write a snapshot")?;
        let digest: [u8; 32] = Sha256::digest(bytes).into();
        if self.durable_snapshot == Some(digest) {
            return Ok(());
        }
        // A failed replacement may have changed the name before its directory
        // flush failed. It invalidates even the previous successful value.
        self.durable_snapshot = None;
        snapshot_slot::write(&self.dir, bytes)?;
        self.durable_snapshot = Some(digest);
        Ok(())
    }
}

/// Reads `log.json` and `state.json` and returns the store's identity and pin:
/// the checks both opens share, in the order they make them.
fn read_identity(dir: &Path) -> StoreResult<(LogConfig, PinnedRoot)> {
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
    Ok((config, pinned))
}

/// The names in `leaves/` of the store's temporary-leaf form, in lexical
/// order. Reads the directory listing only: no entry is opened, removed or
/// renamed.
fn list_leftover_temporaries(dir: &Path) -> StoreResult<Vec<String>> {
    let leaves_dir = dir.join("leaves");
    let unreadable = |source| StoreError::Io {
        context: format!("failed to read leaves directory {}", leaves_dir.display()),
        source,
    };
    let mut names = Vec::new();
    for entry in std::fs::read_dir(&leaves_dir).map_err(unreadable)? {
        let name = entry.map_err(unreadable)?.file_name();
        if let Some(name) = name.to_str()
            && is_leaf_temp_name(name)
        {
            names.push(name.to_string());
        }
    }
    names.sort_unstable();
    Ok(names)
}

/// Whether `name` has the form `leaves::leaf_temp_name` gives: a dot, decimal
/// digits, a dash, [`LEAF_NAME_WIDTH`] decimal digits, a dash, decimal digits
/// and `.tmp`.
fn is_leaf_temp_name(name: &str) -> bool {
    let digits = |part: &str| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit());
    let Some(inner) = name
        .strip_prefix('.')
        .and_then(|rest| rest.strip_suffix(".tmp"))
    else {
        return false;
    };
    let parts: Vec<&str> = inner.split('-').collect();
    matches!(
        parts.as_slice(),
        &[pid, index, sequence]
            if digits(pid) && digits(index) && index.len() == LEAF_NAME_WIDTH && digits(sequence)
    )
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
    crate::durability::sync_all(&file).map_err(|source| StoreError::Io {
        context: format!("failed to flush {} to disk", path.display()),
        source,
    })
}

#[cfg(test)]
mod left_behind_tests;

#[cfg(test)]
#[path = "file_tests.rs"]
mod tests;
