//! A move or an end is asked of one holding, named by the operation it was
//! assigned with, and a move of the version it was seen at. Asked late, of a
//! holding that is no longer the holder's last or no longer where it was
//! seen, it changes nothing.

use std::error::Error;

use lys_identity_server::error::ServerError;
use lys_identity_server::roles_records::{Ending, Holding, Move, Version, Words};
use lys_identity_server::roles_store::RolesStore;

type TestResult = Result<(), Box<dyn Error>>;

fn words(note: &str) -> Words {
    Words {
        responsibilities: "Builds what the brief says.".to_owned(),
        goals: "Green gates.".to_owned(),
        practice: "Small changes.".to_owned(),
        profile: String::new(),
        grant_templates: Vec::new(),
        note: note.to_owned(),
    }
}

fn holding(operation: &str, at: u64, ends_at: Option<u64>) -> Holding {
    Holding {
        operation: operation.to_owned(),
        holder: "person-b".to_owned(),
        version: 0,
        assigned_by: "person-a".to_owned(),
        assigned_at: at,
        ends_at,
        moves: Vec::new(),
        ended: None,
    }
}

fn moved(from: u32, to: u32, at: u64) -> Move {
    Move {
        from,
        to,
        by: "person-a".to_owned(),
        at,
    }
}

fn ending(at: u64) -> Ending {
    Ending {
        by: "person-a".to_owned(),
        at,
    }
}

/// A store holding the role `role-1` at three versions, with `person-b`
/// assigned as `hold-1` at the first.
fn kept(dir: &tempfile::TempDir, ends_at: Option<u64>) -> Result<RolesStore, Box<dyn Error>> {
    let mut store = RolesStore::open(&dir.path().join("roles.json"))?;
    let first = Version {
        number: 1,
        operation: "role-1".to_owned(),
        words: words("first"),
        made_by: "person-a".to_owned(),
        made_at: 10,
    };
    store.make("Builder".to_owned(), first)?;
    store.assign("role-1", holding("hold-1", 20, ends_at))?;
    store.revise("role-1", "version-2", words("second"), "person-a", 30)?;
    store.revise("role-1", "version-3", words("third"), "person-a", 31)?;
    Ok(store)
}

#[test]
fn a_move_is_made_once_and_only_from_the_version_it_was_seen_at() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mut store = kept(&dir, None)?;
    store.move_holder("role-1", "person-b", "hold-1", moved(1, 2, 40))?;
    store.move_holder("role-1", "person-b", "hold-1", moved(1, 2, 41))?;
    let stale = store.move_holder("role-1", "person-b", "hold-1", moved(1, 3, 42));
    assert!(
        matches!(stale, Err(ServerError::HoldingChanged)),
        "{stale:?}"
    );
    let role = store.role("role-1").ok_or("no role")?;
    assert_eq!(role.holdings[0].version, 2);
    assert_eq!(role.holdings[0].moves, [moved(1, 2, 40)]);
    Ok(())
}

#[test]
fn a_change_asked_of_an_earlier_holding_leaves_the_later_one() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mut store = kept(&dir, Some(100))?;
    store.assign("role-1", holding("hold-2", 150, None))?;
    let before = store.roles().to_vec();

    let late = store.end("role-1", "person-b", "hold-1", ending(160));
    assert!(matches!(late, Err(ServerError::HoldingChanged)), "{late:?}");
    let late = store.move_holder("role-1", "person-b", "hold-1", moved(1, 2, 161));
    assert!(matches!(late, Err(ServerError::HoldingChanged)), "{late:?}");
    assert_eq!(store.roles(), before, "nothing was changed");

    store.end("role-1", "person-b", "hold-2", ending(170))?;
    let role = store.role("role-1").ok_or("no role")?;
    assert_eq!(role.holdings[0].ended, None);
    assert_eq!(role.holdings[1].ended, Some(ending(170)));
    Ok(())
}

#[test]
fn a_change_names_a_holding_the_holder_has() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mut store = kept(&dir, None)?;
    let before = store.roles().to_vec();
    let other = store.end("role-1", "person-b", "hold-9", ending(40));
    assert!(
        matches!(other, Err(ServerError::HoldingChanged)),
        "{other:?}"
    );
    let nobody = store.end("role-1", "person-c", "hold-1", ending(40));
    assert!(
        matches!(nobody, Err(ServerError::HolderUnknown)),
        "{nobody:?}"
    );
    assert_eq!(store.roles(), before, "nothing was changed");
    Ok(())
}
