#![cfg(test)]
//! LYSLOGSTORE-008 R4: a v1 directory, built here from the old layout's own
//! bytes and never through the store, is migrated once at the first writable
//! open, kept aside whole, and left alone by every open after.
//!
//! These cases hold once this build's layout is past v1; while it is still
//! v1 the migration is inert by construction and they say so by failing.

use std::path::Path;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;

use crate::error::StoreError;
use crate::file::FileLeafStore;
use crate::frontier::Frontier;
use crate::migrate::{kept_name, migrate_v1};
use crate::store::{LeafStore, PinnedRoot};

type Outcome = Result<(), Box<dyn std::error::Error>>;

const ORIGIN: &str = "example.com/lys/migrate-test";

/// Write a v1 directory by hand: the marker, the pin, one file per leaf, and
/// the snapshot slot when given. `pinned` leaves are under the pin; the rest
/// sit past it, as an unfinished v1 append would.
fn v1_fixture(
    dir: &Path,
    leaves: &[&[u8]],
    pinned: usize,
    snapshot: Option<&[u8]>,
) -> Result<PinnedRoot, Box<dyn std::error::Error>> {
    std::fs::create_dir_all(dir.join("leaves"))?;
    std::fs::write(
        dir.join("log.json"),
        format!("{{\"format\":\"lys/log-dir/v1\",\"origin\":\"{ORIGIN}\"}}"),
    )?;
    for (index, bytes) in leaves.iter().enumerate() {
        std::fs::write(dir.join("leaves").join(format!("{index:020}")), bytes)?;
    }
    let frontier = Frontier::from_leaves(&leaves[..pinned]);
    let pin = PinnedRoot {
        tree_size: frontier.size(),
        root: frontier.root(),
    };
    std::fs::write(
        dir.join("state.json"),
        format!(
            "{{\"tree_size\":{},\"root_hash\":\"{}\"}}",
            pin.tree_size,
            STANDARD.encode(pin.root)
        ),
    )?;
    if let Some(bytes) = snapshot {
        std::fs::write(dir.join("snapshot.bin"), bytes)?;
    }
    Ok(pin)
}

fn listing(dir: &Path) -> Result<Vec<(String, Vec<u8>)>, Box<dyn std::error::Error>> {
    let mut entries = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if entry.file_type()?.is_dir() {
            for (inner, bytes) in listing(&entry.path())? {
                entries.push((format!("{name}/{inner}"), bytes));
            }
        } else {
            entries.push((name, std::fs::read(entry.path())?));
        }
    }
    entries.sort();
    Ok(entries)
}

#[test]
fn a_v1_store_is_migrated_once_and_kept_aside_whole() -> Outcome {
    let root = tempfile::tempdir()?;
    let dir = root.path().join("directory");
    let leaves: Vec<&[u8]> = vec![b"one", b"two", b"three", b"four", b"five"];
    let pin = v1_fixture(&dir, &leaves, 5, Some(b"snapshot bytes"))?;
    let before = listing(&dir)?;

    let migrated = migrate_v1(&dir)?.ok_or("nothing migrated")?;
    assert_eq!(migrated.leaves, 5);
    assert_eq!(migrated.beyond_pin, 0);
    assert!(migrated.snapshot);
    assert_eq!(migrated.kept, kept_name(&dir)?);
    assert_eq!(
        listing(&migrated.kept)?,
        before,
        "the v1 directory is kept whole"
    );
    assert!(!root.path().join("directory.migrating").exists());

    let store = FileLeafStore::open(&dir)?;
    assert_eq!(store.extent(), 5);
    assert_eq!(store.pinned(), pin);
    for (index, bytes) in (0..).zip(&leaves) {
        assert_eq!(store.leaf(index)?.as_deref(), Some(*bytes));
    }
    assert_eq!(store.snapshot()?.as_deref(), Some(&b"snapshot bytes"[..]));
    drop(store);

    let after = listing(&dir)?;
    assert_eq!(
        migrate_v1(&dir)?,
        None,
        "a second open finds the new layout"
    );
    assert_eq!(listing(&dir)?, after, "and changes nothing");
    FileLeafStore::open_read_only(&dir)?;
    Ok(())
}

#[test]
fn leaves_past_the_v1_pin_are_not_migrated_and_are_counted() -> Outcome {
    let root = tempfile::tempdir()?;
    let dir = root.path().join("directory");
    let leaves: Vec<&[u8]> = vec![b"one", b"two", b"three"];
    let pin = v1_fixture(&dir, &leaves, 2, None)?;
    let migrated = migrate_v1(&dir)?.ok_or("nothing migrated")?;
    assert_eq!(migrated.leaves, 2);
    assert_eq!(migrated.beyond_pin, 1);
    assert!(!migrated.snapshot);
    let store = FileLeafStore::open(&dir)?;
    assert_eq!(store.extent(), 2);
    assert_eq!(store.pinned(), pin);
    assert!(
        migrated
            .kept
            .join("leaves")
            .join(format!("{:020}", 2))
            .exists(),
        "the kept copy still holds the unpinned leaf"
    );
    Ok(())
}

#[test]
fn a_v1_store_that_does_not_match_its_own_pin_is_refused_and_untouched() -> Outcome {
    let root = tempfile::tempdir()?;
    let dir = root.path().join("directory");
    let leaves: Vec<&[u8]> = vec![b"one", b"two"];
    v1_fixture(&dir, &leaves, 2, None)?;
    std::fs::write(dir.join("leaves").join(format!("{:020}", 1)), b"not two")?;
    let before = listing(&dir)?;
    let err = migrate_v1(&dir).unwrap_err();
    assert!(matches!(err, StoreError::Corrupt { .. }), "{err}");
    assert_eq!(listing(&dir)?, before);
    assert!(!kept_name(&dir)?.exists());
    assert!(!root.path().join("directory.migrating").exists());
    Ok(())
}

#[test]
fn a_kept_name_already_taken_refuses_the_migration_by_name() -> Outcome {
    let root = tempfile::tempdir()?;
    let dir = root.path().join("directory");
    let leaves: Vec<&[u8]> = vec![b"one"];
    v1_fixture(&dir, &leaves, 1, None)?;
    std::fs::create_dir_all(kept_name(&dir)?)?;
    let before = listing(&dir)?;
    let err = migrate_v1(&dir).unwrap_err();
    assert!(
        matches!(err, StoreError::MigrationKeptExists { .. }),
        "{err}"
    );
    assert_eq!(listing(&dir)?, before);
    Ok(())
}

#[test]
fn a_directory_that_is_not_v1_is_left_alone() -> Outcome {
    let root = tempfile::tempdir()?;
    let dir = root.path().join("directory");
    FileLeafStore::create(&dir, ORIGIN)?;
    let before = listing(&dir)?;
    assert_eq!(migrate_v1(&dir)?, None);
    assert_eq!(listing(&dir)?, before);
    assert_eq!(migrate_v1(&root.path().join("absent"))?, None);
    Ok(())
}
