//! Consecutive durable leaves with one directory flush.

use super::{AFTER_LINK, AfterLink, FileLeafStore, LeftBehind, StoreError, StoreResult, Write};
use super::{link_leaf, next_process_sequence, write_leaf_temp};
use crate::store::batch_end;

impl FileLeafStore {
    pub(super) fn start_batch(&mut self, end: u64) -> StoreResult<()> {
        self.refuse_if_read_only("begin a batch")?;
        if let Some(index) = self.durability_uncertain {
            return Err(StoreError::ReopenRequired { index });
        }
        if let Some(end) = self.batch_intent {
            return Err(StoreError::BatchIntentPending { end });
        }
        if self.extent != self.pinned.tree_size {
            return Err(StoreError::BatchStartUnpinned {
                extent: self.extent,
                pinned: self.pinned.tree_size,
            });
        }
        if end <= self.pinned.tree_size {
            return Err(StoreError::BatchIntentInvalid {
                pinned: self.pinned.tree_size,
                end,
            });
        }
        self.pin_uncertain = true;
        if let Err(error) = super::write_state(&self.dir, self.pinned, Some(end)) {
            self.durability_uncertain = Some(self.extent);
            return Err(error);
        }
        self.batch_intent = Some(end);
        Ok(())
    }

    pub(super) fn put_batch(&mut self, index: u64, leaves: &[&[u8]]) -> StoreResult<()> {
        self.put_batch_with(index, leaves, &AFTER_LINK)
    }

    fn put_batch_with(
        &mut self,
        index: u64,
        leaves: &[&[u8]],
        after_link: &AfterLink,
    ) -> StoreResult<()> {
        self.refuse_if_read_only("write leaves")?;
        if let Some(index) = self.durability_uncertain {
            return Err(StoreError::ReopenRequired { index });
        }
        let end = batch_end(index, leaves.len())?;
        if leaves.is_empty() {
            return Ok(());
        }
        if index < self.extent {
            return Err(StoreError::LeafAlreadyWritten { index });
        }
        if index > self.extent {
            return Err(StoreError::LeafWouldLeaveGap {
                index,
                next: self.extent,
            });
        }
        let directory = self.dir.join("leaves");
        // A partial batch has no durable directory acknowledgement. Even an
        // ordinary write error must hold this handle until a fresh open.
        self.durability_uncertain = Some(index);
        for (index, bytes) in (index..end).zip(leaves) {
            let temporary = write_leaf_temp(
                &directory,
                index,
                |file| file.write_all(bytes),
                &mut next_process_sequence,
            )?;
            link_leaf(&temporary, &self.leaf_path(index), index)?;
            self.extent = index + 1;
            if let Err(source) = (after_link.remove_temp)(&temporary) {
                self.left_behind.push(LeftBehind {
                    index,
                    path: temporary,
                    source,
                });
            }
        }
        (after_link.flush_dir)(&directory)
            .map_err(|source| StoreError::LeafDurabilityUncertain { index, source })?;
        self.durability_uncertain = None;
        Ok(())
    }
}

#[cfg(test)]
#[path = "batch_tests.rs"]
mod tests;
