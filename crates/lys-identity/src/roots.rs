//! The root each leaf of a log completed, kept beside the log, so a receipt
//! is rebuilt from its one leaf and one stored root rather than by reading
//! every leaf from the nearest checkpoint up to it.
//!
//! One file of 32-byte records: record `i` is the root of the tree of the
//! first `i + 1` leaves, so a leaf's coordinate is its index, `i + 1`, the
//! record, and the hash of the leaf read back. It is local state derived
//! wholly from the log, never a wire contract, and nothing is signed over it.
//!
//! # What is believed, and when it is rebuilt
//!
//! A record is written as its leaf is recorded, and the file is flushed to
//! the disk before every snapshot, so every record below a snapshot's size
//! was durable before that snapshot was. A start keeps the records below the
//! snapshot's size, drops any after it, and checks the record at the last
//! checkpoint it holds against that checkpoint's root, and the record at the
//! snapshot's size against the root the snapshot signs. A file missing,
//! shorter than the snapshot, or failing the check is rebuilt from the log,
//! by name and logged: the leaves from the checkpoint at or below the first
//! missing record are read and folded again. The roots of the leaves the
//! start read after the snapshot are then written from the fold itself.
//!
//! A file that cannot be opened or written is logged by name and set aside:
//! every receipt is then rebuilt from the nearest checkpoint, as it was
//! before the file existed, and the next start rebuilds the file.

use std::fs::{File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use crate::checkpoints::Checkpoints;

/// The width of one record: a SHA-256 root.
const RECORD: u64 = 32;

/// The roots file, open, and how many records it holds.
#[derive(Debug)]
struct Roots {
    file: File,
    len: u64,
}

/// Where the roots are kept beside the log, and the file while it is usable.
#[derive(Debug, Default)]
pub(crate) struct Beside {
    path: Option<PathBuf>,
    roots: Option<Roots>,
}

impl Roots {
    /// The roots at `path` for a log whose first `size` leaves the start did
    /// not read, the tree of which has the root `root`, rebuilt from `leaf`
    /// where the file does not hold them, then followed by the roots of the
    /// leaves after them, `tail`.
    fn kept(
        path: &Path,
        (size, root): (u64, &[u8; 32]),
        tail: &[[u8; 32]],
        checkpoints: &Checkpoints,
        leaf: impl Fn(u64) -> io::Result<Vec<u8>>,
    ) -> io::Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)?;
        let bytes = file.metadata()?.len();
        let mut roots = Self {
            file,
            len: (bytes / RECORD).min(size),
        };
        let whole = roots.len == size && size > 0;
        if !roots.agrees(checkpoints)? || (whole && roots.get(size - 1)? != Some(*root)) {
            tracing::warn!(
                "RootsRefused: {} does not hold the roots of the snapshot and the checkpoints it covers; it is rebuilt from the log",
                path.display()
            );
            roots.len = 0;
        }
        let held = roots.len;
        if held < size {
            tracing::warn!(
                "RootsRebuilt: {} holds the roots of {held} leaves, not {size}; the rest are read from the log",
                path.display()
            );
            let Some(checkpoint) = checkpoints.before(held) else {
                return Err(io::Error::other(format!(
                    "no checkpoint is held below leaf {held}"
                )));
            };
            let mut frontier = checkpoint.clone();
            for index in frontier.size()..size {
                frontier.push(&leaf(index)?);
                if index >= held {
                    roots.put(index, &frontier.root())?;
                }
            }
        }
        for root in tail {
            roots.put(roots.len, root)?;
        }
        roots.file.set_len(roots.len * RECORD)?;
        roots.file.sync_data()?;
        Ok(roots)
    }

    /// Whether the record at the last checkpoint the file covers is that
    /// checkpoint's root; true when it covers none.
    fn agrees(&self, checkpoints: &Checkpoints) -> io::Result<bool> {
        let Some(last) = self.len.checked_sub(1) else {
            return Ok(true);
        };
        let Some(checkpoint) = checkpoints.before(last) else {
            return Ok(false);
        };
        let Some(index) = checkpoint.size().checked_sub(1) else {
            return Ok(true);
        };
        Ok(self.get(index)? == Some(checkpoint.root()))
    }

    /// The record at `index`, none past the last.
    fn get(&self, index: u64) -> io::Result<Option<[u8; 32]>> {
        if index >= self.len {
            return Ok(None);
        }
        let mut file = &self.file;
        file.seek(SeekFrom::Start(index * RECORD))?;
        let mut root = [0_u8; 32];
        file.read_exact(&mut root)?;
        Ok(Some(root))
    }

    /// Write the record at `index`, which must be the next.
    fn put(&mut self, index: u64, root: &[u8; 32]) -> io::Result<()> {
        if index != self.len {
            return Err(io::Error::other(format!(
                "the root of leaf {index} is not the next record, {}",
                self.len
            )));
        }
        self.file.seek(SeekFrom::Start(index * RECORD))?;
        self.file.write_all(root)?;
        self.len += 1;
        Ok(())
    }
}

impl Beside {
    /// The roots kept at `path` for a log whose snapshot is at `snapshot`,
    /// its size and root, as [`Roots`] keeps them, or set aside by name when
    /// they cannot be; none kept when there is no `path`.
    pub(crate) fn open(
        path: Option<PathBuf>,
        snapshot: (u64, &[u8; 32]),
        tail: &[[u8; 32]],
        checkpoints: &Checkpoints,
        leaf: impl Fn(u64) -> io::Result<Vec<u8>>,
    ) -> Self {
        let Some(at) = path else {
            return Self::default();
        };
        let roots = match Roots::kept(&at, snapshot, tail, checkpoints, leaf) {
            Ok(roots) => Some(roots),
            Err(error) => {
                tracing::error!("RootsNotKept: {}: {error}", at.display());
                None
            }
        };
        Self {
            path: Some(at),
            roots,
        }
    }

    /// Where the roots are kept, if anywhere.
    pub(crate) fn path(&self) -> Option<PathBuf> {
        self.path.clone()
    }

    /// The root the leaf at `index` completed, when it is held; none sends
    /// the caller to the nearest checkpoint.
    pub(crate) fn root(&self, index: u64) -> Option<[u8; 32]> {
        let roots = self.roots.as_ref()?;
        match roots.get(index) {
            Ok(root) => root,
            Err(error) => {
                tracing::error!("RootNotRead: leaf {index}: {error}");
                None
            }
        }
    }

    /// Keep the root the leaf at `index` completed. A root that cannot be
    /// written sets the file aside by name.
    pub(crate) fn push(&mut self, index: u64, root: &[u8; 32]) {
        let Some(roots) = self.roots.as_mut() else {
            return;
        };
        if let Err(error) = roots.put(index, root) {
            tracing::error!("RootsNotKept: leaf {index}: {error}");
            self.roots = None;
        }
    }

    /// Flush every root written to the disk, before a snapshot is. A flush
    /// that fails sets the file aside by name.
    pub(crate) fn sync(&mut self) {
        let Some(roots) = self.roots.as_mut() else {
            return;
        };
        if let Err(error) = roots.file.sync_data() {
            tracing::error!("RootsNotKept: {error}");
            self.roots = None;
        }
    }
}
