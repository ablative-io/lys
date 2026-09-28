//! The directory's folded state as its snapshot holds it: the version, the
//! projection, and one receipt per leaf in log order.

use crate::projection::{Projection, state};
use crate::receipt::{Receipt, decode_receipt, encode_receipt};
use crate::state_value::{
    Unreadable, array, decode as decode_value, encode as encode_value, list, read_uint, tuple, uint,
};

/// The version of the directory state this crate writes and reads.
const STATE_VERSION: u64 = 1;

/// The state of `projection` with `receipts`, one per leaf.
pub(crate) fn encode(projection: &Projection, receipts: &[Receipt]) -> Result<Vec<u8>, Unreadable> {
    encode_value(&array(vec![
        uint(STATE_VERSION),
        state::encode(projection),
        array(receipts.iter().map(encode_receipt).collect()),
    ]))
}

/// The projection and receipts a state holds, refused unless it is this
/// version, holds `size` receipts in log order, and the last completes `root`.
pub(crate) fn decode(
    bytes: &[u8],
    size: u64,
    root: [u8; 32],
) -> Result<(Projection, Vec<Receipt>), Unreadable> {
    let [version, projection, receipts] = tuple::<3>(decode_value(bytes)?, "a directory state")?;
    let version = read_uint(&version, "a state version")?;
    if version != STATE_VERSION {
        return Err(format!(
            "directory state version {version} is not {STATE_VERSION}"
        ));
    }
    let receipts = list(receipts, "the receipts")?
        .into_iter()
        .map(decode_receipt)
        .collect::<Result<Vec<_>, _>>()?;
    check_receipts(receipts.iter().map(Receipt::coordinate), size, root)?;
    Ok((state::decode(projection)?, receipts))
}

/// Refuses receipts that are not one per leaf, in log order, the last
/// completing the tree of `size` leaves at `root`.
pub(crate) fn check_receipts(
    coordinates: impl ExactSizeIterator<Item = crate::log::Coordinate>,
    size: u64,
    root: [u8; 32],
) -> Result<(), Unreadable> {
    let held = coordinates.len();
    if u64::try_from(held).ok() != Some(size) {
        return Err(format!("the state holds {held} receipts for {size} leaves"));
    }
    let mut last = None;
    for (expected, coordinate) in (0_u64..).zip(coordinates) {
        if coordinate.index != expected || coordinate.tree_size != expected + 1 {
            return Err(format!(
                "receipt {expected} names leaf {} of a tree of {}",
                coordinate.index, coordinate.tree_size
            ));
        }
        last = Some(coordinate.root);
    }
    match last {
        Some(found) if found != root => Err(format!(
            "the last receipt's root is not the snapshot's root at {size}"
        )),
        _ => Ok(()),
    }
}
