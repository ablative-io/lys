//! Claude Code's compaction records into Pi compaction entries (HOME-030 R3),
//! each followed by its `lys.loss` entry (R4).
//!
//! Measured on Claude Code 2.1.281: a compaction is a `system` record with
//! subtype `compact_boundary` (parentUuid null, `logicalParentUuid` the
//! record before it, `compactMetadata` with `preTokens` and, when part of
//! the conversation is kept, `preservedSegment` with `headUuid` and
//! `tailUuid`), then a `user` record with `isCompactSummary: true` whose
//! parentUuid is the boundary's uuid and whose message content is the
//! summary. The pair is matched by that parentUuid, never by adjacency.
//! Earlier files hold a `type:"summary"` record instead.
//!
//! The pair becomes one compaction entry when the summary is read, under the
//! summary's uuid, so a later record naming the summary attaches under the
//! compaction. Its parent is the boundary's `tailUuid` when that names an
//! entry on record, otherwise its `logicalParentUuid` when that does,
//! otherwise the last entry on the file's chain. It keeps from the first
//! entry the record named by `headUuid` produced, or nothing (its first kept
//! entry is itself) when there is no preserved segment. A boundary whose
//! summary has not been read when a later record names it as parent, or at
//! the end of the file, becomes a compaction under the boundary's uuid with
//! an empty summary and `summary_missing` in its details; a summary read
//! after that becomes a second compaction naming the first in
//! `details.completes`, and the first is never rewritten.
//!
//! Invariants: every compaction entry becomes the last entry on the file's
//! chain and its loss entry never does; neither source record is written as
//! a harness event or a user message; a `headUuid` that names no record on
//! record refuses the import by that uuid before anything of the compaction
//! is appended, and is the only compaction refusal.

use std::collections::HashMap;
use std::path::Path;

use serde_json::{Map, Value, json};

use crate::error::HomeError;
use crate::harness::claude_code::import::{Importer, field, str_of};
use crate::record::entries::{Entry, EntryBase, EntryBody};
use crate::record::fresh_id;
use crate::record::loss::{Compacted, LossData, append_loss, loss_data};

/// A `compact_boundary` record as the mapping reads it.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Boundary {
    uuid: String,
    logical_parent: Option<String>,
    tail: Option<String>,
    head: Option<String>,
    pre_tokens: u64,
    timestamp: String,
}

/// A compaction written from a boundary before its summary was read.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Unsummarised {
    first_kept: String,
    tokens_before: u64,
    loss: LossData,
}

/// The compaction records of one import that are not yet complete.
#[derive(Debug, Default)]
pub(super) struct Compactions {
    /// Boundaries read whose compaction is not yet written, in file order.
    pending: Vec<Boundary>,
    /// Compactions written without a summary, by their id (the boundary's uuid).
    unsummarised: HashMap<String, Unsummarised>,
}

/// One compaction entry to write.
struct NewCompaction {
    id: String,
    parent: Option<String>,
    summary: String,
    first_kept: String,
    tokens_before: u64,
    timestamp: String,
    details: Option<Value>,
    /// The summary's parts, stored as blocks with rows under the compaction.
    parts: Vec<Value>,
    /// The loss data of the compaction this one completes, when it does.
    completes: Option<LossData>,
}

impl Boundary {
    fn of(record: &Value, source: &Path, n: usize) -> Result<Self, HomeError> {
        let metadata = record.get("compactMetadata");
        let segment = metadata.and_then(|m| m.get("preservedSegment"));
        let uuid_at = |v: Option<&Value>, name: &str| {
            v.and_then(|v| v.get(name))
                .and_then(Value::as_str)
                .map(str::to_owned)
        };
        Ok(Self {
            uuid: field(record, "uuid", source, n)?,
            logical_parent: uuid_at(Some(record), "logicalParentUuid"),
            tail: uuid_at(segment, "tailUuid"),
            head: uuid_at(segment, "headUuid"),
            pre_tokens: metadata
                .and_then(|m| m.get("preTokens"))
                .and_then(Value::as_u64)
                .unwrap_or(0),
            timestamp: str_of(record, "timestamp").to_owned(),
        })
    }
}

impl Importer<'_> {
    /// Map a record that is part of a compaction; `true` when it was one and
    /// is done with. A record that names a pending boundary as its parent
    /// and is not that boundary's summary first writes the boundary's
    /// compaction without a summary, and is then imported as usual.
    pub(super) fn compaction_record(
        &mut self,
        record: &Value,
        source: &Path,
        n: usize,
    ) -> Result<bool, HomeError> {
        let kind = str_of(record, "type");
        if kind == "summary" {
            self.summary_record(record)?;
            return Ok(true);
        }
        if kind == "system" && str_of(record, "subtype") == "compact_boundary" {
            let boundary = Boundary::of(record, source, n)?;
            self.compactions.pending.push(boundary);
            return Ok(true);
        }
        let Some(parent) = record.get("parentUuid").and_then(Value::as_str) else {
            return Ok(false);
        };
        let pending = self
            .compactions
            .pending
            .iter()
            .position(|b| b.uuid == parent);
        let summary = kind == "user"
            && record.get("isCompactSummary").and_then(Value::as_bool) == Some(true);
        if summary {
            if let Some(at) = pending {
                let boundary = self.compactions.pending.remove(at);
                self.pair(&boundary, record, source, n)?;
                return Ok(true);
            }
            if let Some(earlier) = self.compactions.unsummarised.remove(parent) {
                self.completing(parent, earlier, record, source, n)?;
                return Ok(true);
            }
        }
        if let Some(at) = pending {
            let boundary = self.compactions.pending.remove(at);
            self.unsummarised(&boundary)?;
        }
        Ok(false)
    }

    /// At the end of the file: every boundary whose summary never came.
    pub(super) fn finish_compactions(&mut self) -> Result<(), HomeError> {
        for boundary in std::mem::take(&mut self.compactions.pending) {
            self.unsummarised(&boundary)?;
        }
        Ok(())
    }

    /// The boundary and its summary as one compaction under the summary's uuid.
    fn pair(
        &mut self,
        boundary: &Boundary,
        record: &Value,
        source: &Path,
        n: usize,
    ) -> Result<(), HomeError> {
        let id = field(record, "uuid", source, n)?;
        let first_kept = self.first_kept(boundary, &id)?;
        let (summary, parts) = summary_of(record);
        let compaction = NewCompaction {
            parent: self.boundary_parent(boundary)?,
            summary,
            first_kept,
            tokens_before: boundary.pre_tokens,
            timestamp: str_of(record, "timestamp").to_owned(),
            details: Some(json!({"boundaryUuid": boundary.uuid, "summaryUuid": id})),
            parts,
            completes: None,
            id,
        };
        self.write(compaction)?;
        self.report.compaction_sources += 2;
        Ok(())
    }

    /// A boundary whose summary has not been read, as a compaction under the
    /// boundary's uuid with an empty summary.
    fn unsummarised(&mut self, boundary: &Boundary) -> Result<(), HomeError> {
        let first_kept = self.first_kept(boundary, &boundary.uuid)?;
        let compaction = NewCompaction {
            id: boundary.uuid.clone(),
            parent: self.boundary_parent(boundary)?,
            summary: String::new(),
            first_kept: first_kept.clone(),
            tokens_before: boundary.pre_tokens,
            timestamp: boundary.timestamp.clone(),
            details: Some(json!({"boundaryUuid": boundary.uuid, "summary_missing": true})),
            parts: Vec::new(),
            completes: None,
        };
        let loss = self.write(compaction)?;
        self.report.compaction_sources += 1;
        self.compactions.unsummarised.insert(
            boundary.uuid.clone(),
            Unsummarised {
                first_kept,
                tokens_before: boundary.pre_tokens,
                loss,
            },
        );
        Ok(())
    }

    /// A summary read after its boundary's compaction was written without
    /// one: a second compaction under the summary's uuid, naming the first.
    fn completing(
        &mut self,
        earlier_id: &str,
        earlier: Unsummarised,
        record: &Value,
        source: &Path,
        n: usize,
    ) -> Result<(), HomeError> {
        let id = field(record, "uuid", source, n)?;
        let (summary, parts) = summary_of(record);
        let compaction = NewCompaction {
            parent: Some(earlier_id.to_owned()),
            summary,
            first_kept: earlier.first_kept,
            tokens_before: earlier.tokens_before,
            timestamp: str_of(record, "timestamp").to_owned(),
            details: Some(json!({
                "boundaryUuid": earlier_id,
                "summaryUuid": id,
                "completes": earlier_id,
            })),
            parts,
            completes: Some(earlier.loss),
            id,
        };
        self.write(compaction)?;
        self.report.compaction_sources += 1;
        Ok(())
    }

    /// A `type:"summary"` record: a compaction with a fresh id under the last
    /// main-path entry, keeping nothing, with no tokens counted.
    fn summary_record(&mut self, record: &Value) -> Result<(), HomeError> {
        let id = fresh_id();
        let compaction = NewCompaction {
            parent: self.last_main.clone(),
            summary: str_of(record, "summary").to_owned(),
            first_kept: id.clone(),
            tokens_before: 0,
            timestamp: str_of(record, "timestamp").to_owned(),
            details: None,
            parts: Vec::new(),
            completes: None,
            id,
        };
        self.write(compaction)?;
        self.report.compaction_sources += 1;
        Ok(())
    }

    /// Append the compaction entry, store its summary's parts with their
    /// rows, append its loss entry directly after it, and make it the last
    /// entry on the file's chain. Returns the loss data written.
    fn write(&mut self, compaction: NewCompaction) -> Result<LossData, HomeError> {
        let mut rest = Map::new();
        if let Some(details) = compaction.details {
            rest.insert("details".to_owned(), details);
        }
        let entry = Entry {
            base: EntryBase {
                id: compaction.id.clone(),
                parent_id: compaction.parent.clone(),
                timestamp: compaction.timestamp.clone(),
            },
            body: EntryBody::Compaction {
                summary: compaction.summary,
                first_kept_entry_id: compaction.first_kept.clone(),
                tokens_before: compaction.tokens_before,
                rest,
            },
        };
        self.session.append_entry(&entry)?;
        self.report.entries += 1;
        self.report.compactions += 1;
        for (part, value) in (0u64..).zip(&compaction.parts) {
            self.store_row(&compaction.id, part, value)?;
        }
        let loss = if let Some(earlier) = compaction.completes {
            LossData {
                compaction: compaction.id.clone(),
                tokens_before: compaction.tokens_before,
                ..earlier
            }
        } else {
            let compacted = Compacted {
                id: &compaction.id,
                parent: compaction.parent.as_deref(),
                first_kept: &compaction.first_kept,
                tokens_before: compaction.tokens_before,
            };
            loss_data(self.session, &compacted, &self.rows)?
        };
        append_loss(self.session, &compaction.id, &compaction.timestamp, &loss)?;
        self.report.entries += 1;
        self.first_entry
            .insert(compaction.id.clone(), compaction.id.clone());
        self.last_main = Some(compaction.id.clone());
        self.chain_leaf = Some(compaction.id);
        Ok(loss)
    }

    /// The compaction's parent: the boundary's `tailUuid` when it names an
    /// entry on record, otherwise its `logicalParentUuid` when that does,
    /// otherwise the last entry on the file's chain.
    fn boundary_parent(&self, boundary: &Boundary) -> Result<Option<String>, HomeError> {
        for candidate in [&boundary.tail, &boundary.logical_parent]
            .into_iter()
            .flatten()
        {
            if self.session.contains(candidate)? {
                return Ok(Some(candidate.clone()));
            }
        }
        Ok(self.chain_leaf.clone())
    }

    /// The compaction's first kept entry: the first entry the record named by
    /// `headUuid` produced, or the compaction itself when there is no
    /// preserved segment; a `headUuid` naming no record on record is refused.
    fn first_kept(&self, boundary: &Boundary, own: &str) -> Result<String, HomeError> {
        let Some(head) = &boundary.head else {
            return Ok(own.to_owned());
        };
        if let Some(first) = self.first_entry.get(head) {
            return Ok(first.clone());
        }
        if self.session.contains(head)? {
            return Ok(head.clone());
        }
        Err(HomeError::UnknownFirstKept {
            compaction: own.to_owned(),
            uuid: head.clone(),
        })
    }
}

/// A summary record's summary and its parts: a string content is the
/// summary and part 0 as a text part; an array's text parts joined are the
/// summary, and every part is stored at its index.
fn summary_of(record: &Value) -> (String, Vec<Value>) {
    match record.get("message").and_then(|m| m.get("content")) {
        Some(Value::String(s)) => (s.clone(), vec![json!({"type": "text", "text": s})]),
        Some(Value::Array(parts)) => {
            let text: Vec<&str> = parts
                .iter()
                .filter(|p| p.get("type").and_then(Value::as_str) == Some("text"))
                .filter_map(|p| p.get("text").and_then(Value::as_str))
                .collect();
            (text.join("\n"), parts.clone())
        }
        _ => (String::new(), Vec::new()),
    }
}
