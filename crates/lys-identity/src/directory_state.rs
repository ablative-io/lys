//! The directory's folded state as its snapshot holds it: the version, the
//! number of leaves folded, and the projection. Receipts are not held: each
//! is built from the log when it is asked for.

use crate::projection::{Projection, state};
use crate::state_value::{
    Unreadable, array, decode as decode_value, encode as encode_value, read_uint, tuple, uint,
};

/// The version of the directory state this crate writes and reads.
pub(crate) const STATE_VERSION: u64 = 3;

/// The stored owner's version, after checking the canonical outer shape.
pub(crate) fn version(bytes: &[u8]) -> Result<u64, Unreadable> {
    let [version, _, _] = tuple::<3>(decode_value(bytes)?, "a directory state")?;
    read_uint(&version, "a state version")
}

/// Upgrade the old projection, checking that it covers this snapshot's leaves.
pub(crate) fn migrate_v2(bytes: &[u8], size: u64) -> Result<Projection, Unreadable> {
    let [version, folded, projection] = tuple::<3>(decode_value(bytes)?, "a directory state")?;
    let version = read_uint(&version, "a state version")?;
    if version != 2 {
        return Err(format!(
            "directory migration requires version 2, found {version}"
        ));
    }
    check_folded(read_uint(&folded, "a folded count")?, size)?;
    state::migrate_v2(projection)
}

/// The state of `projection`, the fold of the first `folded` leaves.
pub(crate) fn encode(projection: &Projection, folded: u64) -> Result<Vec<u8>, Unreadable> {
    encode_value(&array(vec![
        uint(STATE_VERSION),
        uint(folded),
        state::encode(projection),
    ]))
}

/// The projection a state holds, refused unless it is this version and the
/// fold of exactly `size` leaves.
pub(crate) fn decode(bytes: &[u8], size: u64) -> Result<Projection, Unreadable> {
    let [version, folded, projection] = tuple::<3>(decode_value(bytes)?, "a directory state")?;
    let version = read_uint(&version, "a state version")?;
    if version != STATE_VERSION {
        return Err(format!(
            "directory state version {version} is not {STATE_VERSION}"
        ));
    }
    check_folded(read_uint(&folded, "a folded count")?, size)?;
    state::decode(projection)
}

/// Refuses a state folded from another number of leaves than its snapshot's.
pub(crate) fn check_folded(folded: u64, size: u64) -> Result<(), Unreadable> {
    if folded == size {
        Ok(())
    } else {
        Err(format!(
            "the state is the fold of {folded} leaves, and the snapshot is at {size}"
        ))
    }
}
