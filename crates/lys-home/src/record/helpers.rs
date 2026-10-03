//! The helpers and constants the record's files share: the name check, the
//! format version written, the clock, fresh entry ids, a JSON
//! value's size, the durable line append and an entry's custom type.

use std::io::Write;
use std::path::Path;

use serde_json::Value;

use crate::error::HomeError;
use crate::record::blocks;
use crate::record::entries::{Entry, EntryBody};
use crate::record::io_counts::IoCounter;

/// Check that a name is one safe path component: letters, digits, `.`, `_`
/// and `-`, not beginning with `.` and non-empty. No length is set here; a
/// name longer than the file system takes is refused by the file system.
/// Every name that is joined onto a directory passes through here, so `..`,
/// `/` and an absolute path can never leave the directory chosen.
pub fn safe_component(what: &'static str, name: &str) -> Result<(), HomeError> {
    let ok = !name.is_empty()
        && !name.starts_with('.')
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'_' || b == b'-');
    if ok {
        Ok(())
    } else {
        Err(HomeError::BadName {
            what,
            name: name.to_owned(),
        })
    }
}

/// The version of Pi's file format this crate writes: the tree format.
pub const PI_FORMAT_VERSION: u32 = 2;

/// Append a line to the session file and sync it, counting the sync.
pub(super) fn write_durable(file: &Path, line: &str, counts: &IoCounter) -> Result<(), HomeError> {
    let mut f = std::fs::OpenOptions::new()
        .append(true)
        .open(file)
        .map_err(|e| HomeError::io("opening the session file", file, e))?;
    f.write_all(line.as_bytes())
        .map_err(|e| HomeError::io("appending an entry", file, e))?;
    f.sync_all()
        .map_err(|e| HomeError::io("syncing the session file", file, e))?;
    counts.synced();
    Ok(())
}

pub(super) fn custom_type_of(entry: &Entry) -> Option<String> {
    match &entry.body {
        EntryBody::Custom { custom_type, .. } => Some(custom_type.clone()),
        _ => None,
    }
}

/// The current time, RFC 3339 with millisecond precision.
#[must_use]
pub fn now() -> String {
    let t = time::OffsetDateTime::now_utc();
    let t = t
        .replace_nanosecond(u32::from(t.millisecond()) * 1_000_000)
        .unwrap_or(t);
    t.format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| String::from("1970-01-01T00:00:00Z"))
}

/// A fresh entry id: 16 random bytes as hex, which Pi accepts as an id.
#[must_use]
pub fn fresh_id() -> String {
    use rand::RngCore;
    let mut bytes = [0u8; 16];
    rand::rng().fill_bytes(&mut bytes);
    blocks::hex_of(&bytes)
}

/// A JSON value's serialised size in bytes.
#[must_use]
pub fn json_len(value: &Value) -> usize {
    serde_json::to_vec(value).map_or(0, |v| v.len())
}
