//! The root each leaf completed, kept in a file beside the log, so a receipt
//! is rebuilt from its one leaf.
//!
//! A receipt names the root of the tree its leaf completed. The checkpoints
//! rebuild that root by reading up to
//! [`CHECKPOINT_EVERY`](crate::checkpoints::CHECKPOINT_EVERY) leaves; this
//! file keeps it instead. It holds one 32-byte root per leaf in log order,
//! the root at position `i` being the root of the tree of size `i + 1`,
//! written as each leaf is recorded. The index and tree size follow from the
//! position, and the leaf hash from the one leaf read.
//!
//! # Invariants
//!
//! - The file is local state derived from the log: never a wire contract,
//!   never signed, and believed only as far as it agrees with the log. When
//!   a log is opened with it, the last root it holds must be the log's root
//!   at that size; a file that disagrees is refused by name, logged, and
//!   rebuilt from the log's leaves.
//! - A file behind the log, a root not written before a stop or the whole
//!   file removed, is brought level from the log's leaves before it answers
//!   anything, and a file ahead of the log is cut back to it. Each is logged
//!   by name.
//! - A root that cannot be written or read is logged by name, and that
//!   receipt, or every later one until the next open brings the file level,
//!   is rebuilt from the checkpoints as it was before the file existed.

use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use lys_log_store::Frontier;

use crate::checkpoints::Checkpoints;

/// The bytes each root takes.
const ROOT_LEN: u64 = 32;

/// What bringing the roots level with a log reads of it.
pub(crate) struct Level<'a> {
    /// The snapshot domain of the log's owner, which each log line names.
    pub(crate) domain: &'static str,
    /// The number of leaves the log records.
    pub(crate) size: u64,
    /// The log's root at that size.
    pub(crate) root: [u8; 32],
    /// The log's checkpoints.
    pub(crate) checkpoints: &'a Checkpoints,
    /// The recorded leaf at an index, read from the store.
    pub(crate) leaf: &'a dyn Fn(u64) -> Result<Vec<u8>, String>,
}

/// The roots file, open for reading and writing at a root's place.
pub(crate) struct Roots {
    path: PathBuf,
    file: File,
    held: u64,
}

fn read_at(file: &File, offset: u64, root: &mut [u8; 32]) -> std::io::Result<()> {
    let mut file = file;
    file.seek(SeekFrom::Start(offset))?;
    file.read_exact(root)
}

fn write_at(file: &File, offset: u64, root: &[u8; 32]) -> std::io::Result<()> {
    let mut file = file;
    file.seek(SeekFrom::Start(offset))?;
    file.write_all(root)
}

impl Roots {
    /// The roots kept in the file at `path`, which is created empty when it
    /// does not exist. A torn last root is cut off.
    pub(crate) fn open(path: &Path) -> Result<Self, String> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)
            .map_err(|error| format!("opening {}: {error}", path.display()))?;
        let length = file
            .metadata()
            .map_err(|error| format!("measuring {}: {error}", path.display()))?
            .len();
        let mut roots = Self {
            path: path.to_owned(),
            file,
            held: length / ROOT_LEN,
        };
        if length % ROOT_LEN != 0 {
            roots.cut(roots.held)?;
        }
        Ok(roots)
    }

    /// Where the roots are kept.
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    /// How many roots are held.
    pub(crate) fn held(&self) -> u64 {
        self.held
    }

    /// The root the leaf at `index` completed, none when it is not held.
    pub(crate) fn root(&self, index: u64) -> Result<Option<[u8; 32]>, String> {
        if index >= self.held {
            return Ok(None);
        }
        let mut root = [0; 32];
        read_at(&self.file, index * ROOT_LEN, &mut root)
            .map_err(|error| format!("reading root {index} of {}: {error}", self.path.display()))?;
        Ok(Some(root))
    }

    /// Keep only the first `held` roots.
    pub(crate) fn cut(&mut self, held: u64) -> Result<(), String> {
        self.file
            .set_len(held * ROOT_LEN)
            .map_err(|error| format!("cutting {} to {held} roots: {error}", self.path.display()))?;
        self.held = held;
        Ok(())
    }

    /// Keep `root` as the root the next leaf completed.
    pub(crate) fn push(&mut self, root: &[u8; 32]) -> Result<(), String> {
        write_at(&self.file, self.held * ROOT_LEN, root).map_err(|error| {
            format!(
                "writing root {} of {}: {error}",
                self.held,
                self.path.display()
            )
        })?;
        self.held += 1;
        Ok(())
    }

    /// Bring the roots level with `log`: a file whose last root is not the
    /// log's root at that size is rebuilt from every leaf, one ahead of the
    /// log is cut back, and one behind is extended from the checkpoint at or
    /// below the roots it holds. Each is logged by name.
    pub(crate) fn level(&mut self, log: &Level<'_>) -> Result<(), String> {
        let (size, domain) = (log.size, log.domain);
        let held = self.held.min(size);
        if self.held > size {
            tracing::warn!(
                domain,
                "CoordinatesAhead: {} holds {} roots for a log of {size}; cut back",
                self.path.display(),
                self.held
            );
        }
        self.cut(held)?;
        if held == size && (size == 0 || self.root(size - 1)? == Some(log.root)) {
            return Ok(());
        }
        let mut frontier = log
            .checkpoints
            .before(held)
            .cloned()
            .ok_or_else(|| format!("no checkpoint is held below leaf {held}"))?;
        for at in frontier.size()..held {
            frontier.push(&(log.leaf)(at)?);
        }
        let mut from = held;
        if held > 0 && self.root(held - 1)? != Some(frontier.root()) {
            tracing::warn!(
                domain,
                "CoordinatesRebuilt: root {} of {} is not the log's; rebuilt from every leaf",
                held - 1,
                self.path.display()
            );
            self.cut(0)?;
            frontier = Frontier::new();
            from = 0;
        } else {
            tracing::warn!(
                domain,
                "CoordinatesBehind: {} holds {held} roots for a log of {size}; brought level from the log",
                self.path.display()
            );
        }
        for at in from..size {
            frontier.push(&(log.leaf)(at)?);
            self.push(&frontier.root())?;
        }
        Ok(())
    }
}
