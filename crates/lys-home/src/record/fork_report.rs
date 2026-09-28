//! The fork report (HOME-006 R4): what a fork wrote, as ids and counts.
//!
//! `entries` is the number of entries copied into the child. The candidate
//! hashes are the block hashes the copied custom entries name in their data
//! (`record` of a `lys.harness_event`; `request`, `response`, `raw_request`
//! and `raw_response` of a `lys.call`) and the SHA-256 of each content part
//! of the copied message entries, serialised as it stands in the entry.
//! `blocks` counts the distinct candidates that name a block the home's
//! store holds, each checked by path and never read; `unstored` counts the
//! distinct candidates that name none. The importer stores a Claude Code
//! part in its source form and writes a mapped part inline, so an inline
//! thinking, tool-call or tool-result part names no block and counts as
//! unstored. A hash key of the wrong shape, or a list item that is not a
//! string, refuses as the entry's shape rather than being skipped. The
//! report carries no part's text, no note and no entry's data.

use std::collections::BTreeSet;

use serde::Serialize;
use serde_json::Value;

use crate::error::HomeError;
use crate::record::blocks::{BlockStore, Hash};
use crate::record::entries::{CUSTOM_CALL, CUSTOM_HARNESS_EVENT, Entry, EntryBody};

/// What a fork wrote, as ids and counts: never a part's text, a note or
/// any entry's data.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ForkReport {
    /// The child's session id.
    pub child: String,
    /// The parent's session id.
    pub parent: String,
    /// The lantern the fork was taken through.
    pub lantern: String,
    /// The lantern's point.
    pub point: String,
    /// The cut entry: the last assistant message at or before the point.
    pub cut_at: String,
    /// Entries copied into the child.
    pub entries: u64,
    /// Distinct candidate hashes that name a block the store holds.
    pub blocks: u64,
    /// Distinct candidate hashes that name no block in the store.
    pub unstored: u64,
    /// Whether the point was carried rather than copied.
    pub coordinate_carried: bool,
    /// The carried entry's id, when one was carried.
    pub carried: Option<String>,
}

/// The distinct candidate hashes the copied entries name: each content
/// part of a message entry by its own SHA-256, and each block hash a
/// harness event or call record names in its data.
pub fn candidate_hashes(session: &str, entries: &[Entry]) -> Result<BTreeSet<Hash>, HomeError> {
    let mut out = BTreeSet::new();
    for entry in entries {
        let shape = || HomeError::EntryShape {
            session: session.to_owned(),
            id: entry.id().to_owned(),
            custom_type: match &entry.body {
                EntryBody::Custom { custom_type, .. } => custom_type.clone(),
                _ => String::new(),
            },
            source: None,
        };
        match &entry.body {
            EntryBody::Message { message } => {
                let Some(parts) = message.get("content").and_then(Value::as_array) else {
                    continue;
                };
                for part in parts {
                    let bytes = serde_json::to_vec(part).map_err(|source| HomeError::Json {
                        context: "a part could not be serialised",
                        source,
                    })?;
                    out.insert(Hash::of(&bytes));
                }
            }
            EntryBody::Custom {
                custom_type,
                data: Some(data),
            } if custom_type == CUSTOM_HARNESS_EVENT => {
                named(data, &["record"], &[], &mut out, &shape)?;
            }
            EntryBody::Custom {
                custom_type,
                data: Some(data),
            } if custom_type == CUSTOM_CALL => named(
                data,
                &["raw_request", "raw_response"],
                &["request", "response"],
                &mut out,
                &shape,
            )?,
            _ => {}
        }
    }
    Ok(out)
}

/// The hashes `data` names: each key of `strings` a hash string, each key
/// of `lists` a list of hash strings. An absent or null key names nothing;
/// a key of any other shape, or a list item that is not a string, is
/// refused as the entry's shape rather than skipped.
fn named(
    data: &Value,
    strings: &[&str],
    lists: &[&str],
    out: &mut BTreeSet<Hash>,
    shape: &dyn Fn() -> HomeError,
) -> Result<(), HomeError> {
    for key in strings {
        match data.get(key) {
            None | Some(Value::Null) => {}
            Some(Value::String(hash)) => {
                out.insert(Hash::parse(hash)?);
            }
            Some(_) => return Err(shape()),
        }
    }
    for key in lists {
        match data.get(key) {
            None | Some(Value::Null) => {}
            Some(Value::Array(list)) => {
                for item in list {
                    let Value::String(hash) = item else {
                        return Err(shape());
                    };
                    out.insert(Hash::parse(hash)?);
                }
            }
            Some(_) => return Err(shape()),
        }
    }
    Ok(())
}

/// How many of `hashes` name a block the store holds, and how many do not,
/// each checked by path and never read.
#[must_use]
pub fn count_held(store: &BlockStore, hashes: &BTreeSet<Hash>) -> (u64, u64) {
    let held = hashes.iter().filter(|hash| store.contains(hash)).count() as u64;
    (held, hashes.len() as u64 - held)
}
