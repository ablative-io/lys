//! The grants' folded state as their snapshot holds it: the version, the
//! grant book, and one receipt per leaf in log order.

use ciborium::Value;

use super::projection::{GrantBook, state};
use super::receipt::GrantReceipt;
use super::types::GrantId;
use crate::directory_state::check_receipts;
use crate::id::ID_LEN;
use crate::state_value::{
    Unreadable, array, bytes, coordinate, decode as decode_value, encode as encode_value, identity,
    list, operation, read_coordinate, read_fixed, read_identity, read_operation, read_uint, tuple,
    uint,
};

/// The version of the grant state this crate writes and reads.
const STATE_VERSION: u64 = 1;

fn receipt(held: &GrantReceipt) -> Value {
    array(vec![
        uint(held.version),
        operation(held.operation),
        identity(held.caller),
        bytes(held.grant.as_bytes()),
        uint(held.change_kind),
        bytes(&held.payload_commitment),
        coordinate(held.coordinate),
    ])
}

fn read_receipt(value: Value) -> Result<GrantReceipt, Unreadable> {
    let [version, op, caller, grant, change_kind, commitment, at] =
        tuple::<7>(value, "a grant receipt")?;
    Ok(GrantReceipt {
        version: read_uint(&version, "a receipt's version")?,
        operation: read_operation(op)?,
        caller: read_identity(caller)?,
        grant: GrantId::from_bytes(read_fixed::<ID_LEN>(grant, "a grant id")?),
        change_kind: read_uint(&change_kind, "a receipt's change kind")?,
        payload_commitment: read_fixed::<32>(commitment, "a payload commitment")?,
        coordinate: read_coordinate(at)?,
    })
}

/// The state of `book` with `receipts`, one per leaf.
pub(crate) fn encode(book: &GrantBook, receipts: &[GrantReceipt]) -> Result<Vec<u8>, Unreadable> {
    encode_value(&array(vec![
        uint(STATE_VERSION),
        state::encode(book)?,
        array(receipts.iter().map(receipt).collect()),
    ]))
}

/// The book and receipts a state holds, refused unless it is this version,
/// holds `size` receipts in log order, and the last completes `root`.
pub(crate) fn decode(
    bytes: &[u8],
    size: u64,
    root: [u8; 32],
) -> Result<(GrantBook, Vec<GrantReceipt>), Unreadable> {
    let [version, book, receipts] = tuple::<3>(decode_value(bytes)?, "a grant state")?;
    let version = read_uint(&version, "a state version")?;
    if version != STATE_VERSION {
        return Err(format!(
            "grant state version {version} is not {STATE_VERSION}"
        ));
    }
    let receipts = list(receipts, "the receipts")?
        .into_iter()
        .map(read_receipt)
        .collect::<Result<Vec<_>, _>>()?;
    check_receipts(receipts.iter().map(|held| held.coordinate), size, root)?;
    Ok((state::decode(book)?, receipts))
}
