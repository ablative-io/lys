#![cfg(test)]

use std::error::Error;
use std::sync::Arc;

use lys_core::Ed25519Identity;

use super::TeamStore;
use crate::read_views::Login;
use crate::teams_state::{Changed, Checked, Created, Held, Hold, Line};

type TestResult = Result<(), Box<dyn Error>>;

fn created(id: &str) -> Line {
    Line::Created(Created {
        id: id.to_owned(),
        owner: "owner".to_owned(),
        name: id.to_owned(),
        description: String::new(),
        by: Login {
            provider: "issuer".to_owned(),
            subject: "owner".to_owned(),
        },
        at: 1,
    })
}

fn added(team: &str) -> Line {
    Line::Added(Changed {
        operation: format!("{team}-added"),
        team: team.to_owned(),
        member: "member".to_owned(),
        by: Login {
            provider: "issuer".to_owned(),
            subject: "owner".to_owned(),
        },
        at: 2,
    })
}

fn held(operation: &str, team: &str) -> Line {
    Line::Held(Hold {
        operation: operation.to_owned(),
        team: team.to_owned(),
        member: "member".to_owned(),
        reason: "not admitted".to_owned(),
        at: 3,
    })
}

fn checked(operation: &str) -> Line {
    Line::Checked(Checked {
        operation: operation.to_owned(),
        at: 4,
    })
}

fn populated() -> Result<Held, String> {
    let mut held = Held::default();
    for number in 0..64 {
        let id = format!("team-{number}");
        held.hold(created(&id))?;
        held.hold(added(&id))?;
    }
    Ok(held)
}

#[test]
fn refreshing_an_empty_migration_copies_no_team_records() -> TestResult {
    let dir = tempfile::tempdir()?;
    let key = Arc::new(Ed25519Identity::load_or_generate(&dir.path().join("key"))?);
    let mut store = TeamStore::open(&dir.path().join("teams"), key)?;
    store.held = populated()?;
    crate::teams_state::reset_team_copies();
    store.refresh_overlay()?;
    store.stage_migration(Vec::new())?;
    assert_eq!(crate::teams_state::team_copies(), 0);
    assert!(store.overlay.is_none());
    Ok(())
}

#[test]
fn staged_migration_copies_only_affected_teams_and_builds_catalogue_on_demand() -> TestResult {
    let dir = tempfile::tempdir()?;
    let key = Arc::new(Ed25519Identity::load_or_generate(&dir.path().join("key"))?);
    let mut store = TeamStore::open(&dir.path().join("teams"), key)?;
    store.held = populated()?;
    store.held.teams[0].lead = Some("member".to_owned());
    let lines = vec![held("migration-hold", "team-0"), checked("migration-check")];
    let mut expected = store.held.clone();
    for line in &lines {
        expected.hold(line.clone())?;
    }
    let base_bytes = store.held.encode()?;
    crate::teams_state::reset_team_copies();
    store.stage_migration(lines)?;
    assert_eq!(crate::teams_state::team_copies(), 1);
    assert_eq!(store.team("team-0"), expected.team("team-0"));
    assert!(std::ptr::eq(
        store.team("team-63").ok_or("no unchanged team")?,
        store.held.team("team-63").ok_or("no base team")?
    ));
    assert_eq!(store.held.encode()?, base_bytes);
    assert!(!store.migration_checked());
    assert_eq!(
        store.subtree("team-0")?,
        store
            .held
            .subtree("team-0")
            .map_err(|reason| format!("{reason:?}"))?
    );
    crate::teams_state::reset_team_copies();
    assert_eq!(store.teams(), expected.teams.as_slice());
    assert_eq!(crate::teams_state::team_copies(), 64);
    crate::teams_state::reset_team_copies();
    assert_eq!(store.teams(), expected.teams.as_slice());
    assert_eq!(crate::teams_state::team_copies(), 0);
    store.stage_migration(Vec::new())?;
    assert_eq!(store.team("team-0"), store.held.team("team-0"));
    Ok(())
}

#[test]
fn staged_duplicate_operations_members_and_checks_keep_their_refusals() -> TestResult {
    let dir = tempfile::tempdir()?;
    let key = Arc::new(Ed25519Identity::load_or_generate(&dir.path().join("key"))?);
    let mut store = TeamStore::open(&dir.path().join("teams"), key)?;
    store.held = populated()?;
    for lines in [
        vec![held("same", "team-0"), held("same", "team-1")],
        vec![held("first", "team-0"), held("second", "team-0")],
        vec![checked("first"), checked("second")],
    ] {
        let mut expected = store.held.clone();
        let failure = lines
            .iter()
            .find_map(|line| expected.hold(line.clone()).err())
            .ok_or("migration was not refused")?;
        let actual = store
            .stage_migration(lines)
            .err()
            .ok_or("migration accepted")?;
        let crate::error::ServerError::Team(crate::error_team::TeamError::Unavailable { reason }) =
            actual
        else {
            return Err("migration had another refusal".into());
        };
        assert_eq!(reason, failure);
    }
    Ok(())
}

#[test]
fn staged_holds_finish_durably_and_survive_restart() -> TestResult {
    let dir = tempfile::tempdir()?;
    let key = Arc::new(Ed25519Identity::load_or_generate(&dir.path().join("key"))?);
    let path = dir.path().join("teams");
    let mut store = TeamStore::open(&path, Arc::clone(&key))?;
    store.keep(created("team"))?;
    store.keep(added("team"))?;
    let lines = vec![held("hold", "team"), checked("check")];
    store.stage_migration(lines.clone())?;
    let expected = store.team("team").ok_or("no staged team")?.clone();
    store.finish_migration()?;
    assert_eq!(store.team("team"), Some(&expected));
    assert!(store.migration_checked());
    assert!(store.overlay.is_none());
    let store = TeamStore::open(&path, key)?;
    assert_eq!(store.team("team"), Some(&expected));
    assert!(store.migration_checked());
    for line in lines {
        assert_eq!(store.recorded(line.operation()), Some(line));
    }
    Ok(())
}

#[test]
fn borrowed_team_reads_match_pending_migration_without_copying_the_catalogue() -> TestResult {
    let dir = tempfile::tempdir()?;
    let key = Arc::new(Ed25519Identity::load_or_generate(&dir.path().join("key"))?);
    let mut store = TeamStore::open(&dir.path().join("teams"), key)?;
    store.held = populated()?;
    store.held.teams[0].lead = Some("member".to_owned());
    let lines = vec![held("borrow-hold", "team-0"), checked("borrow-check")];
    let mut expected = store.held.clone();
    for line in &lines {
        expected.hold(line.clone())?;
    }
    store.stage_migration(lines)?;
    crate::teams_state::reset_team_copies();
    let borrowed: Vec<_> = store.teams_iter().collect();
    assert_eq!(borrowed.len(), 64);
    for (actual, expected) in borrowed.iter().zip(&expected.teams) {
        assert_eq!(*actual, expected);
    }
    assert_eq!(
        crate::teams_state::team_copies(),
        0,
        "a borrowed read must not build the catalogue"
    );
    assert!(std::ptr::eq(
        borrowed[0],
        store.team("team-0").ok_or("no migrated team")?
    ));
    assert!(std::ptr::eq(
        borrowed[63],
        store.held.team("team-63").ok_or("no base team")?
    ));
    assert_eq!(borrowed, store.teams().iter().collect::<Vec<_>>());
    store.stage_migration(Vec::new())?;
    let borrowed: Vec<_> = store.teams_iter().collect();
    assert_eq!(borrowed, store.held.teams.iter().collect::<Vec<_>>());
    assert_eq!(borrowed.len(), 64);
    Ok(())
}
