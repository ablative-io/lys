//! The `lys.loss` entry (HOME-030 R4): what one compaction summarised, as
//! ids, counts and hashes.
//!
//! The entry is the line directly after its compaction entry, with the
//! compaction as its parent: a side leaf, so the path through the
//! compaction and the context path are unchanged. It is not the render's
//! `<uuid>.loss.json`, which accounts for what a render dropped.
//!
//! The span is the context the compaction summarised as that context stood:
//! the entries of the compaction's ancestry that Pi's context reading
//! covered at the compaction's parent, less the entries the compaction
//! keeps. On the ancestry, root first, it starts at the root when no
//! compaction entry sits earlier on it; otherwise at the nearest earlier
//! compaction's first kept entry when that entry is on the ancestry, and at
//! that earlier compaction entry when its first kept entry is itself or is
//! not on the ancestry. It ends at the entry whose child on the ancestry is
//! the first kept entry, or at the compaction's parent when the compaction
//! keeps nothing. It is one unbroken run of the ancestry, so its first and
//! last ids and the parent ids between them name every id in it.
//!
//! Invariants: the span is walked through the index by seeking to each
//! entry, never by reading the whole session file; the data names ids,
//! counts and hashes and never a text, a thinking, a tool input, a tool
//! result or a summary; no span entry or block is removed, rewritten or
//! moved.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::error::HomeError;
use crate::record::Session;
use crate::record::block_rows::BlockRowWriter;
use crate::record::blocks::hex_of;
use crate::record::entries::{CUSTOM_LOSS, Entry, EntryBase, EntryBody};
use crate::record::helpers::fresh_id;
use crate::record::index::IndexRow;

/// What a `lys.loss` entry carries in `custom.data`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LossData {
    /// The compaction entry's id.
    pub compaction: String,
    /// The compaction's `firstKeptEntryId`.
    pub first_kept: String,
    /// Whether the compaction keeps nothing: its first kept entry is itself.
    pub kept_none: bool,
    /// The span's first entry id, root first; `None` for an empty span.
    pub span_first: Option<String>,
    /// The span's last entry id; `None` for an empty span.
    pub span_last: Option<String>,
    /// How many entries the span holds.
    pub entries: u64,
    /// The span's message entries: user, assistant and `toolResult`.
    pub messages: u64,
    /// The `toolCall` parts in the span's assistant messages.
    pub tool_calls: u64,
    /// The span's `toolResult` messages.
    pub tool_results: u64,
    /// The block rows whose entry is in the span.
    pub blocks: u64,
    /// The span entries' line lengths in the session file, as the index
    /// holds them, summed.
    pub entry_bytes: u64,
    /// The byte lengths of the blocks those rows name, summed.
    pub block_bytes: u64,
    /// The SHA-256 hex of those rows' hashes in span order then part order,
    /// each followed by one newline.
    pub blocks_sha256: String,
    /// The compaction's `tokensBefore`.
    pub tokens_before: u64,
}

/// The compaction a loss entry is computed for, as its writer knows it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Compacted<'a> {
    /// The compaction entry's id.
    pub id: &'a str,
    /// Its parent's id.
    pub parent: Option<&'a str>,
    /// Its `firstKeptEntryId`.
    pub first_kept: &'a str,
    /// Its `tokensBefore`.
    pub tokens_before: u64,
}

/// The SHA-256 hex of block hashes, each followed by one newline, in the
/// order given.
pub fn blocks_digest(hashes: &[&str]) -> String {
    let mut digest = Sha256::new();
    for hash in hashes {
        digest.update(hash.as_bytes());
        digest.update(b"\n");
    }
    hex_of(&digest.finalize())
}

/// The loss data of a compaction already appended to `session`, its span
/// walked through the index by seeking and its blocks taken from the rows
/// `rows` holds.
pub fn loss_data(
    session: &Session,
    compacted: &Compacted<'_>,
    rows: &BlockRowWriter,
) -> Result<LossData, HomeError> {
    let span = span_of(session, compacted)?;
    let mut data = LossData {
        compaction: compacted.id.to_owned(),
        first_kept: compacted.first_kept.to_owned(),
        kept_none: compacted.first_kept == compacted.id,
        span_first: span.first().map(|(row, _)| row.id.clone()),
        span_last: span.last().map(|(row, _)| row.id.clone()),
        entries: 0,
        messages: 0,
        tool_calls: 0,
        tool_results: 0,
        blocks: 0,
        entry_bytes: 0,
        block_bytes: 0,
        blocks_sha256: String::new(),
        tokens_before: compacted.tokens_before,
    };
    let mut hashes: Vec<&str> = Vec::new();
    for (row, entry) in &span {
        data.entries += 1;
        data.entry_bytes += row.len;
        count_message(entry, &mut data);
        let mut held: Vec<_> = rows.rows_of(&row.id).iter().collect();
        held.sort_by_key(|h| h.part);
        for h in held {
            data.blocks += 1;
            data.block_bytes += h.len;
            hashes.push(h.hash.as_str());
        }
    }
    data.blocks_sha256 = blocks_digest(&hashes);
    Ok(data)
}

/// Append the loss entry of the compaction `compaction` as a side leaf
/// under it: a fresh id, the compaction's timestamp, and the head left
/// where it stands. Returns the entry's id.
pub fn append_loss(
    session: &mut Session,
    compaction: &str,
    timestamp: &str,
    data: &LossData,
) -> Result<String, HomeError> {
    let value = serde_json::to_value(data).map_err(|source| HomeError::Json {
        context: "a loss entry could not be serialised",
        source,
    })?;
    let entry = Entry {
        base: EntryBase {
            id: fresh_id(),
            parent_id: Some(compaction.to_owned()),
            timestamp: timestamp.to_owned(),
        },
        body: EntryBody::Custom {
            custom_type: CUSTOM_LOSS.to_owned(),
            data: Some(value),
        },
    };
    session.append_line(&entry)?;
    Ok(entry.base.id)
}

/// Count one span entry into the message fields: user, assistant and
/// `toolResult` messages, the `toolCall` parts of an assistant message, and
/// the `toolResult` messages again on their own.
fn count_message(entry: &Entry, data: &mut LossData) {
    let EntryBody::Message { message } = &entry.body else {
        return;
    };
    match message.get("role").and_then(Value::as_str) {
        Some("assistant") => {
            data.messages += 1;
            data.tool_calls += message
                .get("content")
                .and_then(Value::as_array)
                .map_or(0, |parts| {
                    parts
                        .iter()
                        .filter(|p| p.get("type").and_then(Value::as_str) == Some("toolCall"))
                        .count() as u64
                });
        }
        Some("toolResult") => {
            data.messages += 1;
            data.tool_results += 1;
        }
        Some("user") => data.messages += 1,
        _ => {}
    }
}

/// The span's rows and entries, in ancestry order, read by seeking.
fn span_of<'s>(
    session: &'s Session,
    compacted: &Compacted<'_>,
) -> Result<Vec<(&'s IndexRow, Entry)>, HomeError> {
    session.fresh()?;
    let Some(parent) = compacted.parent else {
        return Ok(Vec::new());
    };
    let ancestry = session.index.ancestry(parent)?;
    // Walk back from the parent, reading each entry, to the nearest
    // earlier compaction; every entry read on the way is kept for the span.
    let mut read: Vec<Option<Entry>> = vec![None; ancestry.len()];
    let mut start = 0;
    for at in (0..ancestry.len()).rev() {
        let entry = read_one(session, ancestry[at])?;
        let earlier = match &entry.body {
            EntryBody::Compaction {
                first_kept_entry_id,
                ..
            } => Some(first_kept_entry_id.clone()),
            _ => None,
        };
        read[at] = Some(entry);
        if let Some(first_kept) = earlier {
            start = ancestry[..at]
                .iter()
                .position(|row| row.id == first_kept)
                .unwrap_or(at);
            break;
        }
    }
    let end = if compacted.first_kept == compacted.id {
        ancestry.len()
    } else {
        ancestry
            .iter()
            .position(|row| row.id == compacted.first_kept)
            .unwrap_or(ancestry.len())
    };
    let end = end.max(start);
    ancestry[start..end]
        .iter()
        .zip(read.drain(start..end))
        .map(|(&row, held)| match held {
            Some(entry) => Ok((row, entry)),
            None => read_one(session, row).map(|entry| (row, entry)),
        })
        .collect()
}

/// One entry at its row, read by seeking to it.
fn read_one(session: &Session, row: &IndexRow) -> Result<Entry, HomeError> {
    let (mut entries, _) = session.index.read_rows_from(&session.file, &[row])?;
    entries.pop().ok_or(HomeError::StaleIndex {
        path: session.file.clone(),
        reason: "a row read no entry",
    })
}
