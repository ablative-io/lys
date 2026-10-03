//! Damage a segment store the way a test needs to: a byte flipped inside a
//! leaf's record, a leaf replaced by other bytes behind a valid checksum, a
//! segment cut at or inside a record, and a leaf's bytes read back from its
//! record. Every helper finds the leaf's segment and offset from the store's
//! own files, so a test never spells the layout.

use std::path::{Path, PathBuf};

use super::segment::{
    Read1, offsets_path, push_record, read_offsets, read_record, segment_path, segments_dir,
};
use crate::store::PinnedRoot;

/// The segment holding `index` and the offset of its record in it.
fn locate(dir: &Path, index: u64) -> (u64, u64) {
    let mut firsts: Vec<u64> = std::fs::read_dir(segments_dir(dir))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .filter(|name| !name.contains('.'))
        .map(|name| name.parse().unwrap())
        .collect();
    firsts.sort_unstable();
    let first = *firsts
        .iter()
        .rev()
        .find(|first| **first <= index)
        .expect("a segment holds the leaf");
    let offsets = read_offsets(&offsets_path(dir, first)).unwrap();
    let position = usize::try_from(index - first).unwrap();
    (first, offsets[position])
}

/// The whole records of the segment whose first leaf is `first`, in order.
fn records(dir: &Path, first: u64) -> Vec<(Vec<u8>, Option<PinnedRoot>)> {
    let path = segment_path(dir, first);
    let len = std::fs::metadata(&path).unwrap().len();
    let mut file = std::fs::File::open(&path).unwrap();
    read_offsets(&offsets_path(dir, first))
        .unwrap()
        .into_iter()
        .map(
            |offset| match read_record(&mut file, offset, len).unwrap() {
                Read1::Whole(record) => (record.leaf, record.pin),
                other => panic!("record at {offset} is not whole: {other:?}"),
            },
        )
        .collect()
}

/// Rewrite the segment whose first leaf is `first` from `records`, with its
/// offsets file, every record carrying a checksum that holds.
fn rewrite(dir: &Path, first: u64, records: &[(Vec<u8>, Option<PinnedRoot>)]) {
    let mut buffer = Vec::new();
    let mut offsets = Vec::with_capacity(records.len());
    for (leaf, pin) in records {
        offsets.push(u64::try_from(buffer.len()).unwrap());
        push_record(&mut buffer, leaf, *pin).unwrap();
    }
    std::fs::write(segment_path(dir, first), buffer).unwrap();
    let mut bytes = Vec::with_capacity(offsets.len() * 8);
    for offset in offsets {
        bytes.extend_from_slice(&offset.to_le_bytes());
    }
    std::fs::write(offsets_path(dir, first), bytes).unwrap();
}

/// The segment file holding leaf `index`.
pub(crate) fn segment_of(dir: &Path, index: u64) -> PathBuf {
    segment_path(dir, locate(dir, index).0)
}

/// The bytes of leaf `index`, read from its record.
pub(crate) fn leaf_bytes(dir: &Path, index: u64) -> Vec<u8> {
    let (first, _) = locate(dir, index);
    let position = usize::try_from(index - first).unwrap();
    records(dir, first).swap_remove(position).0
}

/// Flip the first byte of leaf `index` inside its record, so the record's
/// checksum no longer holds.
pub(crate) fn tamper_leaf_byte(dir: &Path, index: u64) {
    let (first, offset) = locate(dir, index);
    let path = segment_path(dir, first);
    let mut bytes = std::fs::read(&path).unwrap();
    let at = usize::try_from(offset).unwrap() + 4;
    bytes[at] ^= 0x01;
    std::fs::write(&path, bytes).unwrap();
}

/// Replace leaf `index` with `planted` behind a checksum that holds, moving
/// the records after it, so only the tree can tell.
pub(crate) fn plant_leaf(dir: &Path, index: u64, planted: &[u8]) {
    let (first, _) = locate(dir, index);
    let mut held = records(dir, first);
    held[usize::try_from(index - first).unwrap()].0 = planted.to_vec();
    rewrite(dir, first, &held);
}

/// Replace the pin carried by the record of leaf `index` with `planted`,
/// behind a checksum that holds, so only the open's own reading of the pin
/// against the record's place can tell.
pub(crate) fn plant_pin(dir: &Path, index: u64, planted: PinnedRoot) {
    let (first, _) = locate(dir, index);
    let mut held = records(dir, first);
    held[usize::try_from(index - first).unwrap()].1 = Some(planted);
    rewrite(dir, first, &held);
}

/// Cut the segment where the record of leaf `index` begins, and its offsets
/// with it: the records from `index` on are gone whole, as an act whose
/// flush never returned leaves them.
pub(crate) fn cut_before_record(dir: &Path, index: u64) {
    let (first, offset) = locate(dir, index);
    cut(dir, first, index, offset);
}

/// Cut the segment three bytes into the record of leaf `index`: a record
/// torn mid-write, with the offsets of the records before it.
pub(crate) fn cut_inside_record(dir: &Path, index: u64) {
    let (first, offset) = locate(dir, index);
    cut(dir, first, index, offset + 3);
}

fn cut(dir: &Path, first: u64, index: u64, at: u64) {
    std::fs::OpenOptions::new()
        .write(true)
        .open(segment_path(dir, first))
        .unwrap()
        .set_len(at)
        .unwrap();
    std::fs::OpenOptions::new()
        .write(true)
        .open(offsets_path(dir, first))
        .unwrap()
        .set_len((index - first) * 8)
        .unwrap();
}

/// Flip the first byte of the pin carried by the record of leaf `index`,
/// so the pin's record no longer passes its checksum.
pub(crate) fn tamper_pin_byte(dir: &Path, index: u64) {
    let (first, offset) = locate(dir, index);
    let leaf_len = leaf_bytes(dir, index).len();
    let path = segment_path(dir, first);
    let mut bytes = std::fs::read(&path).unwrap();
    let at = usize::try_from(offset).unwrap() + 4 + leaf_len + 1;
    assert_eq!(bytes[at - 1], 1, "leaf {index} carries no pin");
    bytes[at] ^= 0x01;
    std::fs::write(&path, bytes).unwrap();
}
