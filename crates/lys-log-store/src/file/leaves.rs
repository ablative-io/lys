//! The leaves directory: a leaf written under a temporary name and linked to
//! its own, the directory flushed, and the extent read back from the names.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::error::{StoreError, StoreResult};

use super::LEAF_NAME_WIDTH;

/// Distinguishes temporary leaf names made by one process.
static LEAF_TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// Takes the next sequence number from [`LEAF_TEMP_SEQUENCE`].
pub(super) fn next_process_sequence() -> u64 {
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
pub(super) const LEAF_TEMP_ATTEMPTS: u32 = 16;

/// The hidden temporary name a leaf is written under before it is linked:
/// `.<pid>-<index padded to LEAF_NAME_WIDTH>-<sequence>.tmp`.
///
/// It begins with `.` so that opening the store never counts it, and carries
/// the process id, the leaf index and a per-process sequence number so that no
/// two writers share one.
pub(super) fn leaf_temp_name(pid: u32, index: u64, sequence: u64) -> String {
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
pub(super) fn write_leaf_temp(
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
pub(super) fn link_leaf(tmp_path: &Path, leaf_path: &Path, index: u64) -> StoreResult<()> {
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
pub(super) fn fsync_dir(dir: &Path) -> StoreResult<()> {
    sync_dir(dir).map_err(|source| StoreError::Io {
        context: format!("failed to flush directory {} to disk", dir.display()),
        source,
    })
}

/// Opens a directory and flushes it; see [`fsync_dir`] for the platform scope.
#[cfg(unix)]
pub(super) fn sync_dir(dir: &Path) -> std::io::Result<()> {
    std::fs::File::open(dir).and_then(|handle| handle.sync_all())
}

#[cfg(not(unix))]
pub(super) fn sync_dir(_dir: &Path) -> std::io::Result<()> {
    Ok(())
}

/// Removes a file by path.
pub(super) fn remove_file(path: &Path) -> std::io::Result<()> {
    std::fs::remove_file(path)
}

/// The extent found from the pin forward, touching only the names the pin
/// does not cover.
///
/// The pin is written after the leaves it covers are durable, so the leaf
/// just below it must be a file, and the extent is the first index at or past
/// the pin with no file. A name one past that is a gap and is refused. A leaf
/// removed from below the pin is refused when it is read. Costs one lookup
/// per unpinned leaf plus two, whatever the log's size.
pub(super) fn probed_extent(dir: &Path, pinned: u64) -> StoreResult<u64> {
    let leaves_dir = dir.join("leaves");
    let named = |index: u64| leaves_dir.join(format!("{index:0LEAF_NAME_WIDTH$}"));
    if let Some(last_pinned) = pinned.checked_sub(1)
        && !named(last_pinned).is_file()
    {
        return Err(StoreError::Corrupt {
            path: dir.to_path_buf(),
            reason: format!(
                "the pin covers {pinned} leaves but leaf index {last_pinned} is not a file"
            ),
        });
    }
    let mut extent = pinned;
    while named(extent).is_file() {
        extent = extent.checked_add(1).ok_or_else(|| StoreError::Corrupt {
            path: dir.to_path_buf(),
            reason: "more leaves than u64 can index".to_string(),
        })?;
    }
    if let Some(beyond) = extent.checked_add(1)
        && named(beyond).exists()
    {
        return Err(StoreError::Corrupt {
            path: dir.to_path_buf(),
            reason: format!(
                "leaves are not contiguous: expected leaf index {extent}, found {beyond}"
            ),
        });
    }
    Ok(extent)
}

/// Enumerates `leaves/` and returns the contiguous extent.
///
/// Entries beginning with `.` are ignored — they can never be leaf names,
/// which are exactly 20 digits, so ignoring them cannot mask a missing or
/// extra leaf. Any other unexpected entry is corruption. The index set must be
/// exactly `0..n`. Its cost grows with the log, so it is the audit, never the
/// open.
pub(super) fn contiguous_extent(dir: &Path) -> StoreResult<u64> {
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
