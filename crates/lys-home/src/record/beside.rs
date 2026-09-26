//! A side leaf beside the context path, and the session head by hash
//! (HOME-002 R4).
//!
//! [`Session::append_beside`] appends an entry as a child of the head without
//! moving the head: the entry is durable in the same order as any append (the
//! line, then its index row) through the one write path `append_entry` uses,
//! and the head file is not touched. The render walker never sees such a
//! leaf; `customs_everywhere` does. [`Session::append_under`] does the same
//! under a named entry instead of the head, so a record that follows a side
//! leaf (the given entry after a render event, HOME-003 R4) hangs under it
//! and the head still does not move.
//!
//! [`Session::head_hash`] names the session's position by content: the
//! SHA-256 of the head entry's line bytes, trailing newline included, exactly
//! the bytes its index row's offset and length name; for a session with no
//! head, the header line's bytes. The bytes are read by seeking to the row,
//! never by reading the file.

use std::io::{Read, Seek, SeekFrom};

use crate::error::HomeError;
use crate::record::blocks::Hash;
use crate::record::entries::{Entry, EntryBase, EntryBody};
use crate::record::index::read_header;
use crate::record::{Session, fresh_id, now};

impl Session {
    /// Append an entry as a child of the head, with a fresh id and the
    /// current time, and leave the head where it stands. Returns the id.
    pub fn append_beside(&mut self, body: EntryBody) -> Result<String, HomeError> {
        self.reconcile()?;
        let id = fresh_id();
        let entry = Entry {
            base: EntryBase {
                id: id.clone(),
                parent_id: self.head.clone(),
                timestamp: now(),
            },
            body,
        };
        self.append_line(&entry)?;
        Ok(id)
    }

    /// Append an entry as a child of a named entry on record, with a fresh
    /// id and the current time, and leave the head where it stands. Returns
    /// the id; a parent not on record is refused by name.
    pub fn append_under(&mut self, parent: &str, body: EntryBody) -> Result<String, HomeError> {
        self.reconcile()?;
        let id = fresh_id();
        let entry = Entry {
            base: EntryBase {
                id: id.clone(),
                parent_id: Some(parent.to_owned()),
                timestamp: now(),
            },
            body,
        };
        self.append_line(&entry)?;
        Ok(id)
    }

    /// The SHA-256 of the head entry's line, newline included, read by
    /// seeking to its row; of the header line when the session has no head.
    pub fn head_hash(&self) -> Result<Hash, HomeError> {
        self.fresh()?;
        let (offset, len) = if let Some(id) = &self.head {
            let row = self.index.row(id).ok_or_else(|| HomeError::UnknownEntry {
                session: self.header.id.clone(),
                id: id.clone(),
            })?;
            (row.offset, row.len)
        } else {
            let (_, len) = read_header(&self.file)?;
            (0, len)
        };
        let mut file = std::fs::File::open(&self.file)
            .map_err(|e| HomeError::io("opening the session file", &self.file, e))?;
        file.seek(SeekFrom::Start(offset))
            .map_err(|e| HomeError::io("seeking the session file", &self.file, e))?;
        let mut bytes = vec![
            0u8;
            usize::try_from(len).map_err(|source| HomeError::RowTooLong {
                path: self.file.clone(),
                len,
                source,
            })?
        ];
        file.read_exact(&mut bytes)
            .map_err(|e| HomeError::io("reading the head entry", &self.file, e))?;
        Ok(Hash::of(&bytes))
    }
}
