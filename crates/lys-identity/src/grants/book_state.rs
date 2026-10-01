//! The grant book as a snapshot holds it.
//!
//! Every grant in its canonical encoding with the index that issued it, its
//! revocation, its last observed use and its count of recorded uses; every
//! operation with the index it made; and every refused event's index,
//! operation and refusal in its stable encoded form. The lineage and resource
//! indexes are not written: each is the grants' own sources and resources,
//! rebuilt on reading exactly as [`GrantBook::apply`] builds them.

use ciborium::Value;

use super::{GrantBook, GrantRecord, LastUse, Revocation};
use crate::grants::codec::decode_grant;
use crate::grants::codec::encode_grant;
use crate::grants::events::{route_code, route_from};
use crate::grants::refusal_codec::{decode_refusal, encode_refusal};
use crate::grants::types::Source;
use crate::state_value::{
    Unreadable, array, bytes, indexed, list, nullable, operation, read_indexed, read_nullable,
    read_operation, read_text, read_uint, text, tuple, uint,
};

fn record(held: &GrantRecord) -> Value {
    let revoked = held.revoked.as_ref().map(|revocation| {
        array(vec![
            operation(revocation.operation),
            uint(revocation.index),
            uint(revocation.at),
            text(&revocation.reason),
        ])
    });
    let last_use = match held.last_use {
        LastUse::NotSeen => None,
        LastUse::Seen { at, route, index } => {
            Some(array(vec![uint(at), uint(route_code(route)), uint(index)]))
        }
    };
    array(vec![
        bytes(&encode_grant(&held.grant)),
        uint(held.index),
        nullable(revoked),
        nullable(last_use),
        uint(held.uses),
    ])
}

fn read_record(value: Value) -> Result<GrantRecord, Unreadable> {
    let [grant, index, revoked, last_use, uses] = tuple::<5>(value, "a grant record")?;
    let Value::Bytes(grant) = grant else {
        return Err("a grant is not a byte string".to_owned());
    };
    let revoked = read_nullable(revoked)
        .map(|revocation| {
            let [op, index, at, reason] = tuple::<4>(revocation, "a revocation")?;
            Ok::<_, Unreadable>(Revocation {
                operation: read_operation(op)?,
                index: read_uint(&index, "a revocation's index")?,
                at: read_uint(&at, "a revocation's time")?,
                reason: read_text(reason, "a revocation's reason")?,
            })
        })
        .transpose()?;
    let last_use = match read_nullable(last_use) {
        None => LastUse::NotSeen,
        Some(seen) => {
            let [at, route, index] = tuple::<3>(seen, "a last use")?;
            let route = read_uint(&route, "a route")?;
            LastUse::Seen {
                at: read_uint(&at, "a use's time")?,
                route: route_from(route).ok_or_else(|| format!("route {route} is not a route"))?,
                index: read_uint(&index, "a use's index")?,
            }
        }
    };
    Ok(GrantRecord {
        grant: decode_grant(&grant).map_err(|error| error.to_string())?,
        index: read_uint(&index, "a grant's index")?,
        revoked,
        last_use,
        uses: read_uint(&uses, "a grant's use count")?,
    })
}

/// The book as a state value, or why a refusal it keeps has no stable form.
pub(crate) fn encode(book: &GrantBook) -> Result<Value, Unreadable> {
    let refused = book
        .refused
        .iter()
        .map(|(index, (op, refusal))| {
            Ok(array(vec![
                uint(*index),
                operation(*op),
                encode_refusal(refusal)?,
            ]))
        })
        .collect::<Result<Vec<_>, Unreadable>>()?;
    Ok(array(vec![
        array(book.records.values().map(record).collect()),
        indexed(
            book.operations
                .iter()
                .map(|(op, index)| (*op, *index))
                .collect(),
            |op| operation(*op),
        ),
        array(refused),
    ]))
}

/// The book a state value holds, with its lineage and resource indexes rebuilt.
pub(crate) fn decode(value: Value) -> Result<GrantBook, Unreadable> {
    let [records, operations, refused] = tuple::<3>(value, "a grant book")?;
    let mut book = GrantBook::new();
    for held in list(records, "the grant records")? {
        let held = read_record(held)?;
        let id = held.grant.id();
        book.index_kind(&held.grant);
        if let Source::Grant(source) = held.grant.source() {
            book.children.entry(source).or_default().insert(id);
        }
        book.by_resource
            .entry(held.grant.resource().clone())
            .or_default()
            .insert(id);
        if book.records.insert(id, held).is_some() {
            return Err(format!("grant {id} is recorded twice"));
        }
    }
    book.operations = read_indexed(operations, "an operation", read_operation)?
        .into_iter()
        .collect();
    for kept in list(refused, "the refused events")? {
        let [index, op, refusal] = tuple::<3>(kept, "a refused event")?;
        let index = read_uint(&index, "a refused event's index")?;
        book.refused
            .insert(index, (read_operation(op)?, decode_refusal(refusal)?));
    }
    Ok(book)
}
