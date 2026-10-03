//! The segment layout (LYSLOGSTORE-008 R1): leaves as length-prefixed,
//! checksummed records in `leaves/segments/<first index>`, with an offsets
//! file `<first index>.offsets` of one little-endian u64 per record beside
//! each.
//!
//! A record is the leaf's length as a little-endian u32, the leaf bytes, one
//! byte saying whether a pin follows, the pin when it does (tree size as a
//! little-endian u64 and the 32-byte root), and the CRC-32C of everything
//! before it. The pin rides on the last record of an act, so one act of any
//! size is one write and one flush of the segment, and the store's head is
//! the last whole record that carries a pin. Byte `k` of the leaf at record
//! offset `o` is at `o + 4 + k`.
//!
//! The offsets file is written after the act without a flush of its own. On
//! open only its tail is checked against the segment: entries past the last
//! whole record are cut, and records past the last entry are found by
//! reading forward from it, never from the start of the segment.

use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use super::crc32c::crc32c;
use crate::error::{StoreError, StoreResult};
use crate::store::PinnedRoot;

/// Width of a segment filename: `u64::MAX` has 20 decimal digits.
pub(super) const NAME_WIDTH: usize = 20;
/// The suffix of a segment's offsets file.
pub(super) const OFFSETS_SUFFIX: &str = ".offsets";
/// The size a segment is rolled past, between acts.
pub(super) const ROLL_BYTES: u64 = 64 * 1024 * 1024;

const LENGTH: usize = 4;
const FLAG: usize = 1;
const PIN: usize = 8 + 32;
const CRC: usize = 4;

/// A whole record, read back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Record {
    /// The leaf's bytes.
    pub(super) leaf: Vec<u8>,
    /// The pin the record carries, on the last record of an act.
    pub(super) pin: Option<PinnedRoot>,
    /// The record's length in the segment, header to checksum.
    pub(super) bytes: u64,
}

/// The directory of segments under the store at `dir`.
pub(super) fn segments_dir(dir: &Path) -> PathBuf {
    dir.join("leaves").join("segments")
}

/// The segment file whose first leaf is `first`.
pub(super) fn segment_path(dir: &Path, first: u64) -> PathBuf {
    segments_dir(dir).join(format!("{first:0NAME_WIDTH$}"))
}

/// The offsets file beside the segment whose first leaf is `first`.
pub(super) fn offsets_path(dir: &Path, first: u64) -> PathBuf {
    segments_dir(dir).join(format!("{first:0NAME_WIDTH$}{OFFSETS_SUFFIX}"))
}

/// Append one record for `leaf`, carrying `pin` when given, to `buffer`.
pub(super) fn push_record(
    buffer: &mut Vec<u8>,
    leaf: &[u8],
    pin: Option<PinnedRoot>,
) -> StoreResult<()> {
    let length = u32::try_from(leaf.len()).map_err(|source| StoreError::Io {
        context: format!("a leaf of {} bytes does not fit one record", leaf.len()),
        source: std::io::Error::new(std::io::ErrorKind::InvalidInput, source),
    })?;
    let start = buffer.len();
    buffer.extend_from_slice(&length.to_le_bytes());
    buffer.extend_from_slice(leaf);
    match pin {
        Some(pin) => {
            buffer.push(1);
            buffer.extend_from_slice(&pin.tree_size.to_le_bytes());
            buffer.extend_from_slice(&pin.root);
        }
        None => buffer.push(0),
    }
    let crc = crc32c(&buffer[start..]);
    buffer.extend_from_slice(&crc.to_le_bytes());
    Ok(())
}

/// What reading a record at an offset found.
#[derive(Debug)]
pub(super) enum Read1 {
    /// A whole record whose checksum holds.
    Whole(Record),
    /// The segment ends before the record does: `bytes` of it are there.
    Short { bytes: u64 },
    /// The record is whole in length but its checksum or shape does not hold.
    Damaged { bytes: u64, reason: String },
}

/// Read the record at `offset` of the segment `file`, of length `len`.
pub(super) fn read_record(
    file: &mut std::fs::File,
    offset: u64,
    len: u64,
) -> std::io::Result<Read1> {
    let available = len.saturating_sub(offset);
    if available < LENGTH as u64 {
        return Ok(Read1::Short { bytes: available });
    }
    file.seek(SeekFrom::Start(offset))?;
    let mut length = [0u8; LENGTH];
    file.read_exact(&mut length)?;
    let leaf_len = u64::from(u32::from_le_bytes(length));
    let without_pin = LENGTH as u64 + leaf_len + FLAG as u64 + CRC as u64;
    if available < without_pin {
        return Ok(Read1::Short { bytes: available });
    }
    let mut leaf = vec![0u8; usize::try_from(leaf_len).map_err(std::io::Error::other)?];
    file.read_exact(&mut leaf)?;
    let mut flag = [0u8; FLAG];
    file.read_exact(&mut flag)?;
    let pin_bytes = match flag[0] {
        0 => 0,
        1 => PIN as u64,
        other => {
            return Ok(Read1::Damaged {
                bytes: without_pin,
                reason: format!("pin flag is {other}, not 0 or 1"),
            });
        }
    };
    let total = without_pin + pin_bytes;
    if available < total {
        return Ok(Read1::Short { bytes: available });
    }
    let mut pin_raw = [0u8; PIN];
    let pin = if pin_bytes == 0 {
        None
    } else {
        file.read_exact(&mut pin_raw)?;
        let mut size = [0u8; 8];
        size.copy_from_slice(&pin_raw[..8]);
        let mut root = [0u8; 32];
        root.copy_from_slice(&pin_raw[8..]);
        Some(PinnedRoot {
            tree_size: u64::from_le_bytes(size),
            root,
        })
    };
    let mut crc = [0u8; CRC];
    file.read_exact(&mut crc)?;
    let mut checked = Vec::with_capacity(usize::try_from(total).map_err(std::io::Error::other)?);
    checked.extend_from_slice(&length);
    checked.extend_from_slice(&leaf);
    checked.extend_from_slice(&flag);
    if pin.is_some() {
        checked.extend_from_slice(&pin_raw);
    }
    if crc32c(&checked) != u32::from_le_bytes(crc) {
        return Ok(Read1::Damaged {
            bytes: total,
            reason: "CRC-32C does not match the record".to_owned(),
        });
    }
    Ok(Read1::Whole(Record {
        leaf,
        pin,
        bytes: total,
    }))
}

/// The offsets a segment's offsets file holds, in order.
pub(super) fn read_offsets(path: &Path) -> StoreResult<Vec<u64>> {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(source) => {
            return Err(StoreError::Io {
                context: format!("failed to read offsets file {}", path.display()),
                source,
            });
        }
    };
    // A short last entry is a write that never finished; it is cut with the
    // rest of the tail check.
    Ok(bytes
        .chunks_exact(8)
        .map(|chunk| {
            let mut entry = [0u8; 8];
            entry.copy_from_slice(chunk);
            u64::from_le_bytes(entry)
        })
        .collect())
}

/// Write `offsets` whole to `path`, without a flush of its own.
pub(super) fn write_offsets(path: &Path, offsets: &[u64]) -> StoreResult<()> {
    let mut bytes = Vec::with_capacity(offsets.len() * 8);
    for offset in offsets {
        bytes.extend_from_slice(&offset.to_le_bytes());
    }
    std::fs::write(path, bytes).map_err(|source| StoreError::Io {
        context: format!("failed to write offsets file {}", path.display()),
        source,
    })
}

/// Append `offsets` to the offsets file at `path`, without a flush of its own.
pub(super) fn append_offsets(path: &Path, offsets: &[u64]) -> StoreResult<()> {
    let mut bytes = Vec::with_capacity(offsets.len() * 8);
    for offset in offsets {
        bytes.extend_from_slice(&offset.to_le_bytes());
    }
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(path)
        .map_err(|source| StoreError::Io {
            context: format!("failed to open offsets file {}", path.display()),
            source,
        })?;
    file.write_all(&bytes).map_err(|source| StoreError::Io {
        context: format!("failed to append to offsets file {}", path.display()),
        source,
    })
}

/// A segment's tail, checked against its offsets: the whole records in
/// order with their offsets, and what lies past the last whole one.
#[derive(Debug)]
pub(super) struct Checked {
    /// The offset of every whole record, in order.
    pub(super) offsets: Vec<u64>,
    /// The pin of each whole record, in order.
    pub(super) pins: Vec<Option<PinnedRoot>>,
    /// The end of the last whole record.
    pub(super) end: u64,
    /// Bytes past the last whole record, and why they are not a record.
    pub(super) tail: Option<(u64, String)>,
}

/// Check the segment at `segment` against the offsets at `offsets`: entries
/// whose record is not whole are dropped from the end, and records past the
/// last good entry are found by reading forward from it. Only the tail is
/// read; the records before it are trusted to their offsets.
pub(super) fn check_tail(segment: &Path, offsets: &Path) -> StoreResult<Checked> {
    let mut file = std::fs::File::open(segment).map_err(|source| StoreError::Io {
        context: format!("failed to open segment {}", segment.display()),
        source,
    })?;
    let len = file
        .metadata()
        .map_err(|source| StoreError::Io {
            context: format!("failed to read the length of {}", segment.display()),
            source,
        })?
        .len();
    let mut known = read_offsets(offsets)?;
    // Offsets must ascend and lie within the segment; a run that does not is
    // cut with the tail, never trusted.
    while let Some(&last) = known.last() {
        let ascending = known.len() < 2 || known[known.len() - 2] < last;
        if ascending && last < len {
            break;
        }
        known.pop();
    }
    let io = |source| StoreError::Io {
        context: format!("failed to read segment {}", segment.display()),
        source,
    };
    // Drop entries from the end whose record is not whole.
    let mut pins: Vec<Option<PinnedRoot>> = Vec::new();
    let mut end = 0;
    let mut tail = None;
    while let Some(&last) = known.last() {
        match read_record(&mut file, last, len).map_err(io)? {
            Read1::Whole(record) => {
                end = last + record.bytes;
                pins = vec![None; known.len() - 1];
                pins.push(record.pin);
                break;
            }
            Read1::Short { .. } | Read1::Damaged { .. } => {
                known.pop();
            }
        }
    }
    // Read forward from the last good entry for records the offsets file
    // never recorded.
    loop {
        match read_record(&mut file, end, len).map_err(io)? {
            Read1::Whole(record) => {
                known.push(end);
                pins.push(record.pin);
                end += record.bytes;
            }
            Read1::Short { bytes } => {
                if bytes > 0 {
                    tail = Some((bytes, format!("{bytes} bytes short of a whole record")));
                }
                break;
            }
            Read1::Damaged { reason, .. } => {
                tail = Some((len - end, reason));
                break;
            }
        }
    }
    Ok(Checked {
        offsets: known,
        pins,
        end,
        tail,
    })
}

/// The pins of the records at `offsets` that `check_tail` did not read,
/// filled in backwards from the end until one carries a pin. Only as many
/// records as it takes to find the head are read.
pub(super) fn pins_backwards(
    segment: &Path,
    checked: &mut Checked,
) -> StoreResult<Option<(usize, PinnedRoot)>> {
    let mut file = std::fs::File::open(segment).map_err(|source| StoreError::Io {
        context: format!("failed to open segment {}", segment.display()),
        source,
    })?;
    let len = checked.end;
    for position in (0..checked.offsets.len()).rev() {
        if checked.pins[position].is_none() && position + 1 < checked.offsets.len() {
            // Not read by the tail check; read it now.
            match read_record(&mut file, checked.offsets[position], len).map_err(|source| {
                StoreError::Io {
                    context: format!("failed to read segment {}", segment.display()),
                    source,
                }
            })? {
                Read1::Whole(record) => checked.pins[position] = record.pin,
                Read1::Short { bytes } => {
                    return Err(StoreError::CorruptRecord {
                        segment: segment.to_path_buf(),
                        offset: checked.offsets[position],
                        reason: format!("the record is short: {bytes} bytes of it are there"),
                    });
                }
                Read1::Damaged { reason, .. } => {
                    return Err(StoreError::CorruptRecord {
                        segment: segment.to_path_buf(),
                        offset: checked.offsets[position],
                        reason,
                    });
                }
            }
        }
        if let Some(pin) = checked.pins[position] {
            return Ok(Some((position, pin)));
        }
    }
    Ok(None)
}
