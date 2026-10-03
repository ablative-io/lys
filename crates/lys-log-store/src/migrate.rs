//! LYSLOGSTORE-008 R4: the one migration out of the v1 per-leaf layout.
//!
//! An install that cannot open its own stores, or wipes them, is not an
//! install (Tom, 3 October 2026). A v1 directory is migrated once, at the
//! first writable open: its leaves under the pin are read as bytes, appended
//! to a new store beside it as one act with the same pin, the new store is
//! reopened and checked by extent, pin and every leaf's bytes, and only then
//! is the v1 directory renamed aside and kept, never deleted, and the new one
//! renamed into its place. A second open finds the new layout and does
//! nothing. A read-only open of a v1 directory is refused by name; a reader
//! never changes a directory. The migration is reported to the opener so the
//! start record can count it.
//!
//! Leaves past the pin, an append the v1 store never pinned, are not
//! migrated: the kept directory still holds them, and their count is
//! reported. The leaves under the pin must hash to the pinned root before
//! anything is written; a v1 directory that does not match its own pin is
//! refused and left exactly as it was.
//!
//! The switch is two renames, `<name>` to `<name>.v1` and then
//! `<name>.migrating` to `<name>`, and a crash between them leaves nothing at
//! the store's path. That state is looked for first, by every writable open
//! and by [`FileLeafStore::create`]: the built copy is checked against the
//! kept v1 bytes again and the second rename is finished. A kept copy with
//! nothing beside it is refused by name, never guessed at, and never built
//! over with an empty store.

use std::path::{Path, PathBuf};

use crate::error::{StoreError, StoreResult};
use crate::file::{FileLeafStore, v1};
use crate::frontier::Frontier;
use crate::store::LeafStore;

/// What one migration did, for the start record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Migrated {
    /// Where the v1 directory is kept, whole.
    pub kept: PathBuf,
    /// The leaves migrated: every leaf under the v1 pin.
    pub leaves: u64,
    /// Leaves the v1 directory held past its pin, left in the kept copy.
    pub beyond_pin: u64,
    /// Whether a snapshot was carried over.
    pub snapshot: bool,
}

/// The name the v1 directory is kept under: the store's own name with `.v1`.
pub fn kept_name(dir: &Path) -> StoreResult<PathBuf> {
    sibling(dir, "v1")
}

/// Migrate the store at `dir` out of the v1 layout, when it is in one, or
/// finish a switch a crash interrupted. Answers `None` when there is nothing
/// to do: the directory is not a v1 store and no switch is pending.
///
/// # Errors
///
/// [`StoreError::Corrupt`] when the v1 directory does not match its own pin
/// or the new store does not read back what was written;
/// [`StoreError::MigrationKeptExists`] when the kept name is taken;
/// [`StoreError::MigrationCopyMissing`] when only the kept copy remains; and
/// the errors of creating and appending to the new store. On any error the
/// v1 directory is as it was.
pub fn migrate_v1(dir: &Path) -> StoreResult<Option<Migrated>> {
    if let Some(finished) = finish_switch(dir)? {
        return Ok(Some(finished));
    }
    if !v1::present(dir)? {
        return Ok(None);
    }
    let contents = checked_contents(dir)?;
    let kept = kept_name(dir)?;
    if kept.exists() {
        return Err(StoreError::MigrationKeptExists { path: kept });
    }
    let building = sibling(dir, "migrating")?;
    if building.exists() {
        // An earlier attempt of this migration that never reached the switch:
        // the v1 directory is still the store, and this is only our own
        // unfinished copy of it.
        std::fs::remove_dir_all(&building).map_err(|source| StoreError::Io {
            context: format!(
                "failed to clear the unfinished migration at {}",
                building.display()
            ),
            source,
        })?;
    }
    let mut store = FileLeafStore::create(&building, &contents.origin)?;
    if !contents.leaves.is_empty() {
        let borrowed: Vec<&[u8]> = contents.leaves.iter().map(Vec::as_slice).collect();
        store.append(0, &borrowed, contents.pinned)?;
    }
    if let Some(snapshot) = &contents.snapshot {
        store.put_snapshot(snapshot)?;
    }
    drop(store);
    verify(&building, &contents)?;
    rename(dir, &kept)?;
    rename(&building, dir)?;
    sync_parent(dir)?;
    Ok(Some(Migrated {
        kept,
        leaves: contents.pinned.tree_size,
        beyond_pin: contents.beyond_pin,
        snapshot: contents.snapshot.is_some(),
    }))
}

/// Whether a switch is pending at `dir`: nothing at the store's path, and
/// the kept v1 copy beside it. A reader refuses this state by name; only a
/// writable open or [`FileLeafStore::create`] finishes it.
///
/// # Errors
///
/// [`StoreError::Corrupt`] when `dir` has no name to keep a copy under.
pub fn switch_pending(dir: &Path) -> StoreResult<bool> {
    Ok(!dir.exists() && kept_name(dir)?.exists())
}

/// Finish a switch a crash interrupted between its two renames: `dir` is
/// absent, the v1 directory is already kept, and the built copy stands
/// beside it. The copy is checked against the kept bytes as it was before
/// the first rename, then renamed into place. Answers `None` when no switch
/// is pending.
///
/// # Errors
///
/// [`StoreError::MigrationCopyMissing`] when the kept copy stands alone:
/// nothing is guessed and nothing is built over it. [`StoreError::Corrupt`]
/// when the kept copy does not match its own pin or the built copy does not
/// read back the kept bytes; then both stay where they are.
pub fn finish_switch(dir: &Path) -> StoreResult<Option<Migrated>> {
    if !switch_pending(dir)? {
        return Ok(None);
    }
    let kept = kept_name(dir)?;
    let building = sibling(dir, "migrating")?;
    if !building.exists() {
        return Err(StoreError::MigrationCopyMissing {
            path: dir.to_path_buf(),
            kept,
        });
    }
    let contents = checked_contents(&kept)?;
    verify(&building, &contents)?;
    rename(&building, dir)?;
    sync_parent(dir)?;
    Ok(Some(Migrated {
        kept,
        leaves: contents.pinned.tree_size,
        beyond_pin: contents.beyond_pin,
        snapshot: contents.snapshot.is_some(),
    }))
}

/// Read a v1 directory and refuse one whose leaves under the pin do not hash
/// to its pinned root.
fn checked_contents(dir: &Path) -> StoreResult<v1::Contents> {
    let contents = v1::read(dir)?;
    let frontier = Frontier::from_leaves(&contents.leaves);
    if frontier.size() != contents.pinned.tree_size || frontier.root() != contents.pinned.root {
        return Err(StoreError::Corrupt {
            path: dir.to_path_buf(),
            reason: format!(
                "the {} leaves under the pin do not hash to the pinned root; nothing migrated",
                contents.pinned.tree_size
            ),
        });
    }
    Ok(contents)
}

/// Reopen the new store and check it holds exactly what the v1 directory did.
fn verify(building: &Path, contents: &v1::Contents) -> StoreResult<()> {
    let corrupt = |reason: String| StoreError::Corrupt {
        path: building.to_path_buf(),
        reason,
    };
    let check = FileLeafStore::open_read_only(building)?;
    if check.extent() != contents.pinned.tree_size {
        return Err(corrupt(format!(
            "the migrated store holds {} leaves, not the {} under the pin",
            check.extent(),
            contents.pinned.tree_size
        )));
    }
    if check.pinned() != contents.pinned {
        return Err(corrupt(
            "the migrated store's pin is not the v1 pin".to_owned(),
        ));
    }
    for (index, bytes) in (0..).zip(&contents.leaves) {
        if check.leaf(index)?.as_deref() != Some(bytes.as_slice()) {
            return Err(corrupt(format!(
                "leaf {index} of the migrated store is not the v1 leaf"
            )));
        }
    }
    if check.snapshot()? != contents.snapshot {
        return Err(corrupt(
            "the migrated store's snapshot is not the v1 snapshot".to_owned(),
        ));
    }
    Ok(())
}

fn sibling(dir: &Path, suffix: &str) -> StoreResult<PathBuf> {
    let name = dir
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| StoreError::Corrupt {
            path: dir.to_path_buf(),
            reason: "the store's directory has no name to keep it under".to_owned(),
        })?;
    Ok(dir.with_file_name(format!("{name}.{suffix}")))
}

fn rename(from: &Path, to: &Path) -> StoreResult<()> {
    std::fs::rename(from, to).map_err(|source| StoreError::Io {
        context: format!("failed to rename {} to {}", from.display(), to.display()),
        source,
    })
}

/// Flush the directory holding `dir`, so both renames of the switch survive
/// a crash together.
fn sync_parent(dir: &Path) -> StoreResult<()> {
    let parent = dir.parent().filter(|parent| !parent.as_os_str().is_empty());
    let Some(parent) = parent else {
        return Ok(());
    };
    let opened = std::fs::File::open(parent).map_err(|source| StoreError::Io {
        context: format!("failed to open {} to flush it", parent.display()),
        source,
    })?;
    crate::durability::sync_all(&opened).map_err(|source| StoreError::Io {
        context: format!("failed to flush {} to disk", parent.display()),
        source,
    })
}
