//! The grants' folded state as their snapshot holds it: the version, the
//! number of leaves folded, and the grant book. Receipts are not held: each
//! is built from the log when it is asked for.

use super::projection::{GrantBook, state};
use crate::directory_state::check_folded;
use crate::state_value::{
    Unreadable, array, decode as decode_value, encode as encode_value, read_uint, tuple, uint,
};

/// The version of the grant state this crate writes and reads. Version 3
/// added each grant's count of recorded uses; a version 2 state is refused by
/// name and the book rebuilt from the whole log.
const STATE_VERSION: u64 = 3;

/// The state of `book`, the fold of the first `folded` leaves, or why a
/// refusal it keeps has no stable form.
pub(crate) fn encode(book: &GrantBook, folded: u64) -> Result<Vec<u8>, Unreadable> {
    encode_value(&array(vec![
        uint(STATE_VERSION),
        uint(folded),
        state::encode(book)?,
    ]))
}

/// The book a state holds, refused unless it is this version and the fold of
/// exactly `size` leaves.
pub(crate) fn decode(bytes: &[u8], size: u64) -> Result<GrantBook, Unreadable> {
    let [version, folded, book] = tuple::<3>(decode_value(bytes)?, "a grant state")?;
    let version = read_uint(&version, "a state version")?;
    if version != STATE_VERSION {
        return Err(format!(
            "grant state version {version} is not {STATE_VERSION}"
        ));
    }
    check_folded(read_uint(&folded, "a folded count")?, size)?;
    state::decode(book)
}
