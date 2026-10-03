#![cfg(test)]
//! LYSLOGSTORE-008 R4: a v1 directory, built here from the old layout's own
//! bytes and never through the store, is migrated once at the first writable
//! open, kept aside whole, and left alone by every open after. A crash at
//! either rename of the switch is finished or refused by name, never
//! guessed at, and never built over with an empty store.

use std::path::Path;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;

use crate::error::StoreError;
use crate::file::FileLeafStore;
use crate::frontier::Frontier;
use crate::migrate::{kept_name, migrate_v1};
use crate::store::{LeafStore, PinnedRoot};

type Fallible<T> = Result<T, Box<dyn std::error::Error>>;
type Outcome = Fallible<()>;
/// A directory's files by relative name and bytes, sorted.
type Listing = Vec<(String, Vec<u8>)>;

const ORIGIN: &str = "example.com/lys/migrate-test";

/// Write a v1 directory by hand: the marker, the pin, one file per leaf, and
/// the snapshot slot when given. `pinned` leaves are under the pin; the rest
/// sit past it, as an unfinished v1 append would.
fn v1_fixture(
    dir: &Path,
    leaves: &[&[u8]],
    pinned: usize,
    snapshot: Option<&[u8]>,
) -> Fallible<PinnedRoot> {
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

fn listing(dir: &Path) -> Fallible<Listing> {
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

/// The leaves every switch case below starts from.
const LEAVES: [&[u8]; 5] = [b"one", b"two", b"three", b"four", b"five"];

/// The state a crash between the two renames leaves: the v1 directory kept
/// under `<name>.v1`, the built copy beside it as `<name>.migrating`, and
/// nothing at the store's path. Built from a real migration of a v1 fixture
/// at `dir`, stopped by moving the finished store back to the copy's name.
/// Answers the v1 pin and the kept directory's listing.
fn between_the_renames(dir: &Path) -> Fallible<(PinnedRoot, Listing)> {
    let pin = v1_fixture(dir, &LEAVES, 5, Some(b"snapshot bytes"))?;
    let before = listing(dir)?;
    migrate_v1(dir)?.ok_or("nothing migrated")?;
    std::fs::rename(dir, dir.with_file_name("directory.migrating"))?;
    assert!(!dir.exists());
    assert_eq!(listing(&kept_name(dir)?)?, before);
    Ok((pin, before))
}

#[test]
fn a_crash_between_the_two_renames_is_finished_by_the_next_open() -> Outcome {
    let root = tempfile::tempdir()?;
    let dir = root.path().join("directory");
    let (pin, kept_before) = between_the_renames(&dir)?;

    let err = FileLeafStore::open_read_only(&dir).unwrap_err();
    assert!(
        matches!(err, StoreError::MigrationPending { .. }),
        "a reader refuses the window by name: {err}"
    );
    assert!(!dir.exists(), "and changes nothing");

    let store = FileLeafStore::open(&dir)?;
    let finished = store.migrated().ok_or("the open finished no switch")?;
    assert_eq!(finished.leaves, 5);
    assert_eq!(finished.beyond_pin, 0);
    assert!(finished.snapshot);
    assert_eq!(finished.kept, kept_name(&dir)?);
    assert_eq!(store.extent(), 5);
    assert_eq!(store.pinned(), pin);
    for (index, bytes) in (0..).zip(&LEAVES) {
        assert_eq!(store.leaf(index)?.as_deref(), Some(*bytes));
    }
    assert_eq!(store.snapshot()?.as_deref(), Some(&b"snapshot bytes"[..]));
    drop(store);
    assert!(!root.path().join("directory.migrating").exists());
    assert_eq!(
        listing(&kept_name(&dir)?)?,
        kept_before,
        "the v1 copy is kept whole"
    );
    assert_eq!(migrate_v1(&dir)?, None, "the next open finds nothing to do");
    FileLeafStore::open_read_only(&dir)?;
    Ok(())
}

#[test]
fn the_create_a_server_start_falls_back_to_finishes_the_switch_instead_of_an_empty_store() -> Outcome
{
    let root = tempfile::tempdir()?;
    let dir = root.path().join("directory");
    let (pin, kept_before) = between_the_renames(&dir)?;
    // The identity server creates a store wherever log.json is missing and
    // opens it after; the window must not answer that with an empty log.
    assert!(!dir.join("log.json").exists());
    let created = FileLeafStore::create(&dir, ORIGIN)?;
    assert_eq!(created.extent(), 5, "the kept history, not an empty store");
    assert_eq!(created.pinned(), pin);
    let finished = created.migrated().ok_or("create finished no switch")?;
    assert_eq!(finished.leaves, 5);
    drop(created);
    let reopened = FileLeafStore::open(&dir)?;
    assert_eq!(reopened.extent(), 5);
    assert_eq!(reopened.migrated(), None);
    assert_eq!(listing(&kept_name(&dir)?)?, kept_before);
    assert!(!root.path().join("directory.migrating").exists());
    Ok(())
}

#[test]
fn a_kept_copy_standing_alone_is_refused_by_name_and_never_built_over() -> Outcome {
    let root = tempfile::tempdir()?;
    let dir = root.path().join("directory");
    let (_pin, kept_before) = between_the_renames(&dir)?;
    std::fs::remove_dir_all(root.path().join("directory.migrating"))?;

    for attempt in [
        FileLeafStore::open(&dir),
        FileLeafStore::create(&dir, ORIGIN),
    ] {
        let err = attempt.err().ok_or("the kept copy alone was guessed at")?;
        assert!(
            matches!(err, StoreError::MigrationCopyMissing { .. }),
            "{err}"
        );
        assert!(
            err.to_string().contains("rename the kept copy back"),
            "the refusal names the fix: {err}"
        );
    }
    let err = FileLeafStore::open_read_only(&dir).unwrap_err();
    assert!(matches!(err, StoreError::MigrationPending { .. }), "{err}");
    assert!(!dir.exists(), "nothing was built at the store's path");
    assert_eq!(
        listing(&kept_name(&dir)?)?,
        kept_before,
        "the kept copy is untouched"
    );
    Ok(())
}

#[test]
fn a_built_copy_that_does_not_match_the_kept_bytes_is_refused_and_both_stay() -> Outcome {
    let root = tempfile::tempdir()?;
    let dir = root.path().join("directory");
    let (_pin, kept_before) = between_the_renames(&dir)?;
    let building = root.path().join("directory.migrating");
    std::fs::remove_dir_all(&building)?;
    let mut other = FileLeafStore::create(&building, ORIGIN)?;
    let frontier = Frontier::from_leaves(&LEAVES[..4]);
    other.append(
        0,
        &LEAVES[..4],
        PinnedRoot {
            tree_size: frontier.size(),
            root: frontier.root(),
        },
    )?;
    drop(other);
    let copy_before = listing(&building)?;

    let err = FileLeafStore::open(&dir).unwrap_err();
    assert!(matches!(err, StoreError::Corrupt { .. }), "{err}");
    assert!(!dir.exists(), "the mismatched copy was not switched in");
    assert_eq!(listing(&building)?, copy_before, "and was not removed");
    assert_eq!(
        listing(&kept_name(&dir)?)?,
        kept_before,
        "the kept copy is untouched"
    );
    Ok(())
}

#[test]
fn an_unfinished_copy_beside_a_v1_store_is_rebuilt_before_the_switch() -> Outcome {
    let root = tempfile::tempdir()?;
    let dir = root.path().join("directory");
    let pin = v1_fixture(&dir, &LEAVES, 5, None)?;
    // A crash before the first rename: the v1 directory is still the store
    // and the copy beside it is whatever the stopped attempt left.
    let building = root.path().join("directory.migrating");
    std::fs::create_dir_all(&building)?;
    std::fs::write(building.join("log.json"), b"half-written")?;
    let migrated = migrate_v1(&dir)?.ok_or("nothing migrated")?;
    assert_eq!(migrated.leaves, 5);
    assert!(!building.exists());
    let store = FileLeafStore::open(&dir)?;
    assert_eq!(store.extent(), 5);
    assert_eq!(store.pinned(), pin);
    Ok(())
}
