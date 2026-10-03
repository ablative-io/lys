//! [`FileLeafStore`] — a directory-backed [`LeafStore`] over segment files.
//!
//! # Layout (`lys/log-dir/v2`)
//!
//! ```text
//! <dir>/
//!   log.json                          the format marker and the origin, written once
//!   leaves/segments/<first>           records, the segment named by its first leaf's index
//!   leaves/segments/<first>.offsets   one little-endian u64 per record, no flush of its own
//!   snapshot.bin                      the one snapshot slot, replaced whole
//! ```
//!
//! A record is the leaf's length, the leaf's bytes, whether a pin follows,
//! the pin when it does, and a CRC-32C over all of it; see [`segment`]. An
//! append of any size is one act: every record of it, the last carrying the
//! pin, is one write and one flush of the segment, and the offsets file is
//! written after without a flush of its own. So the store's head is the last
//! whole record that carries a pin, and a kill anywhere inside an act leaves
//! records nobody acknowledged: the writable open cuts them and says so, the
//! read-only open serves the head and says so, through
//! [`FileLeafStore::unfinished_tail`]. Nothing is ever adopted that was not
//! pinned. Segments roll between acts once past 64 MiB.
//!
//! Opening reads the marker and only the last segment's tail: entries of its
//! offsets file past the last whole record are cut, records past the last
//! entry are found by reading forward from it, and the head is found by
//! reading back from the end until a record carries a pin. Earlier segments
//! are trusted to their offsets; [`FileLeafStore::audit_leaves`] reads every
//! record of every segment when a whole-store check is wanted.
//!
//! A directory still in the v1 per-leaf layout is migrated once by the first
//! writable open ([`crate::migrate`]) and refused by name by a read-only one.

use std::io::Write;
use std::path::{Path, PathBuf};

use lys_core::merkle::{AppendOnlyTree, RawLeaf};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::{StoreError, StoreResult};
use crate::migrate::Migrated;
use crate::store::{LeafStore, PinnedRoot, batch_end};

mod crc32c;
mod segment;
mod snapshot_slot;
pub(crate) mod v1;

use segment::{Read1, append_offsets, offsets_path, push_record, read_record, segment_path};

/// Detection marker in `log.json`. A local-state version tag, not a wire
/// contract.
pub(crate) const LOG_DIR_FORMAT: &str = "lys/log-dir/v2";

/// `log.json` — the store's immutable identity.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct LogConfig {
    /// Always [`LOG_DIR_FORMAT`]; a detection marker only.
    format: String,
    /// The log's origin, pinned at creation.
    origin: String,
}

/// Bytes past the store's head that no acknowledged act wrote: the records
/// and part-records of an act a kill interrupted. A writable open cut them;
/// a read-only open left them and serves the head.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnfinishedTail {
    /// The segment they are in.
    pub segment: PathBuf,
    /// Where they begin: the end of the head's record.
    pub offset: u64,
    /// How many bytes they are.
    pub bytes: u64,
}

/// A directory-backed [`LeafStore`].
///
/// Holds no leaf bytes in memory: the segments' first indices, the extent and
/// the pin are established when the store opens, and leaves are read on
/// demand through their offsets file.
pub struct FileLeafStore {
    dir: PathBuf,
    origin: String,
    extent: u64,
    pinned: PinnedRoot,
    /// The first leaf index of every segment, ascending; appends go to the
    /// last.
    segments: Vec<u64>,
    /// The length of the last segment up to its last whole record.
    last_end: u64,
    /// The record offsets of the last segment, as the open checked them and
    /// appends extend them; the offsets file is written from these.
    last_offsets: Vec<u64>,
    /// The act whose durability this handle could not confirm, if any. While
    /// set, every write is refused until the store is reopened.
    durability_uncertain: Option<u64>,
    /// Set on a handle from [`FileLeafStore::open_read_only`], whose
    /// `append`, `pin` and `put_snapshot` refuse with [`StoreError::ReadOnly`].
    read_only: bool,
    /// Digest of the last snapshot this handle flushed, including its name.
    durable_snapshot: Option<[u8; 32]>,
    /// What the open found past the head, cut or left as the open's kind says.
    unfinished_tail: Option<UnfinishedTail>,
    /// The migration out of v1 this open performed, when it did.
    migrated: Option<Migrated>,
    /// The size a segment is rolled past, between acts.
    roll_bytes: u64,
}

impl std::fmt::Debug for FileLeafStore {
    /// Summarizes the store without reading or dumping leaf content.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FileLeafStore")
            .field("dir", &self.dir)
            .field("origin", &self.origin)
            .field("extent", &self.extent)
            .field("segments", &self.segments.len())
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
        let segments = segment::segments_dir(dir);
        std::fs::create_dir_all(&segments).map_err(|source| StoreError::Io {
            context: format!("failed to create log store directory {}", dir.display()),
            source,
        })?;
        let config = LogConfig {
            format: LOG_DIR_FORMAT.to_string(),
            origin: origin.to_string(),
        };
        write_durably(&config_path, &json_bytes(&config, "log config")?)?;
        write_durably(&segment_path(dir, 0), &[])?;
        write_durably(&offsets_path(dir, 0), &[])?;
        fsync_dir(&segments)?;
        fsync_dir(dir)?;
        // The one Merkle fact storage knows: zero leaves have exactly one
        // possible root, so computing it here cannot go wrong, whereas taking
        // it from the caller invites being handed a different one.
        Ok(Self {
            dir: dir.to_path_buf(),
            origin: config.origin,
            extent: 0,
            pinned: empty_pin(),
            segments: vec![0],
            last_end: 0,
            last_offsets: Vec::new(),
            durability_uncertain: None,
            read_only: false,
            durable_snapshot: None,
            unfinished_tail: None,
            migrated: None,
            roll_bytes: segment::ROLL_BYTES,
        })
    }

    /// Opens the store at `dir` for writing. A directory still in the v1
    /// layout is migrated first, once ([`crate::migrate::migrate_v1`]), and
    /// [`FileLeafStore::migrated`] says so. Only the last segment's tail is
    /// read; an act a kill interrupted is cut there and named by
    /// [`FileLeafStore::unfinished_tail`].
    ///
    /// # Errors
    ///
    /// [`StoreError::NotInitialized`] if `dir` is not a store,
    /// [`StoreError::Corrupt`] with the specific discrepancy for a malformed
    /// `log.json`, an unexpected entry under `leaves/segments/`, or segments
    /// that do not meet, [`StoreError::CorruptRecord`] for a record the head
    /// search cannot read, the migration's errors, and [`StoreError::Io`] on
    /// filesystem failure.
    pub fn open(dir: &Path) -> StoreResult<Self> {
        let migrated = crate::migrate::migrate_v1(dir)?;
        let mut store = Self::scan(dir, false)?;
        store.migrated = migrated;
        if let Some(tail) = store.unfinished_tail.clone() {
            store.cut(&tail)?;
        }
        // An offsets file cut short or run past the end is repaired from the
        // records that stand; a whole one is left as it is.
        let last = *store.segments.last().unwrap_or(&0);
        let offsets = offsets_path(dir, last);
        if segment::read_offsets(&offsets)? != store.last_offsets {
            segment::write_offsets(&offsets, &store.last_offsets)?;
        }
        Ok(store)
    }

    /// Opens the store at `dir` for a reader: the checks of
    /// [`FileLeafStore::open`] without creating, writing, renaming or
    /// removing any file. A v1 directory is refused
    /// [`StoreError::MigrationPending`]; an unfinished act is served at the
    /// head and named by [`FileLeafStore::unfinished_tail`].
    ///
    /// The handle's `append`, `pin` and `put_snapshot` refuse with
    /// [`StoreError::ReadOnly`].
    ///
    /// # Errors
    ///
    /// [`StoreError::MigrationPending`] for a v1 directory, and otherwise the
    /// errors of [`FileLeafStore::open`].
    pub fn open_read_only(dir: &Path) -> StoreResult<Self> {
        if v1::present(dir)? {
            return Err(StoreError::MigrationPending {
                path: dir.to_path_buf(),
            });
        }
        Self::scan(dir, true)
    }

    /// Roll segments past `bytes` instead of 64 MiB; for tests of the roll.
    #[cfg(test)]
    pub(crate) fn with_roll_bytes(mut self, bytes: u64) -> Self {
        self.roll_bytes = bytes;
        self
    }

    /// The store's directory.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// What the open found past the head: cut by a writable open, left by a
    /// read-only one. `None` when the last act was whole.
    pub fn unfinished_tail(&self) -> Option<&UnfinishedTail> {
        self.unfinished_tail.as_ref()
    }

    /// The migration out of the v1 layout this open performed, when it did.
    pub fn migrated(&self) -> Option<&Migrated> {
        self.migrated.as_ref()
    }

    /// Reads every record of every segment and checks each against its
    /// offset and checksum, the segments against each other, and the count
    /// against this store's extent.
    ///
    /// # Errors
    ///
    /// [`StoreError::CorruptRecord`] for a record that does not read,
    /// [`StoreError::Corrupt`] for a count or a segment boundary that does
    /// not agree, and [`StoreError::Io`] on filesystem failure.
    pub fn audit_leaves(&self) -> StoreResult<()> {
        let mut counted = 0;
        for (position, &first) in self.segments.iter().enumerate() {
            if first != counted {
                return Err(StoreError::Corrupt {
                    path: self.dir.clone(),
                    reason: format!(
                        "segment {first} begins at {first} but {counted} leaves precede it"
                    ),
                });
            }
            let path = segment_path(&self.dir, first);
            let expected_end = if position + 1 == self.segments.len() {
                self.last_end
            } else {
                self.segment_len(first)?
            };
            let offsets = segment::read_offsets(&offsets_path(&self.dir, first))?;
            let mut file = open_segment(&path)?;
            let mut offset = 0;
            let mut records = 0usize;
            while offset < expected_end {
                if offsets.get(records) != Some(&offset) {
                    return Err(StoreError::CorruptRecord {
                        segment: path,
                        offset,
                        reason: format!("the offsets file does not place record {records} here"),
                    });
                }
                match read_record(&mut file, offset, expected_end)
                    .map_err(|source| reading(&path, source))?
                {
                    Read1::Whole(record) => {
                        offset += record.bytes;
                        records += 1;
                    }
                    Read1::Short { bytes } => {
                        return Err(short(&path, offset, bytes));
                    }
                    Read1::Damaged { reason, .. } => {
                        return Err(StoreError::CorruptRecord {
                            segment: path,
                            offset,
                            reason,
                        });
                    }
                }
            }
            counted +=
                u64::try_from(records).map_err(|source| StoreError::LeafCountUnrepresentable {
                    count: self.extent,
                    source,
                })?;
        }
        if counted != self.extent {
            return Err(StoreError::Corrupt {
                path: self.dir.clone(),
                reason: format!(
                    "the segments hold {counted} leaves but this store's extent is {}",
                    self.extent
                ),
            });
        }
        Ok(())
    }

    /// Read the marker, list the segments, check the last one's tail and
    /// find the head. Writes nothing.
    fn scan(dir: &Path, read_only: bool) -> StoreResult<Self> {
        let config = read_identity(dir)?;
        let segments = list_segments(dir)?;
        // Every segment but the last is sealed: its offsets file is whole and
        // flushed by the roll, and the next segment begins where it ends.
        for pair in segments.windows(2) {
            let count = segment::read_offsets(&offsets_path(dir, pair[0]))?.len();
            let count =
                u64::try_from(count).map_err(|source| StoreError::LeafCountUnrepresentable {
                    count: pair[0],
                    source,
                })?;
            let next = pair[0]
                .checked_add(count)
                .ok_or_else(|| StoreError::Corrupt {
                    path: dir.to_path_buf(),
                    reason: format!("segment {} overflows the leaf index range", pair[0]),
                })?;
            if next != pair[1] {
                return Err(StoreError::Corrupt {
                    path: dir.to_path_buf(),
                    reason: format!(
                        "segment {} ends at {next} but the next segment begins at {}",
                        pair[0], pair[1]
                    ),
                });
            }
        }
        let last = *segments.last().ok_or_else(|| StoreError::Corrupt {
            path: dir.to_path_buf(),
            reason: "leaves/segments holds no segment".to_owned(),
        })?;
        let last_path = segment_path(dir, last);
        let last_len = file_len(&last_path)?;
        let mut checked = segment::check_tail(&last_path, &offsets_path(dir, last))?;
        let head = segment::pins_backwards(&last_path, &mut checked)?;
        let (extent, pinned, last_end) = match head {
            Some((position, pin)) => {
                let index = last
                    + u64::try_from(position).map_err(|source| {
                        StoreError::LeafCountUnrepresentable {
                            count: last,
                            source,
                        }
                    })?;
                if pin.tree_size != index + 1 {
                    return Err(StoreError::CorruptRecord {
                        segment: last_path.clone(),
                        offset: checked.offsets[position],
                        reason: format!(
                            "the record of leaf {index} carries a pin at tree size {}",
                            pin.tree_size
                        ),
                    });
                }
                let end = checked
                    .offsets
                    .get(position + 1)
                    .copied()
                    .unwrap_or(checked.end);
                (pin.tree_size, pin, end)
            }
            None if segments.len() == 1 => (0, empty_pin(), 0),
            None => {
                // The last segment holds no finished act: the head is the
                // last record of the one before, which a roll sealed.
                let previous = segments[segments.len() - 2];
                let pin = sealed_pin(dir, previous)?;
                if pin.tree_size != last {
                    return Err(StoreError::Corrupt {
                        path: dir.to_path_buf(),
                        reason: format!(
                            "segment {previous} ends pinned at {} but segment {last} follows it",
                            pin.tree_size
                        ),
                    });
                }
                (pin.tree_size, pin, 0)
            }
        };
        let kept = head.map_or(0, |(position, _)| position + 1);
        let mut last_offsets = checked.offsets;
        last_offsets.truncate(kept);
        let unfinished_tail = (last_end < last_len).then(|| UnfinishedTail {
            segment: last_path,
            offset: last_end,
            bytes: last_len - last_end,
        });
        Ok(Self {
            dir: dir.to_path_buf(),
            origin: config.origin,
            extent,
            pinned,
            segments,
            last_end,
            last_offsets,
            durability_uncertain: None,
            read_only,
            durable_snapshot: None,
            unfinished_tail,
            migrated: None,
            roll_bytes: segment::ROLL_BYTES,
        })
    }

    /// Cut an unfinished act: truncate the last segment at the head's end
    /// and flush it. The offsets file is written from the kept offsets by
    /// the open that follows.
    fn cut(&mut self, tail: &UnfinishedTail) -> StoreResult<()> {
        let file = std::fs::OpenOptions::new()
            .write(true)
            .open(&tail.segment)
            .map_err(|source| StoreError::Io {
                context: format!(
                    "failed to open segment {} to cut it",
                    tail.segment.display()
                ),
                source,
            })?;
        file.set_len(tail.offset).map_err(|source| StoreError::Io {
            context: format!(
                "failed to cut segment {} at {}",
                tail.segment.display(),
                tail.offset
            ),
            source,
        })?;
        crate::durability::sync_all(&file).map_err(|source| StoreError::Io {
            context: format!(
                "failed to flush segment {} after the cut",
                tail.segment.display()
            ),
            source,
        })?;
        drop(file);
        Ok(())
    }

    fn refuse_if_read_only(&self, operation: &'static str) -> StoreResult<()> {
        if self.read_only {
            return Err(StoreError::ReadOnly {
                path: self.dir.clone(),
                operation,
            });
        }
        Ok(())
    }

    /// A pin the store accepts moves the size forward or holds it; it never
    /// moves the size backwards, and at the held size it is the held root.
    fn check_pin(&self, pin: PinnedRoot) -> StoreResult<()> {
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
            use base64::Engine;
            let standard = base64::engine::general_purpose::STANDARD;
            return Err(StoreError::PinRootChanged {
                tree_size: pin.tree_size,
                held: standard.encode(self.pinned.root),
                offered: standard.encode(pin.root),
            });
        }
        Ok(())
    }

    fn segment_len(&self, first: u64) -> StoreResult<u64> {
        file_len(&segment_path(&self.dir, first))
    }

    /// Begin a new segment at `first` when the last one is past the roll
    /// size: seal the last offsets file, create the new files, flush the
    /// directory. The act that follows goes into the new segment.
    fn roll_if_due(&mut self, first: u64) -> StoreResult<()> {
        if self.last_end <= self.roll_bytes {
            return Ok(());
        }
        let last = *self.segments.last().unwrap_or(&0);
        let sealed = std::fs::File::open(offsets_path(&self.dir, last)).map_err(|source| {
            StoreError::Io {
                context: format!("failed to open offsets of segment {last} to seal it"),
                source,
            }
        })?;
        crate::durability::sync_all(&sealed).map_err(|source| StoreError::Io {
            context: format!("failed to flush offsets of segment {last}"),
            source,
        })?;
        drop(sealed);
        write_durably(&segment_path(&self.dir, first), &[])?;
        write_durably(&offsets_path(&self.dir, first), &[])?;
        fsync_dir(&segment::segments_dir(&self.dir))?;
        self.segments.push(first);
        self.last_end = 0;
        self.last_offsets.clear();
        Ok(())
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
        let position = self.segments.partition_point(|&first| first <= index);
        let first = self.segments[position.saturating_sub(1)];
        let path = segment_path(&self.dir, first);
        let in_last = position == self.segments.len();
        let recorded = if in_last {
            usize::try_from(index - first)
                .ok()
                .and_then(|at| self.last_offsets.get(at).copied())
        } else {
            read_offset_at(&offsets_path(&self.dir, first), index - first)?
        };
        let offset = recorded.ok_or_else(|| StoreError::CorruptRecord {
            segment: path.clone(),
            offset: 0,
            reason: format!("no offset is recorded for leaf {index}"),
        })?;
        let len = if in_last {
            self.last_end
        } else {
            self.segment_len(first)?
        };
        let mut file = open_segment(&path)?;
        match read_record(&mut file, offset, len).map_err(|source| reading(&path, source))? {
            Read1::Whole(record) => Ok(Some(record.leaf)),
            Read1::Short { bytes } => Err(short(&path, offset, bytes)),
            Read1::Damaged { reason, .. } => Err(StoreError::CorruptRecord {
                segment: path,
                offset,
                reason,
            }),
        }
    }

    /// A leaf is written only with its pin, as one act; the file store has no
    /// leaf-alone write.
    fn put_leaf(&mut self, index: u64, bytes: &[u8]) -> StoreResult<()> {
        self.refuse_if_read_only("write a leaf")?;
        Err(StoreError::LeafWithoutPin {
            index,
            bytes: bytes.len(),
        })
    }

    fn append(&mut self, index: u64, leaves: &[&[u8]], pin: PinnedRoot) -> StoreResult<()> {
        self.refuse_if_read_only("append leaves with their pin")?;
        if let Some(uncertain) = self.durability_uncertain {
            return Err(StoreError::ReopenRequired { index: uncertain });
        }
        let end = batch_end(index, leaves.len())?;
        if index < self.extent {
            return Err(StoreError::LeafAlreadyWritten { index });
        }
        if index > self.extent {
            return Err(StoreError::LeafWouldLeaveGap {
                index,
                next: self.extent,
            });
        }
        if pin.tree_size != end {
            return Err(StoreError::PinNotOfAppend {
                end,
                tree_size: pin.tree_size,
            });
        }
        self.check_pin(pin)?;
        if leaves.is_empty() {
            return self.pin(pin);
        }
        self.roll_if_due(index)?;
        let first = *self.segments.last().unwrap_or(&0);
        let mut buffer = Vec::new();
        let mut offsets = Vec::with_capacity(leaves.len());
        let last = leaves.len() - 1;
        for (position, bytes) in leaves.iter().enumerate() {
            offsets.push(self.last_end + bytes_len(&buffer)?);
            push_record(&mut buffer, bytes, (position == last).then_some(pin))?;
        }
        let written = bytes_len(&buffer)?;
        // Nothing is acknowledged until the one flush returns: from the first
        // byte written to that flush, any failure holds this handle until a
        // fresh open, which cuts whatever of the act is there.
        self.durability_uncertain = Some(index);
        let path = segment_path(&self.dir, first);
        let uncertain = |source| StoreError::LeafDurabilityUncertain { index, source };
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .map_err(uncertain)?;
        file.write_all(&buffer).map_err(uncertain)?;
        crate::durability::sync_all(&file).map_err(uncertain)?;
        drop(file);
        append_offsets(&offsets_path(&self.dir, first), &offsets)?;
        self.last_offsets.extend(offsets);
        self.last_end += written;
        self.extent = end;
        self.pinned = pin;
        self.durability_uncertain = None;
        Ok(())
    }

    fn pinned(&self) -> PinnedRoot {
        self.pinned
    }

    /// The pin rides on the act that made it, so a pin alone can only repeat
    /// the held one: equal is accepted and writes nothing; any other is
    /// refused by name.
    fn pin(&mut self, pin: PinnedRoot) -> StoreResult<()> {
        self.refuse_if_read_only("pin")?;
        if let Some(index) = self.durability_uncertain {
            return Err(StoreError::ReopenRequired { index });
        }
        self.check_pin(pin)?;
        if pin == self.pinned {
            return Ok(());
        }
        Err(StoreError::PinNotOfAppend {
            end: self.extent,
            tree_size: pin.tree_size,
        })
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

/// The pin of the empty tree.
fn empty_pin() -> PinnedRoot {
    let (root, tree_size) = AppendOnlyTree::<RawLeaf>::new().root().to_parts();
    PinnedRoot { tree_size, root }
}

fn bytes_len(buffer: &[u8]) -> StoreResult<u64> {
    u64::try_from(buffer.len()).map_err(|source| StoreError::LeafCountUnrepresentable {
        count: u64::MAX,
        source,
    })
}

fn reading(path: &Path, source: std::io::Error) -> StoreError {
    StoreError::Io {
        context: format!("failed to read segment {}", path.display()),
        source,
    }
}

fn short(path: &Path, offset: u64, bytes: u64) -> StoreError {
    StoreError::CorruptRecord {
        segment: path.to_path_buf(),
        offset,
        reason: format!("the record is short: {bytes} bytes of it are there"),
    }
}

fn file_len(path: &Path) -> StoreResult<u64> {
    Ok(std::fs::metadata(path)
        .map_err(|source| StoreError::Io {
            context: format!("failed to read the length of {}", path.display()),
            source,
        })?
        .len())
}

/// Reads `log.json` and answers the store's identity.
fn read_identity(dir: &Path) -> StoreResult<LogConfig> {
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
    Ok(config)
}

/// The first indices of the segments under `dir`, ascending. A name that is
/// neither a segment, its offsets file nor dot-prefixed is `Corrupt`.
fn list_segments(dir: &Path) -> StoreResult<Vec<u64>> {
    let segments_dir = segment::segments_dir(dir);
    let unreadable = |source| StoreError::Io {
        context: format!(
            "failed to read segments directory {}",
            segments_dir.display()
        ),
        source,
    };
    let entries = std::fs::read_dir(&segments_dir).map_err(unreadable)?;
    let mut firsts = Vec::new();
    let mut offsets = Vec::new();
    for entry in entries {
        let entry = entry.map_err(unreadable)?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            return Err(unexpected_entry(dir, &name.to_string_lossy()));
        };
        if name.starts_with('.') {
            continue;
        }
        let (digits, is_offsets) = match name.strip_suffix(segment::OFFSETS_SUFFIX) {
            Some(digits) => (digits, true),
            None => (name, false),
        };
        let index = (digits.len() == segment::NAME_WIDTH
            && digits.bytes().all(|byte| byte.is_ascii_digit()))
        .then(|| digits.parse::<u64>().ok())
        .flatten()
        .ok_or_else(|| unexpected_entry(dir, name))?;
        if is_offsets {
            offsets.push(index);
        } else {
            firsts.push(index);
        }
    }
    firsts.sort_unstable();
    offsets.sort_unstable();
    for index in &offsets {
        if firsts.binary_search(index).is_err() {
            return Err(StoreError::Corrupt {
                path: dir.to_path_buf(),
                reason: format!(
                    "offsets file {index:0width$}.offsets has no segment",
                    width = segment::NAME_WIDTH
                ),
            });
        }
    }
    if firsts.first() != Some(&0) {
        return Err(StoreError::Corrupt {
            path: dir.to_path_buf(),
            reason: "no segment begins at leaf 0".to_owned(),
        });
    }
    Ok(firsts)
}

fn unexpected_entry(dir: &Path, name: &str) -> StoreError {
    StoreError::Corrupt {
        path: dir.to_path_buf(),
        reason: format!("unexpected entry in leaves/segments: {name:?}"),
    }
}

/// The pin on the last record of the sealed segment beginning at `first`.
fn sealed_pin(dir: &Path, first: u64) -> StoreResult<PinnedRoot> {
    let path = segment_path(dir, first);
    let offsets = segment::read_offsets(&offsets_path(dir, first))?;
    let offset = *offsets.last().ok_or_else(|| StoreError::Corrupt {
        path: dir.to_path_buf(),
        reason: format!("sealed segment {first} holds no record"),
    })?;
    let len = file_len(&path)?;
    let mut file = open_segment(&path)?;
    match read_record(&mut file, offset, len).map_err(|source| reading(&path, source))? {
        Read1::Whole(record) => record.pin.ok_or_else(|| StoreError::CorruptRecord {
            segment: path.clone(),
            offset,
            reason: "the last record of a sealed segment carries no pin".to_owned(),
        }),
        Read1::Short { bytes } => Err(short(&path, offset, bytes)),
        Read1::Damaged { reason, .. } => Err(StoreError::CorruptRecord {
            segment: path,
            offset,
            reason,
        }),
    }
}

fn open_segment(path: &Path) -> StoreResult<std::fs::File> {
    std::fs::File::open(path).map_err(|source| StoreError::Io {
        context: format!("failed to open segment {}", path.display()),
        source,
    })
}

/// The offset of record `position` in the offsets file at `path`, if recorded.
fn read_offset_at(path: &Path, position: u64) -> StoreResult<Option<u64>> {
    use std::io::{Read, Seek, SeekFrom};
    let io = |source| StoreError::Io {
        context: format!("failed to read offsets file {}", path.display()),
        source,
    };
    let mut file = std::fs::File::open(path).map_err(io)?;
    let at = position.checked_mul(8).ok_or_else(|| StoreError::Corrupt {
        path: path.to_path_buf(),
        reason: format!("record {position} is past the offsets file's range"),
    })?;
    file.seek(SeekFrom::Start(at)).map_err(io)?;
    let mut entry = [0u8; 8];
    match file.read_exact(&mut entry) {
        Ok(()) => Ok(Some(u64::from_le_bytes(entry))),
        Err(source) if source.kind() == std::io::ErrorKind::UnexpectedEof => Ok(None),
        Err(source) => Err(io(source)),
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

/// Flushes a directory so the names it holds survive a crash.
fn fsync_dir(dir: &Path) -> StoreResult<()> {
    let opened = std::fs::File::open(dir).map_err(|source| StoreError::Io {
        context: format!("failed to open directory {} to flush it", dir.display()),
        source,
    })?;
    crate::durability::sync_all(&opened).map_err(|source| StoreError::Io {
        context: format!("failed to flush directory {}", dir.display()),
        source,
    })
}

/// The segment file whose first leaf is `first`, for tests that cut or
/// damage it.
#[cfg(test)]
pub(crate) fn segment_file(dir: &Path, first: u64) -> PathBuf {
    segment_path(dir, first)
}

/// The record offsets the offsets file of segment `first` holds, for tests
/// that cut a segment at a record boundary.
#[cfg(test)]
pub(crate) fn segment_offsets(dir: &Path, first: u64) -> StoreResult<Vec<u64>> {
    segment::read_offsets(&offsets_path(dir, first))
}

#[cfg(test)]
#[path = "file_tests.rs"]
mod tests;
