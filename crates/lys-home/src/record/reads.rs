//! A session's reads by seeking: one entry, the path from the root to the
//! head, the context path, and the custom entries of one type on the path
//! or anywhere in the file.
//!
//! Every read seeks to the rows the index names and deserialises only those
//! entries, and counts each one it deserialises. The context path moves the
//! entries out of the path it read and clones none; the custom entries on
//! the path are chosen by their index rows, so only those are read.

use crate::error::HomeError;
use crate::record::entries::Entry;
use crate::record::index::IndexRow;
use crate::record::session::Session;

impl Session {
    /// One entry by id, read by seeking to it.
    pub fn entry(&self, id: &str) -> Result<Entry, HomeError> {
        self.fresh()?;
        let row = self.index.row(id).ok_or_else(|| HomeError::UnknownEntry {
            session: self.header.id.clone(),
            id: id.to_owned(),
        })?;
        let (mut entries, _) = self.index.read_rows_from(&self.file, &[row])?;
        self.io.read(entries.len());
        entries.pop().ok_or(HomeError::StaleIndex {
            path: self.file.clone(),
            reason: "a row read no entry",
        })
    }

    /// The entries from the root to the head, root first, read by seeking to
    /// each; the second value is how many bytes of the file were read.
    pub fn path(&self) -> Result<(Vec<Entry>, u64), HomeError> {
        self.fresh()?;
        let Some(head) = &self.head else {
            return Ok((Vec::new(), 0));
        };
        let rows = self.index.ancestry(head)?;
        let (entries, read) = self.index.read_rows_from(&self.file, &rows)?;
        self.io.read(entries.len());
        Ok((entries, read))
    }

    /// The context path: what Pi's `buildSessionContext` feeds the model. With
    /// a compaction on the path, the compaction entry first, then the kept
    /// entries from `first_kept_entry_id` up to the compaction, then everything
    /// after it; without one, the whole path.
    pub fn context_path(&self) -> Result<Vec<Entry>, HomeError> {
        let (path, _) = self.path()?;
        let Some(positions) = Self::context_positions(&path) else {
            return Ok(path);
        };
        // Each position is named once, so each entry is taken out once.
        let mut slots: Vec<Option<Entry>> = path.into_iter().map(Some).collect();
        Ok(positions
            .into_iter()
            .filter_map(|n| slots.get_mut(n).and_then(Option::take))
            .collect())
    }

    /// The custom entries of a given custom type on the path, root first.
    pub fn customs(&self, custom_type: &str) -> Result<Vec<Entry>, HomeError> {
        self.fresh()?;
        let Some(head) = &self.head else {
            return Ok(Vec::new());
        };
        let rows: Vec<&IndexRow> = self
            .index
            .ancestry(head)?
            .into_iter()
            .filter(|row| row.custom.as_deref() == Some(custom_type))
            .collect();
        let (entries, _) = self.index.read_rows_from(&self.file, &rows)?;
        self.io.read(entries.len());
        Ok(entries)
    }

    /// Every durable custom entry of a given custom type in the file, on any
    /// branch and whatever the head, in file order, read by seeking to each.
    pub fn customs_everywhere(&self, custom_type: &str) -> Result<Vec<Entry>, HomeError> {
        self.fresh()?;
        let rows = self.index.rows_of_custom(custom_type);
        let (entries, _) = self.index.read_rows_from(&self.file, &rows)?;
        self.io.read(entries.len());
        Ok(entries)
    }
}
