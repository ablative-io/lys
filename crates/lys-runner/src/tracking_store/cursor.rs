//! Reading the feed by cursor: a page of whole entries after it, whether
//! anything follows it, and the byte a cursor names.

use super::*;
use crate::error::RunnerError;
use crate::operations::OperationOutcome;
use crate::refusals::RefusalRecord;
use crate::tracking::{Figures, Unavailable, UsageRecord, count, note};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

impl Feed {
    /// The entries after `cursor`, the first when none is given, up to
    /// [`PAGE_MAX`]; refused `cursor_expired` for another feed's cursor and
    /// `cursor_ahead` for one past its end.
    pub fn page(&self, cursor: Option<&str>) -> Result<FeedPage, RunnerError> {
        if let Some(writer) = &self.writer {
            writer.barrier()?;
        }
        let from = self.offset(cursor)?;
        let (entries, _, unread) = self.limited_lines(from, self.index.committed, PAGE_MAX * 2)?;
        if let (Some(reason), true) = (unread, entries.is_empty()) {
            return Err(failed(reason));
        }
        let mut page = Vec::new();
        let mut next = from;
        for (entry, after) in entries {
            if page.len() == PAGE_MAX {
                break;
            }
            next = after;
            if !matches!(entry.body, Body::Commit(_)) {
                page.push(entry);
            }
        }
        Ok(FeedPage {
            format: FEED_FORMAT.to_owned(),
            entries: page,
            cursor: format!("{}:{next}", self.index.feed),
        })
    }

    /// Whether anything is committed after `cursor`.
    pub fn after(&self, cursor: Option<&str>) -> Result<bool, RunnerError> {
        Ok(self.offset(cursor)? < self.index.committed)
    }

    pub(super) fn offset(&self, cursor: Option<&str>) -> Result<u64, RunnerError> {
        let Some(cursor) = cursor else {
            return Ok(0);
        };
        let (feed, offset) = cursor.split_once(':').ok_or_else(|| {
            RunnerError::refused(
                "cursor_invalid",
                "a feed cursor is the feed's id and an offset",
            )
        })?;
        if feed != self.index.feed {
            return Err(RunnerError::refused(
                "cursor_expired",
                format!(
                    "the cursor is of feed {feed}, and this runner keeps feed {}: read again from the start of this one",
                    self.index.feed
                ),
            ));
        }
        let offset: u64 = offset.parse().map_err(|error| {
            RunnerError::refused(
                "cursor_invalid",
                format!("a feed cursor's offset is a number: {error}"),
            )
        })?;
        if offset > self.index.committed {
            return Err(RunnerError::refused(
                "cursor_ahead",
                "the cursor is past the end of the feed",
            ));
        }
        Ok(offset)
    }
}
