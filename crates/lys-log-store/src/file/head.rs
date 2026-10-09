//! The store's head under one writer at a time: the lock an act or a roll
//! holds on the last segment, the check under that lock that no other writer
//! has moved the store past this handle, the act's one write and one flush,
//! and the roll into a new segment.

use std::io::Write;
use std::path::Path;

use super::segment::{self, append_offsets, offsets_path, segment_path};
use super::{FileLeafStore, fsync_dir, write_durably};
use crate::error::{StoreError, StoreResult};

impl FileLeafStore {
    /// Certify this handle's head under the append lock for one reading.
    /// The lock is released when the callback returns, including a refusal.
    /// A later reading must certify its head again; this is no writer lease.
    ///
    /// # Errors
    /// Returns [`StoreError::MigrationPending`] for a deferred legacy store,
    /// [`StoreError::ReopenRequired`] for an uncertain append, and the original
    /// head-lock, corruption or conflicting-writer refusal before the callback.
    /// The callback's error is returned unchanged.
    pub fn with_current_head<T>(
        &self,
        reading: impl FnOnce(&Self) -> StoreResult<T>,
    ) -> StoreResult<T> {
        if self.legacy.is_some() {
            return Err(StoreError::MigrationPending {
                path: self.dir.clone(),
            });
        }
        if let Some(index) = self.durability_uncertain {
            return Err(StoreError::ReopenRequired { index });
        }
        let head = self.hold_head(self.extent)?;
        let result = reading(self);
        drop(head);
        result
    }

    /// Begin a new segment at `first` when the last one is past the roll
    /// size: seal the last offsets file, create the new files, flush the
    /// directory. The act that follows goes into the new segment.
    pub(super) fn roll_if_due(&mut self, first: u64) -> StoreResult<()> {
        if self.last_end <= self.roll_bytes {
            return Ok(());
        }
        let last = *self.segments.last().unwrap_or(&0);
        // The roll is this writer's only while it holds the head: a second
        // writer rolling at the same leaf is refused there, or at the new
        // segment's name, which is made only if nobody has made it.
        let head = self.hold_head(first)?;
        let sealed = std::fs::File::open(offsets_path(&self.dir, last)).map_err(|source| {
            StoreError::Io {
                context: format!("failed to open offsets of segment {last} to seal it"),
                source,
            }
        })?;
        crate::durability::sync_all(&sealed).map_err(|source| StoreError::Io {
            context: format!("failed to flush offsets of segment {last}"),
            source,
        })?;
        drop(sealed);
        create_durably(&segment_path(&self.dir, first), first)?;
        write_durably(&offsets_path(&self.dir, first), &[])?;
        fsync_dir(&segment::segments_dir(&self.dir))?;
        drop(head);
        self.segments.push(first);
        self.last_end = 0;
        self.last_offsets.clear();
        Ok(())
    }

    /// Open the last segment for appending and hold its writer lock, having
    /// confirmed under that lock that the store is where this handle left it:
    /// the segment ends at this handle's last record and no segment begins at
    /// `index`. A second writer's act or roll since this handle last wrote
    /// shows as one or the other, and `index` is then that writer's: the
    /// refusal is [`StoreError::LeafAlreadyWritten`], with nothing written.
    /// The lock is let go when the returned file is dropped.
    fn hold_head(&self, index: u64) -> StoreResult<std::fs::File> {
        let last = *self.segments.last().unwrap_or(&0);
        let path = segment_path(&self.dir, last);
        let failed = |what: &str, source| StoreError::Io {
            context: format!("failed to {what} segment {}", path.display()),
            source,
        };
        let file = std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .map_err(|source| failed("open", source))?;
        file.lock().map_err(|source| failed("lock", source))?;
        let held = file
            .metadata()
            .map_err(|source| failed("measure", source))?
            .len();
        if held < self.last_end {
            return Err(StoreError::Corrupt {
                path: self.dir.clone(),
                reason: format!(
                    "the segment is {held} bytes, shorter than the {} this handle wrote or read",
                    self.last_end
                ),
            });
        }
        let rolled = index != last
            && segment_path(&self.dir, index)
                .try_exists()
                .map_err(|source| failed("look past", source))?;
        if held > self.last_end || rolled {
            return Err(StoreError::LeafAlreadyWritten { index });
        }
        Ok(file)
    }

    /// Write the framed records of the act at `index` behind this handle's
    /// last record, flush them once, and append their `offsets`. The act goes
    /// in only under the head's lock: a second writer's act since this
    /// handle's last is refused there, before any byte.
    pub(super) fn write_act(
        &mut self,
        index: u64,
        buffer: &[u8],
        offsets: &[u64],
    ) -> StoreResult<()> {
        let first = *self.segments.last().unwrap_or(&0);
        let mut file = self.hold_head(index)?;
        // Nothing is acknowledged until the one flush returns: from the first
        // byte written to that flush, any failure holds this handle until a
        // fresh open, which cuts whatever of the act is there.
        self.durability_uncertain = Some(index);
        let uncertain = |source| StoreError::LeafDurabilityUncertain { index, source };
        file.write_all(buffer).map_err(uncertain)?;
        crate::durability::sync_all(&file).map_err(uncertain)?;
        append_offsets(&offsets_path(&self.dir, first), offsets)?;
        drop(file);
        self.durability_uncertain = None;
        Ok(())
    }
}

/// Creates the empty segment whose first leaf is `first` and flushes it,
/// only if no file has that name: a name already there is another writer's
/// segment, and `first` is that writer's leaf.
fn create_durably(path: &Path, first: u64) -> StoreResult<()> {
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|source| {
            if source.kind() == std::io::ErrorKind::AlreadyExists {
                StoreError::LeafAlreadyWritten { index: first }
            } else {
                StoreError::Io {
                    context: format!("failed to create {}", path.display()),
                    source,
                }
            }
        })?;
    crate::durability::sync_all(&file).map_err(|source| StoreError::Io {
        context: format!("failed to flush {} to disk", path.display()),
        source,
    })
}

#[cfg(test)]
mod tests {
    use std::error::Error;
    use std::sync::Arc;

    use super::FileLeafStore;
    use crate::StoreError;

    #[test]
    fn deferred_migration_and_uncertain_durability_refuse_before_reading()
    -> Result<(), Box<dyn Error>> {
        let dir = tempfile::TempDir::new()?;
        let readings = (|| -> Result<_, Box<dyn Error>> {
            let mut store = FileLeafStore::create(dir.path(), "example.test/head-refusals")?;
            let mut calls = 0;
            store.legacy = Some(Arc::new(|| Ok(false)));
            let legacy = store.with_current_head(|_| {
                calls += 1;
                Ok(())
            });
            store.legacy = None;
            store.durability_uncertain = Some(3);
            let uncertain = store.with_current_head(|_| {
                calls += 1;
                Ok(())
            });
            drop(store);
            Ok((legacy, uncertain, calls))
        })();
        let (legacy, uncertain, calls) = match (readings, dir.close()) {
            (Ok(value), Ok(())) => value,
            (Err(error), Ok(())) => return Err(error),
            (Ok(_), Err(error)) => return Err(error.into()),
            (Err(error), Err(cleanup)) => {
                return Err(
                    format!("head fixture failed: {error}; cleanup failed: {cleanup}").into(),
                );
            }
        };
        assert!(matches!(legacy, Err(StoreError::MigrationPending { .. })));
        assert!(matches!(
            uncertain,
            Err(StoreError::ReopenRequired { index: 3 })
        ));
        assert_eq!(calls, 0);
        Ok(())
    }
}
