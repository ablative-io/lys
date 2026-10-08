//! Preserve the installed reader's format until its replacement commits.

use std::path::Path;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;

use super::{
    FileLeafStore, MigrationReady, fsync_dir, json_bytes, legacy_leaves, v1, write_durably,
};
use crate::{LeafStore, PinnedRoot, StoreError, StoreResult};

impl FileLeafStore {
    /// Opens a writable store without migrating v1 until `ready` answers true.
    /// The check is dropped after migration; ordinary v2 calls never poll it.
    ///
    /// # Errors
    /// The errors of [`Self::open`], the commit-point check, and named legacy
    /// leaf or pin corruption.
    pub fn open_deferred(dir: &Path, ready: MigrationReady) -> StoreResult<Self> {
        if !v1::present(dir)? || ready()? {
            return Self::open(dir);
        }
        let (origin, pinned) = v1::identity(dir)?;
        fsync_dir(&dir.join("leaves"))?;
        let extent = legacy_leaves::probed_extent(dir, pinned.tree_size)?;
        Ok(Self {
            legacy: Some(ready),
            dir: dir.to_path_buf(),
            origin,
            extent,
            pinned,
            segments: Vec::new(),
            last_end: 0,
            last_offsets: Vec::new(),
            durability_uncertain: None,
            read_only: false,
            durable_snapshot: None,
            unfinished_tail: None,
            migrated: None,
            roll_bytes: super::segment::ROLL_BYTES,
        })
    }

    /// The deferred check, retained when an owner rebuilds its snapshot.
    pub fn deferred_migration(&self) -> Option<MigrationReady> {
        self.legacy.clone()
    }

    pub(super) fn migrate_if_ready(&mut self) -> StoreResult<()> {
        if let Some(index) = self.durability_uncertain {
            return Err(StoreError::ReopenRequired { index });
        }
        if let Some(ready) = &self.legacy
            && ready()?
        {
            // A failed switch may have renamed the directory already. This
            // handle must never write through its former layout afterward.
            self.durability_uncertain = Some(self.extent);
            *self = Self::open(&self.dir)?;
        }
        Ok(())
    }
}

pub(super) fn append(
    store: &mut FileLeafStore,
    index: u64,
    leaves: &[&[u8]],
    pinned: PinnedRoot,
) -> StoreResult<()> {
    let dir = store.dir.join("leaves");
    for (index, bytes) in (index..).zip(leaves) {
        let tmp = legacy_leaves::write_leaf_temp(
            &dir,
            index,
            |file| std::io::Write::write_all(file, bytes),
            &mut legacy_leaves::next_process_sequence,
        )?;
        legacy_leaves::link_leaf(&tmp, &dir.join(format!("{index:020}")), index)?;
        store.extent = index + 1;
        let removed = std::fs::remove_file(&tmp);
        let flushed = fsync_dir(&dir);
        if let Err(error) = flushed {
            store.durability_uncertain = Some(index);
            return Err(StoreError::Io {
                context: match removed {
                    Ok(()) => error.to_string(),
                    Err(source) => {
                        format!("{error}; removing {} also failed: {source}", tmp.display())
                    }
                },
                source: std::io::Error::other("legacy leaf durability was not confirmed"),
            });
        }
        if let Err(source) = removed {
            store.durability_uncertain = Some(index);
            return Err(StoreError::Io {
                context: format!("failed to remove linked leaf temporary {}", tmp.display()),
                source,
            });
        }
    }
    store.durability_uncertain = Some(index);
    pin(store, pinned)?;
    store.durability_uncertain = None;
    Ok(())
}

pub(super) fn pin(store: &mut FileLeafStore, pinned: PinnedRoot) -> StoreResult<()> {
    let state = serde_json::json!({
        "tree_size": pinned.tree_size,
        "root_hash": STANDARD.encode(pinned.root),
    });
    let tmp = store.dir.join("state.json.tmp");
    write_durably(&tmp, &json_bytes(&state, "legacy pin")?)?;
    std::fs::rename(&tmp, store.dir.join("state.json")).map_err(|source| StoreError::Io {
        context: "failed to replace legacy pin".to_owned(),
        source,
    })?;
    fsync_dir(&store.dir)?;
    store.pinned = pinned;
    Ok(())
}

pub(super) fn audit(store: &FileLeafStore) -> StoreResult<()> {
    let extent = legacy_leaves::contiguous_extent(&store.dir)?;
    if extent != store.extent() {
        return Err(StoreError::Corrupt {
            path: store.dir.clone(),
            reason: format!("legacy extent changed from {} to {extent}", store.extent()),
        });
    }
    Ok(())
}
