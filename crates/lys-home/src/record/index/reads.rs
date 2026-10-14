//! Index lookups, ancestry and reads at recorded offsets.

use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use super::{Index, IndexRow};
use crate::error::HomeError;
use crate::record::entries::Entry;

impl Index {
    /// Where the session file ends, as the index knows it.
    #[must_use]
    pub fn end(&self) -> u64 {
        self.end
    }

    /// How many entries are indexed.
    #[must_use]
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    /// Whether no entries are indexed.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// The row of an entry.
    #[must_use]
    pub fn row(&self, id: &str) -> Option<&IndexRow> {
        self.by_id.get(id).map(|&i| &self.rows[i])
    }

    /// Every row, in file order.
    #[must_use]
    pub fn rows(&self) -> &[IndexRow] {
        &self.rows
    }

    /// The last row appended.
    #[must_use]
    pub fn last(&self) -> Option<&IndexRow> {
        self.rows.last()
    }

    /// The rows of custom entries of this custom type, in file order.
    #[must_use]
    pub fn rows_of_custom(&self, custom_type: &str) -> Vec<&IndexRow> {
        self.rows
            .iter()
            .filter(|r| r.custom.as_deref() == Some(custom_type))
            .collect()
    }

    /// The ids from `id` back to the root, root first.
    ///
    /// # Errors
    ///
    /// Refuses as `UnknownParent` when the starting id or any parent reached
    /// from it has no indexed row.
    pub fn ancestry(&self, id: &str) -> Result<Vec<&IndexRow>, HomeError> {
        let mut path: Vec<&IndexRow> = Vec::new();
        let mut cur: &str = id;
        loop {
            let row = self.row(cur).ok_or_else(|| HomeError::UnknownParent {
                id: path.last().map_or_else(|| cur.to_owned(), |r| r.id.clone()),
                parent: cur.to_owned(),
            })?;
            path.push(row);
            match &row.parent {
                Some(parent) => cur = parent,
                None => break,
            }
        }
        path.reverse();
        Ok(path)
    }

    /// Read the entries at these rows by seeking to each, in the order given.
    /// Returns the entries and how many bytes were read from the file.
    ///
    /// # Errors
    ///
    /// Refuses a file that cannot be opened, sought or read, a row too long
    /// to allocate, a line that is not an entry, or an entry whose id differs
    /// from its indexed row.
    pub fn read_rows_from(
        &self,
        session_file: &Path,
        rows: &[&IndexRow],
    ) -> Result<(Vec<Entry>, u64), HomeError> {
        let mut file = fs::File::open(session_file)
            .map_err(|e| HomeError::io("opening the session file", session_file, e))?;
        let mut out = Vec::with_capacity(rows.len());
        let mut read = 0u64;
        for row in rows {
            file.seek(SeekFrom::Start(row.offset))
                .map_err(|e| HomeError::io("seeking the session file", session_file, e))?;
            let mut buf = vec![
                0u8;
                usize::try_from(row.len).map_err(|source| HomeError::RowTooLong {
                    path: session_file.to_path_buf(),
                    len: row.len,
                    source,
                })?
            ];
            file.read_exact(&mut buf)
                .map_err(|e| HomeError::io("reading an entry", session_file, e))?;
            read += row.len;
            let entry: Entry = serde_json::from_slice(&buf).map_err(|e| HomeError::Malformed {
                path: session_file.to_path_buf(),
                line: 0,
                what: "session entry",
                reason: format!("at offset {}: {e}", row.offset),
            })?;
            if entry.base.id != row.id {
                return Err(HomeError::StaleIndex {
                    path: session_file.to_path_buf(),
                    reason: "an entry's id differs from its row",
                });
            }
            out.push(entry);
        }
        Ok((out, read))
    }
}
