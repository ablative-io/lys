//! The open path of a v2 directory: read the marker, list the segments,
//! check the last one's tail, find the head, and cut an unfinished act for a
//! writable open. Nothing here writes except [`cut`].

use std::path::Path;

use crate::error::{StoreError, StoreResult};
use crate::store::PinnedRoot;

use super::segment::{self, Read1, offsets_path, read_record, segment_path};
use super::{
    FileLeafStore, LOG_DIR_FORMAT, LogConfig, UnfinishedTail, empty_pin, file_len, open_segment,
    parse_state_file, reading, short,
};

/// Read the marker, list the segments, check the last one's tail and find
/// the head. Writes nothing.
pub(super) fn scan(dir: &Path, read_only: bool) -> StoreResult<FileLeafStore> {
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
    Ok(FileLeafStore {
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

/// Cut an unfinished act: truncate the last segment at the head's end and
/// flush it. The offsets file is written from the kept offsets by the open
/// that follows.
pub(super) fn cut(tail: &UnfinishedTail) -> StoreResult<()> {
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
