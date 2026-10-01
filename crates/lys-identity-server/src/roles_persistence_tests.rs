#![cfg(test)]

use std::error::Error;
use std::fs;
use std::path::Path;

use lys_identity::SNAPSHOT_EVERY;

use super::changes::Change;
use super::persistence::HEADER;
use super::{Kept, RECOVERED_TAILS, REPLAYED_CHANGES, RolesStore, WRITTEN_BYTES};
use crate::error::ServerError;
use crate::roles_records::{Ending, Holding, Move, Role, Version, Words};

type TestResult = Result<(), Box<dyn Error>>;

fn words() -> Words {
    Words {
        responsibilities: "Build the approved change.".to_owned(),
        goals: "Correct durable records.".to_owned(),
        practice: "Read before changing.".to_owned(),
        profile: String::new(),
        grant_templates: Vec::new(),
        note: "Keep prior versions.".to_owned(),
    }
}

fn version(number: u32, operation: &str) -> Version {
    Version {
        number,
        operation: operation.to_owned(),
        words: words(),
        made_by: "person-a".to_owned(),
        made_at: u64::from(number),
    }
}

fn legacy() -> Kept {
    Kept {
        roles: vec![Role {
            id: "role-1".to_owned(),
            name: "Builder".to_owned(),
            versions: (1..=256)
                .map(|number| {
                    let operation = if number == 1 {
                        "role-1".to_owned()
                    } else {
                        format!("version-{number}")
                    };
                    version(number, &operation)
                })
                .collect(),
            holdings: Vec::new(),
        }],
    }
}

fn holding() -> Holding {
    Holding {
        operation: "hold-1".to_owned(),
        holder: "person-b".to_owned(),
        version: 0,
        assigned_by: "person-a".to_owned(),
        assigned_at: 300,
        ends_at: Some(900),
        moves: Vec::new(),
        ended: None,
    }
}

fn bounded_write(path: &Path, change: impl FnOnce() -> Result<(), ServerError>) -> TestResult {
    let before = fs::read(path)?;
    WRITTEN_BYTES.with(|written| written.set(0));
    change()?;
    let bytes = WRITTEN_BYTES.with(std::cell::Cell::get);
    assert!(bytes > 0, "a successful change must be written");
    assert!(
        bytes <= 1024,
        "one small role change wrote {bytes} bytes of retained history"
    );
    let after = fs::read(path)?;
    assert!(
        after.starts_with(&before),
        "prior records are never rewritten"
    );
    assert_eq!(after.len().checked_sub(before.len()), Some(bytes));
    Ok(())
}

#[test]
fn normal_changes_append_only_the_change_as_history_grows() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("roles.json");
    fs::write(&path, serde_json::to_vec_pretty(&legacy())?)?;
    let mut store = RolesStore::open(&path)?;
    store.revise("role-1", "version-257", words(), "person-a", 290)?;
    bounded_write(&path, || store.assign("role-1", holding()))?;
    bounded_write(&path, || {
        store
            .revise("role-1", "version-258", words(), "person-a", 310)
            .map(|number| assert_eq!(number, 258))
    })?;
    bounded_write(&path, || {
        store.move_holder(
            "role-1",
            "person-b",
            "hold-1",
            Move {
                from: 257,
                to: 258,
                by: "person-a".to_owned(),
                at: 320,
            },
        )
    })?;
    bounded_write(&path, || {
        store.end(
            "role-1",
            "person-b",
            "hold-1",
            Ending {
                by: "person-a".to_owned(),
                at: 330,
            },
        )
    })?;
    bounded_write(&path, || {
        store.make("Reviewer".to_owned(), version(1, "role-2"))
    })?;
    let expected = store.roles().to_vec();
    drop(store);
    assert_eq!(RolesStore::open(&path)?.roles(), expected);
    Ok(())
}

#[test]
fn an_old_install_preserves_roles_and_idempotency_through_changes_and_restart() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("roles.json");
    let old = legacy();
    let old_bytes = serde_json::to_vec_pretty(&old)?;
    fs::write(&path, &old_bytes)?;
    let mut store = RolesStore::open(&path)?;
    assert_eq!(store.roles(), old.roles);
    assert_eq!(
        fs::read(&path)?,
        old_bytes,
        "opening never rewrites an old install"
    );
    store.make("Builder".to_owned(), version(1, "role-1"))?;
    let reused = store.make("Other".to_owned(), version(1, "role-1"));
    assert!(matches!(reused, Err(ServerError::RoleReused { .. })));
    store.assign("role-1", holding())?;
    assert!(fs::read(&path)?.starts_with(super::persistence::HEADER));
    store.assign("role-1", holding())?;
    let held = store.assign(
        "role-1",
        Holding {
            operation: "hold-2".to_owned(),
            ..holding()
        },
    );
    assert!(matches!(held, Err(ServerError::RoleHeld { .. })));
    let expected = store.roles().to_vec();
    drop(store);
    let mut store = RolesStore::open(&path)?;
    assert_eq!(store.roles(), expected);
    store.assign("role-1", holding())?;
    assert_eq!(store.roles(), expected);
    let entries = fs::read_dir(dir.path())?.collect::<Result<Vec<_>, _>>()?;
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].path(), path);
    Ok(())
}

#[test]
fn a_torn_final_change_is_durably_removed_before_later_changes() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("roles.json");
    let mut store = RolesStore::open(&path)?;
    store.make("Builder".to_owned(), version(1, "role-1"))?;
    let expected = store.roles().to_vec();
    drop(store);
    let complete = fs::read(&path)?;
    let mut bytes = complete.clone();
    bytes.extend_from_slice(b"{\"change\":\"Revise\",\"role\":0");
    fs::write(&path, &bytes)?;
    RECOVERED_TAILS.with(|recovered| recovered.set(0));
    let mut store = RolesStore::open(&path)?;
    assert_eq!(store.roles(), expected);
    assert_eq!(fs::read(&path)?, complete);
    assert_eq!(RECOVERED_TAILS.with(std::cell::Cell::get), 1);
    bounded_write(&path, || {
        store
            .revise("role-1", "version-2", words(), "person-a", 400)
            .map(|number| assert_eq!(number, 2))
    })?;
    let expected = store.roles().to_vec();
    drop(store);
    assert_eq!(RolesStore::open(&path)?.roles(), expected);
    Ok(())
}

#[test]
fn complete_corruption_and_an_incomplete_snapshot_are_refused_without_truncation() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("roles.json");
    let mut store = RolesStore::open(&path)?;
    store.make("Builder".to_owned(), version(1, "role-1"))?;
    drop(store);
    let mut bytes = fs::read(&path)?;
    bytes.extend_from_slice(b"{broken}\n");
    fs::write(&path, &bytes)?;
    assert!(matches!(
        RolesStore::open(&path),
        Err(ServerError::RolesUnavailable { .. })
    ));
    assert_eq!(fs::read(&path)?, bytes);
    let mut incomplete = HEADER.to_vec();
    incomplete.extend_from_slice(b"{\"roles\":[]");
    fs::write(&path, &incomplete)?;
    assert!(matches!(
        RolesStore::open(&path),
        Err(ServerError::RolesUnavailable { .. })
    ));
    assert_eq!(fs::read(&path)?, incomplete);
    Ok(())
}

fn seed_journal(path: &Path, count: u64) -> Result<Kept, Box<dyn Error>> {
    let mut kept = Kept {
        roles: vec![Role {
            id: "role-1".to_owned(),
            name: "Builder".to_owned(),
            versions: vec![version(1, "role-1")],
            holdings: Vec::new(),
        }],
    };
    let mut bytes = HEADER.to_vec();
    serde_json::to_writer(&mut bytes, &kept)?;
    bytes.push(b'\n');
    for offset in 0..count {
        let number = u32::try_from(offset + 2)?;
        let change = Change::Revise {
            role: 0,
            version: version(number, &format!("version-{number}")),
        };
        serde_json::to_writer(&mut bytes, &change)?;
        bytes.push(b'\n');
        change.apply(&mut kept)?;
    }
    fs::write(path, bytes)?;
    Ok(kept)
}

#[test]
fn checkpoint_boundaries_borrow_state_and_bound_actual_replay() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("roles.json");
    let count = SNAPSHOT_EVERY.get();
    let expected = seed_journal(&path, count - 1)?;
    REPLAYED_CHANGES.with(|replayed| replayed.set(0));
    let mut store = RolesStore::open(&path)?;
    assert_eq!(store.roles(), expected.roles);
    assert_eq!(REPLAYED_CHANGES.with(std::cell::Cell::get), count - 1);
    let boundary = u32::try_from(count + 1)?;
    assert_eq!(
        store.revise(
            "role-1",
            &format!("version-{boundary}"),
            words(),
            "person-a",
            400,
        )?,
        boundary
    );
    drop(store);
    REPLAYED_CHANGES.with(|replayed| replayed.set(0));
    let mut store = RolesStore::open(&path)?;
    assert_eq!(REPLAYED_CHANGES.with(std::cell::Cell::get), 0);
    let next = boundary + 1;
    bounded_write(&path, || {
        store
            .revise(
                "role-1",
                &format!("version-{next}"),
                words(),
                "person-a",
                410,
            )
            .map(|number| assert_eq!(number, next))
    })?;
    let expected = store.roles().to_vec();
    drop(store);
    REPLAYED_CHANGES.with(|replayed| replayed.set(0));
    assert_eq!(RolesStore::open(&path)?.roles(), expected);
    assert_eq!(REPLAYED_CHANGES.with(std::cell::Cell::get), 1);
    Ok(())
}

#[test]
fn a_crashed_checkpoint_refuses_the_next_append_until_the_snapshot_is_written() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("roles.json");
    let count = SNAPSHOT_EVERY.get();
    let expected = seed_journal(&path, count)?;
    let mut store = RolesStore::open(&path)?;
    let before = fs::read(&path)?;
    fs::create_dir(path.with_extension("writing"))?;
    let next = u32::try_from(count + 2)?;
    let failed = store.revise(
        "role-1",
        &format!("version-{next}"),
        words(),
        "person-a",
        420,
    );
    assert!(matches!(failed, Err(ServerError::RolesUnavailable { .. })));
    assert_eq!(fs::read(&path)?, before);
    assert_eq!(store.roles(), expected.roles);
    fs::remove_dir(path.with_extension("writing"))?;
    assert_eq!(
        store.revise(
            "role-1",
            &format!("version-{next}"),
            words(),
            "person-a",
            420,
        )?,
        next
    );
    let expected = store.roles().to_vec();
    drop(store);
    REPLAYED_CHANGES.with(|replayed| replayed.set(0));
    assert_eq!(RolesStore::open(&path)?.roles(), expected);
    assert_eq!(REPLAYED_CHANGES.with(std::cell::Cell::get), 1);
    Ok(())
}

#[test]
fn an_unwritten_change_never_runs_ahead_of_the_old_install() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("roles.json");
    let old = legacy();
    let old_bytes = serde_json::to_vec_pretty(&old)?;
    fs::write(&path, &old_bytes)?;
    let mut store = RolesStore::open(&path)?;
    fs::create_dir(path.with_extension("writing"))?;
    let change = store.assign("role-1", holding());
    assert!(matches!(change, Err(ServerError::RolesUnavailable { .. })));
    assert_eq!(store.roles(), old.roles);
    assert_eq!(fs::read(&path)?, old_bytes);
    fs::remove_dir(path.with_extension("writing"))?;
    store.assign("role-1", holding())?;
    let expected = store.roles().to_vec();
    drop(store);
    assert_eq!(RolesStore::open(&path)?.roles(), expected);
    Ok(())
}
