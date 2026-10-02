//! A chain of entries published with one sync per durable file and one
//! directory sync, preserving the same line, row and head ordering.

use std::collections::HashSet;

use serde::Serialize;

use crate::error::HomeError;
use crate::record::entries::{EntryBase, EntryBody};
use crate::record::helpers::{fresh_id, now};
use crate::record::index::IndexRow;
use crate::record::session::{Session, to_line};

#[derive(Clone, Copy)]
pub(super) enum BatchStep {
    Lines,
    Rows,
    Head,
}

#[derive(Serialize)]
struct BorrowedEntry<'a> {
    #[serde(flatten)]
    base: EntryBase,
    #[serde(flatten)]
    body: &'a EntryBody,
}

struct Batch {
    lines: String,
    rows: Vec<IndexRow>,
    ids: Vec<String>,
}

impl Session {
    /// Append a chain below the current head and publish its last entry as
    /// the head. All lines are durable before any rows, and all rows before
    /// the head. An empty batch writes nothing. A published session uses
    /// four syncs regardless of the entry count; staged entries remain
    /// unsynced until publication. A failed write is reconciled before any
    /// later act, and its error is returned even when reconciliation succeeds.
    pub fn append_all(&mut self, bodies: &[EntryBody]) -> Result<Vec<String>, HomeError> {
        self.append_all_observed(bodies, |_| {})
    }

    pub(super) fn append_all_observed(
        &mut self,
        bodies: &[EntryBody],
        mut observed: impl FnMut(BatchStep),
    ) -> Result<Vec<String>, HomeError> {
        self.reconcile()?;
        if bodies.is_empty() {
            return Ok(Vec::new());
        }
        let batch = self.prepare_batch(bodies)?;
        self.write_line(&batch.lines)?;
        observed(BatchStep::Lines);
        if self.staged.is_some() {
            for row in batch.rows {
                self.index.push_row(row);
            }
        } else if let Err(error) = self.index.append_rows(batch.rows, &self.io) {
            self.stale = true;
            self.reconcile()?;
            return Err(error);
        }
        observed(BatchStep::Rows);
        let head = batch.ids.last().map(String::as_str);
        if let Err(error) = self.persist_head(head) {
            self.stale = true;
            self.reconcile()?;
            return Err(error);
        }
        self.head = head.map(str::to_owned);
        for (body, id) in bodies.iter().zip(&batch.ids) {
            self.note_call_body(body, id);
        }
        observed(BatchStep::Head);
        Ok(batch.ids)
    }

    fn prepare_batch(&self, bodies: &[EntryBody]) -> Result<Batch, HomeError> {
        let mut batch = Batch {
            lines: String::new(),
            rows: Vec::with_capacity(bodies.len()),
            ids: Vec::with_capacity(bodies.len()),
        };
        let mut parent = self.head.clone();
        let mut offset = self.index.end();
        let mut seen = HashSet::with_capacity(bodies.len());
        for body in bodies {
            let id = fresh_id();
            if self.index.row(&id).is_some() || !seen.insert(id.clone()) {
                return Err(HomeError::DuplicateEntry {
                    session: self.header.id.clone(),
                    id,
                });
            }
            let entry = BorrowedEntry {
                base: EntryBase {
                    id: id.clone(),
                    parent_id: parent.clone(),
                    timestamp: now(),
                },
                body,
            };
            let line = to_line(&entry, &self.file)?;
            let len = line.len() as u64;
            let end = offset
                .checked_add(len)
                .ok_or_else(|| HomeError::Malformed {
                    path: self.file.clone(),
                    line: 0,
                    what: "session batch",
                    reason: "its byte offsets exceed the record range".to_owned(),
                })?;
            let custom = match body {
                EntryBody::Custom { custom_type, .. } => Some(custom_type.clone()),
                _ => None,
            };
            batch.rows.push(IndexRow {
                id: id.clone(),
                parent,
                offset,
                len,
                custom,
            });
            parent = Some(id.clone());
            offset = end;
            batch.ids.push(id);
            batch.lines.push_str(&line);
        }
        Ok(batch)
    }
}
