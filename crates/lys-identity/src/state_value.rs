//! The values a folded-state snapshot is written in, and their readers.
//!
//! A state is one CBOR value built only of unsigned integers, byte strings,
//! text, arrays and null: no maps and no floats, so every value has one
//! encoding. A state is read back only if encoding what was read gives the
//! very bytes that were read, so trailing or non-canonical bytes are refused.
//! Every reader answers why it refused, and the owner refuses the snapshot by
//! that reason.

use ciborium::Value;

use crate::binding::LoginBinding;
use crate::event::wire;
use crate::id::{AgentId, ID_LEN, IdentityId, PersonId};
use crate::log::Coordinate;
use crate::operation::OperationId;

/// Why a state could not be read.
pub(crate) type Unreadable = String;

/// The bytes of `value`.
pub(crate) fn encode(value: &Value) -> Result<Vec<u8>, Unreadable> {
    let mut out = Vec::new();
    ciborium::into_writer(value, &mut out).map_err(|error| error.to_string())?;
    Ok(out)
}

/// The value `bytes` encode, refused unless re-encoding it gives `bytes`.
pub(crate) fn decode(bytes: &[u8]) -> Result<Value, Unreadable> {
    let value: Value = ciborium::from_reader(bytes).map_err(|error| error.to_string())?;
    if encode(&value)? != bytes {
        return Err("the state is not the one encoding of what it holds".to_owned());
    }
    Ok(value)
}

/// An unsigned integer.
pub(crate) fn uint(value: u64) -> Value {
    Value::Integer(value.into())
}

/// A byte string.
pub(crate) fn bytes(value: &[u8]) -> Value {
    Value::Bytes(value.to_vec())
}

/// A text string.
pub(crate) fn text(value: &str) -> Value {
    Value::Text(value.to_owned())
}

/// An array.
pub(crate) fn array(items: Vec<Value>) -> Value {
    Value::Array(items)
}

/// `value`, or null.
pub(crate) fn nullable(value: Option<Value>) -> Value {
    value.unwrap_or(Value::Null)
}

/// The items of an array of exactly `N`.
pub(crate) fn tuple<const N: usize>(value: Value, what: &str) -> Result<[Value; N], Unreadable> {
    let items = list(value, what)?;
    let found = items.len();
    <[Value; N]>::try_from(items).map_err(|_items| format!("{what} holds {found} items, not {N}"))
}

/// The items of an array.
pub(crate) fn list(value: Value, what: &str) -> Result<Vec<Value>, Unreadable> {
    match value {
        Value::Array(items) => Ok(items),
        _ => Err(format!("{what} is not an array")),
    }
}

/// An unsigned integer.
pub(crate) fn read_uint(value: &Value, what: &str) -> Result<u64, Unreadable> {
    match value {
        Value::Integer(integer) => {
            u64::try_from(*integer).map_err(|_sign| format!("{what} is not an unsigned integer"))
        }
        _ => Err(format!("{what} is not an integer")),
    }
}

/// A text string.
pub(crate) fn read_text(value: Value, what: &str) -> Result<String, Unreadable> {
    match value {
        Value::Text(text) => Ok(text),
        _ => Err(format!("{what} is not text")),
    }
}

/// A byte string of exactly `N` bytes.
pub(crate) fn read_fixed<const N: usize>(value: Value, what: &str) -> Result<[u8; N], Unreadable> {
    match value {
        Value::Bytes(bytes) => {
            <[u8; N]>::try_from(bytes).map_err(|_bytes| format!("{what} is not {N} bytes"))
        }
        _ => Err(format!("{what} is not a byte string")),
    }
}

/// `Some` of a value that is not null.
pub(crate) fn read_nullable(value: Value) -> Option<Value> {
    match value {
        Value::Null => None,
        other => Some(other),
    }
}

/// An operation id.
pub(crate) fn operation(value: OperationId) -> Value {
    bytes(value.as_bytes())
}

/// An operation id.
pub(crate) fn read_operation(value: Value) -> Result<OperationId, Unreadable> {
    Ok(OperationId::from_bytes(read_fixed::<ID_LEN>(
        value,
        "an operation id",
    )?))
}

/// An identity: its kind code and its id.
pub(crate) fn identity(value: IdentityId) -> Value {
    let (kind, id) = match value {
        IdentityId::Person(id) => (wire::PERSON, *id.as_bytes()),
        IdentityId::Agent(id) => (wire::AGENT, *id.as_bytes()),
    };
    array(vec![uint(kind), bytes(&id)])
}

/// An identity.
pub(crate) fn read_identity(value: Value) -> Result<IdentityId, Unreadable> {
    let [kind, id] = tuple::<2>(value, "an identity")?;
    let id = read_fixed::<ID_LEN>(id, "an identity id")?;
    match read_uint(&kind, "an identity kind")? {
        wire::PERSON => Ok(IdentityId::Person(PersonId::from_bytes(id))),
        wire::AGENT => Ok(IdentityId::Agent(AgentId::from_bytes(id))),
        code => Err(format!("identity kind {code} is not a person or an agent")),
    }
}

/// A login binding: its issuer and subject.
pub(crate) fn binding(value: &LoginBinding) -> Value {
    array(vec![text(value.issuer()), text(value.subject())])
}

/// A login binding, checked as one is when it is first bound.
pub(crate) fn read_binding(value: Value) -> Result<LoginBinding, Unreadable> {
    let [issuer, subject] = tuple::<2>(value, "a login binding")?;
    LoginBinding::new(
        &read_text(issuer, "a binding's issuer")?,
        &read_text(subject, "a binding's subject")?,
    )
    .map_err(|error| error.to_string())
}

/// A log coordinate.
pub(crate) fn coordinate(value: Coordinate) -> Value {
    array(vec![
        uint(value.index),
        uint(value.tree_size),
        bytes(&value.root),
        bytes(&value.leaf_hash),
    ])
}

/// A log coordinate.
pub(crate) fn read_coordinate(value: Value) -> Result<Coordinate, Unreadable> {
    let [index, tree_size, root, leaf_hash] = tuple::<4>(value, "a coordinate")?;
    Ok(Coordinate {
        index: read_uint(&index, "a coordinate's index")?,
        tree_size: read_uint(&tree_size, "a coordinate's tree size")?,
        root: read_fixed::<32>(root, "a coordinate's root")?,
        leaf_hash: read_fixed::<32>(leaf_hash, "a coordinate's leaf hash")?,
    })
}

/// A list of `(key, index)` pairs, sorted so the list has one order.
pub(crate) fn indexed<K: Ord>(mut pairs: Vec<(K, u64)>, key: impl Fn(&K) -> Value) -> Value {
    pairs.sort_by(|left, right| left.0.cmp(&right.0));
    array(
        pairs
            .iter()
            .map(|(name, index)| array(vec![key(name), uint(*index)]))
            .collect(),
    )
}

/// A list of `(key, index)` pairs.
pub(crate) fn read_indexed<K>(
    value: Value,
    what: &str,
    key: impl Fn(Value) -> Result<K, Unreadable>,
) -> Result<Vec<(K, u64)>, Unreadable> {
    list(value, what)?
        .into_iter()
        .map(|pair| {
            let [name, index] = tuple::<2>(pair, what)?;
            Ok((key(name)?, read_uint(&index, what)?))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{array, bytes, decode, encode, read_uint, text, tuple, uint};

    #[test]
    fn a_state_reads_back_as_written() -> Result<(), String> {
        let value = array(vec![uint(7), bytes(b"leaf"), text("name")]);
        let [seven, _, _] = tuple::<3>(decode(&encode(&value)?)?, "the test value")?;
        assert_eq!(read_uint(&seven, "seven")?, 7);
        Ok(())
    }

    #[test]
    fn trailing_bytes_are_refused() -> Result<(), String> {
        let mut encoded = encode(&uint(7))?;
        encoded.push(0);
        assert!(decode(&encoded).is_err());
        Ok(())
    }
}
