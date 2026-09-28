//! The teams start from their signed snapshot and read only the leaves after
//! it; what they fold to is the same across a restart; the same act sent
//! again writes nothing; the same operation in other words, a change on a
//! team never created or retired, and a member added twice or removed when
//! absent are each refused by name.

use std::error::Error;
use std::path::Path;
use std::sync::Arc;

use lys_core::Ed25519Identity;
use lys_identity_server::error::ServerError;
use lys_identity_server::read_views::Login;
use lys_identity_server::teams_state::{Changed, Created, Line};
use lys_identity_server::teams_store::TeamStore;
use lys_log_store::Start;

type TestResult = Result<(), Box<dyn Error>>;

const OWNER: &str = "person-00000000000000000000000000000001";
const MEMBER: &str = "agent-00000000000000000000000000000002";

fn key(dir: &Path) -> Result<Arc<Ed25519Identity>, Box<dyn Error>> {
    Ok(Arc::new(Ed25519Identity::load_or_generate(
        &dir.join("teams.key"),
    )?))
}

fn by() -> Login {
    Login {
        provider: "https://issuer.example.test".to_owned(),
        subject: "ada-subject".to_owned(),
    }
}

fn created(id: &str, name: &str, at: u64) -> Line {
    Line::Created(Created {
        id: id.to_owned(),
        owner: OWNER.to_owned(),
        name: name.to_owned(),
        description: "builds the identity screens".to_owned(),
        by: by(),
        at,
    })
}

fn changed(operation: &str, team: &str, member: &str, at: u64) -> Changed {
    Changed {
        operation: operation.to_owned(),
        team: team.to_owned(),
        member: member.to_owned(),
        by: by(),
        at,
    }
}

#[test]
fn teams_fold_the_same_across_a_restart_from_the_snapshot() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("teams");
    let store = TeamStore::open(&path, key(dir.path())?)?;
    assert!(
        matches!(store.start(), Start::Rebuilt { replayed: 0, .. }),
        "{}",
        store.start()
    );
    drop(store);

    let mut store = TeamStore::open(&path, key(dir.path())?)?;
    store.keep(created("op-1", "screens", 5))?;
    store.keep(Line::Added(changed("op-2", "op-1", MEMBER, 6)))?;
    store.keep(created("op-3", "gates", 7))?;
    store.keep(Line::Retired(changed("op-4", "op-3", "", 8)))?;
    let before = store.teams().to_vec();
    drop(store);

    let store = TeamStore::open(&path, key(dir.path())?)?;
    assert_eq!(store.teams(), before.as_slice());
    let screens = store.team("op-1").ok_or("no team op-1")?;
    assert_eq!(screens.members, vec![MEMBER.to_owned()]);
    assert!(store.team("op-3").ok_or("no team op-3")?.retired.is_some());
    Ok(())
}

#[test]
fn the_same_act_again_writes_nothing_and_other_words_are_refused() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mut store = TeamStore::open(&dir.path().join("teams"), key(dir.path())?)?;
    store.keep(created("op-1", "screens", 5))?;
    let again = store.keep(created("op-1", "screens", 9))?;
    assert_eq!(again.created.at, 5);
    assert_eq!(store.teams().len(), 1);
    assert!(matches!(
        store.keep(created("op-1", "other", 9)),
        Err(ServerError::TeamReused { .. })
    ));
    Ok(())
}

#[test]
fn a_change_the_team_does_not_take_is_refused_by_name() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mut store = TeamStore::open(&dir.path().join("teams"), key(dir.path())?)?;
    assert!(matches!(
        store.keep(Line::Added(changed("op-9", "op-1", MEMBER, 5))),
        Err(ServerError::TeamUnknown)
    ));
    store.keep(created("op-1", "screens", 5))?;
    assert!(matches!(
        store.keep(Line::Removed(changed("op-2", "op-1", MEMBER, 6))),
        Err(ServerError::TeamMemberAbsent)
    ));
    store.keep(Line::Added(changed("op-3", "op-1", MEMBER, 7)))?;
    assert!(matches!(
        store.keep(Line::Added(changed("op-4", "op-1", MEMBER, 8))),
        Err(ServerError::TeamMemberHeld)
    ));
    store.keep(Line::Retired(changed("op-5", "op-1", "", 9)))?;
    assert!(matches!(
        store.keep(Line::Removed(changed("op-6", "op-1", MEMBER, 10))),
        Err(ServerError::TeamRetired { .. })
    ));
    Ok(())
}
