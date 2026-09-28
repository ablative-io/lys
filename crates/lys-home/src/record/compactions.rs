//! `lys-home compactions --home <dir> --session <id>` (HOME-030 R5): a
//! session's compactions, each with its loss entry, its span read by id and
//! its blocks checked as held.
//!
//! The root-to-head ancestry comes from the index's rows, ids and parent
//! ids, without reading the session file; each entry on it is then read on
//! its own by seeking, so an entry that cannot be read is named in
//! `unreadable` and the walk goes on. A compaction's loss entry is found
//! through the index's custom rows. Its span's ids are taken from the index
//! by following parent ids from `span_last` back to `span_first`, and each
//! is read on its own; its blocks are the rows of `<id>.blocks.jsonl` whose
//! entry is in the span, checked against the block store by hash.
//!
//! Invariants: the listing is a second party to the import, not a copy of
//! it: it never computes a span by the import's rule and never guesses one
//! from the path, so a compaction without a loss entry has every span field
//! null and a reason; without the rows file, `blocks_verified` is false and
//! every block field is null, and blocks are never inferred another way. The
//! report names ids, counts and hashes, never transcript, block or summary
//! content. Nothing is written: the session is read without a lock
//! ([`SessionReader`]), and a home without a block store is not given one.
//! No process is started.

use std::collections::HashMap;
use std::path::Path;

use serde_json::{Value, json};

use crate::error::HomeError;
use crate::record::Home;
use crate::record::block_rows::{BlockRow, read_block_rows};
use crate::record::blocks::{BlockStore, Hash};
use crate::record::entries::{CUSTOM_LOSS, Entry, EntryBody};
use crate::record::index::{Index, read_head};
use crate::record::loss::{LossData, blocks_digest};
use crate::record::reader::SessionReader;

/// The reason a compaction without a loss entry carries.
pub const NO_LOSS_ENTRY: &str = "unknown: no lys.loss entry, imported before HOME-002";

/// The listing: the report, and whether every check held (the exit status
/// is 0 exactly when it did).
#[derive(Clone, Debug, PartialEq)]
pub struct Listing {
    /// The one JSON report.
    pub report: Value,
    /// Whether nothing was unreadable, every compaction has its loss entry,
    /// the block rows were read, and every span entry and block was held.
    pub whole: bool,
}

/// A readable compaction entry on the path, with what its details say.
struct OnPath {
    entry: Entry,
    first_kept: String,
    tokens_before: u64,
    summary_missing: bool,
    completes: Option<String>,
}

/// List the compactions of session `session` in the home at `home`.
pub fn list_compactions(home: &Path, session: &str) -> Result<Listing, HomeError> {
    let home = Home::read(home)?;
    let reader = home.read_session(session)?;
    let index = reader.index();
    let head = read_head(reader.file(), index)?;
    let ancestry: Vec<String> = match &head {
        Some(head) => index
            .ancestry(head)?
            .into_iter()
            .map(|row| row.id.clone())
            .collect(),
        None => Vec::new(),
    };
    let mut unreadable: Vec<String> = Vec::new();
    let mut compactions: Vec<OnPath> = Vec::new();
    for id in &ancestry {
        match reader.entry(id) {
            Ok(entry) => {
                if let Some(on_path) = on_path(entry) {
                    compactions.push(on_path);
                }
            }
            Err(_) => unreadable.push(id.clone()),
        }
    }
    let mut losses: HashMap<String, (String, LossData)> = HashMap::new();
    for row in index.rows_of_custom(CUSTOM_LOSS) {
        let data = reader.entry(&row.id).ok().as_ref().and_then(loss_data_of);
        match data {
            Some(data) => {
                losses.insert(data.compaction.clone(), (row.id.clone(), data));
            }
            None => unreadable.push(row.id.clone()),
        }
    }
    let rows = read_block_rows(reader.file()).ok().flatten();
    let store_dir = home.root().join("blocks");
    let store = if store_dir.is_dir() {
        Some(home.blocks()?)
    } else {
        None
    };
    let checks = Checks {
        reader: &reader,
        index,
        rows: rows.as_ref().map(|rows| by_entry(rows)),
        store: store.as_ref(),
    };
    let mut whole = unreadable.is_empty() && checks.rows.is_some();
    let mut listed = Vec::new();
    for compaction in &compactions {
        let id = compaction.entry.id();
        let completed_by = compactions
            .iter()
            .find(|c| c.completes.as_deref() == Some(id))
            .map(|c| c.entry.id().to_owned());
        let summary = if compaction.summary_missing && completed_by.is_none() {
            "missing"
        } else {
            "present"
        };
        let mut row = json!({
            "compaction": id,
            "summary": summary,
            "completes": compaction.completes,
            "completed_by": completed_by,
            "first_kept": compaction.first_kept,
            "kept_none": compaction.first_kept == id,
            "tokens_before": compaction.tokens_before,
        });
        let span = if let Some((loss_id, data)) = losses.get(id) {
            let (span, span_whole) = checks.span(loss_id, data);
            whole &= span_whole;
            span
        } else {
            whole = false;
            no_loss()
        };
        if let (Value::Object(fields), Value::Object(span)) = (&mut row, span) {
            fields.extend(span);
        }
        listed.push(row);
    }
    let report = json!({
        "command": "compactions",
        "session": session,
        "blocks_verified": checks.rows.is_some(),
        "unreadable": unreadable,
        "compactions": listed,
    });
    Ok(Listing { report, whole })
}

/// A compaction entry with what its details say; `None` for any other entry.
fn on_path(entry: Entry) -> Option<OnPath> {
    let EntryBody::Compaction {
        first_kept_entry_id,
        tokens_before,
        rest,
        ..
    } = &entry.body
    else {
        return None;
    };
    let details = rest.get("details");
    let summary_missing = details
        .and_then(|d| d.get("summary_missing"))
        .and_then(Value::as_bool)
        == Some(true);
    let completes = details
        .and_then(|d| d.get("completes"))
        .and_then(Value::as_str)
        .map(str::to_owned);
    Some(OnPath {
        first_kept: first_kept_entry_id.clone(),
        tokens_before: *tokens_before,
        summary_missing,
        completes,
        entry,
    })
}

/// A loss entry's data; `None` when the entry carries none or it is not
/// loss data.
fn loss_data_of(entry: &Entry) -> Option<LossData> {
    let EntryBody::Custom {
        data: Some(data), ..
    } = &entry.body
    else {
        return None;
    };
    serde_json::from_value(data.clone()).ok()
}

/// The span fields of a compaction without a loss entry: all null, with the
/// reason.
fn no_loss() -> Value {
    json!({
        "loss": null,
        "span_first": null,
        "span_last": null,
        "entries_expected": null,
        "entries_read": null,
        "entries_missing": null,
        "blocks_expected": null,
        "blocks_held": null,
        "blocks_missing": null,
        "blocks_sha256": null,
        "reason": NO_LOSS_ENTRY,
    })
}

/// The block rows by entry id, each entry's rows in part order.
fn by_entry(rows: &[BlockRow]) -> HashMap<&str, Vec<&BlockRow>> {
    let mut out: HashMap<&str, Vec<&BlockRow>> = HashMap::new();
    for row in rows {
        out.entry(row.entry.as_str()).or_default().push(row);
    }
    for held in out.values_mut() {
        held.sort_by_key(|row| row.part);
    }
    out
}

/// What the span checks read from.
struct Checks<'a> {
    reader: &'a SessionReader,
    index: &'a Index,
    rows: Option<HashMap<&'a str, Vec<&'a BlockRow>>>,
    store: Option<&'a BlockStore>,
}

impl Checks<'_> {
    /// The span fields of a compaction with a loss entry, and whether every
    /// span entry was read and every block held.
    fn span(&self, loss_id: &str, data: &LossData) -> (Value, bool) {
        let ids = self.span_ids(data);
        let mut entries_read = 0u64;
        let mut entries_missing: Vec<&str> = Vec::new();
        for id in &ids {
            if self.reader.entry(id).is_ok() {
                entries_read += 1;
            } else {
                entries_missing.push(id);
            }
        }
        let mut whole = entries_missing.is_empty() && entries_read == data.entries;
        let (blocks_expected, blocks_held, blocks_missing, blocks_sha256) = match &self.rows {
            Some(rows) => {
                let mut held = 0u64;
                let mut missing: Vec<&str> = Vec::new();
                let mut hashes: Vec<&str> = Vec::new();
                for row in ids.iter().filter_map(|id| rows.get(id.as_str())).flatten() {
                    hashes.push(&row.hash);
                    if self.holds(&row.hash) {
                        held += 1;
                    } else {
                        missing.push(&row.hash);
                    }
                }
                whole &= missing.is_empty();
                (
                    json!(data.blocks),
                    json!(held),
                    json!(missing),
                    json!(blocks_digest(&hashes)),
                )
            }
            None => (Value::Null, Value::Null, Value::Null, Value::Null),
        };
        let span = json!({
            "loss": loss_id,
            "span_first": data.span_first,
            "span_last": data.span_last,
            "entries_expected": data.entries,
            "entries_read": entries_read,
            "entries_missing": entries_missing,
            "blocks_expected": blocks_expected,
            "blocks_held": blocks_held,
            "blocks_missing": blocks_missing,
            "blocks_sha256": blocks_sha256,
            "reason": null,
        });
        (span, whole)
    }

    /// The span's ids in ancestry order: from `span_last` back through the
    /// index's parent ids to `span_first`. The walk stops at an id the
    /// index does not hold, which is kept, so its read fails and it is named
    /// missing.
    fn span_ids(&self, data: &LossData) -> Vec<String> {
        let (Some(first), Some(last)) = (&data.span_first, &data.span_last) else {
            return Vec::new();
        };
        let mut ids = Vec::new();
        let mut at = Some(last.clone());
        while let Some(id) = at {
            let parent = self.index.row(&id).and_then(|row| row.parent.clone());
            let done = id == *first || self.index.row(&id).is_none();
            ids.push(id);
            if done {
                break;
            }
            at = parent;
        }
        ids.reverse();
        ids
    }

    /// Whether the store holds a block of this hash; a malformed hash or a
    /// home without a store holds nothing.
    fn holds(&self, hash: &str) -> bool {
        match (self.store, Hash::parse(hash)) {
            (Some(store), Ok(hash)) => store.contains(&hash),
            _ => false,
        }
    }
}
