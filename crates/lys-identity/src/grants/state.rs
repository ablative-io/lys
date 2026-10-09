//! The grants' folded state as their snapshot holds it: the version, the
//! number of leaves folded, and the grant book. Receipts are not held: each
//! is built from the log when it is asked for.

use super::projection::{GrantBook, state};
use crate::directory_state::check_folded;
use crate::state_value::{
    Unreadable, array, decode as decode_value, encode as encode_value, read_uint, tuple, uint,
};

/// The newest grant state version this crate writes and reads. Version 3
/// added each grant's count of recorded uses; a version 2 state is refused by
/// name and the book rebuilt from the whole log. Version 4 holds a grant with
/// a held mode, its key 14 (ACCESS-001 R1), so a reader older than the mode
/// refuses it by its version and not at the grant. A state is written at the
/// version its content needs, as an event is: a book of outright grants is
/// still version 3, byte for byte.
const STATE_VERSION: u64 = 4;
/// The earliest state version read: a version 3 state is a version 4 state
/// whose grants are all outright.
const STATE_READ_FROM: u64 = 3;

/// The state of `book`, the fold of the first `folded` leaves, or why a
/// refusal it keeps has no stable form.
pub(crate) fn encode(book: &GrantBook, folded: u64) -> Result<Vec<u8>, Unreadable> {
    let held = state::holds_a_held_mode(book);
    encode_value(&array(vec![
        uint(if held { STATE_VERSION } else { STATE_READ_FROM }),
        uint(folded),
        state::encode(book)?,
    ]))
}

/// The book a state holds, refused unless it is version 3 or 4 and the fold
/// of exactly `size` leaves.
pub(crate) fn decode(bytes: &[u8], size: u64) -> Result<GrantBook, Unreadable> {
    let [version, folded, book] = tuple::<3>(decode_value(bytes)?, "a grant state")?;
    let version = read_uint(&version, "a state version")?;
    if !(STATE_READ_FROM..=STATE_VERSION).contains(&version) {
        return Err(format!(
            "grant state version {version} is not {STATE_READ_FROM} to {STATE_VERSION}"
        ));
    }
    check_folded(read_uint(&folded, "a folded count")?, size)?;
    state::decode(book)
}
