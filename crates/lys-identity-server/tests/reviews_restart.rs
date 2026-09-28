//! The review decisions start from their signed snapshot and read only the
//! leaves after it; what they fold to is the same across a restart; a
//! decision asked again in the same words is recorded once, and the same
//! operation in other words is refused by name.

use std::error::Error;
use std::path::Path;
use std::sync::Arc;

use lys_core::Ed25519Identity;
use lys_identity_server::error::ServerError;
use lys_identity_server::reviews_state::Kept;
use lys_identity_server::reviews_store::ReviewStore;
use lys_log_store::Start;

type TestResult = Result<(), Box<dyn Error>>;

fn key(dir: &Path) -> Result<Arc<Ed25519Identity>, Box<dyn Error>> {
    Ok(Arc::new(Ed25519Identity::load_or_generate(
        &dir.join("reviews.key"),
    )?))
}

fn kept(operation: &str, grant: &str, note: &str, at: u64) -> Kept {
    Kept {
        grant: grant.to_owned(),
        kept_by: "person-a".to_owned(),
        note: note.to_owned(),
        operation: operation.to_owned(),
        at,
        revision: 3,
    }
}

#[test]
fn decisions_fold_the_same_across_a_restart_from_the_snapshot() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("reviews");
    let store = ReviewStore::open(&path, key(dir.path())?)?;
    assert!(
        matches!(store.start(), Start::Rebuilt { replayed: 0, .. }),
        "{}",
        store.start()
    );
    drop(store);

    let mut store = ReviewStore::open(&path, key(dir.path())?)?;
    assert_eq!(
        store.start(),
        &Start::Resumed {
            size: 0,
            replayed: 0
        }
    );
    store.keep(kept("op-1", "grant-1", "still needed", 5))?;
    store.keep(kept("op-2", "grant-2", "reads the docs", 6))?;
    store.keep(kept("op-3", "grant-1", "needed again", 7))?;
    let before = store.decisions().to_vec();
    drop(store);

    let store = ReviewStore::open(&path, key(dir.path())?)?;
    assert_eq!(
        store.start(),
        &Start::Resumed {
            size: 0,
            replayed: 3
        },
        "a start reads the snapshot and only the leaves after it"
    );
    assert_eq!(store.decisions(), before.as_slice());
    assert_eq!(store.snapshot_failure(), None);
    assert_eq!(
        store.last_for("grant-1").map(|kept| kept.note.as_str()),
        Some("needed again")
    );
    assert_eq!(store.last_for("grant-3"), None);
    Ok(())
}

#[test]
fn a_snapshot_from_another_key_is_refused_and_every_leaf_is_read() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("reviews");
    let mut store = ReviewStore::open(&path, key(dir.path())?)?;
    store.keep(kept("op-1", "grant-1", "still needed", 5))?;
    drop(store);

    let other = tempfile::tempdir()?;
    let store = ReviewStore::open(&path, key(other.path())?)?;
    assert!(
        matches!(store.start(), Start::Rebuilt { replayed: 1, .. }),
        "{}",
        store.start()
    );
    assert_eq!(store.decisions().len(), 1);
    Ok(())
}

#[test]
fn the_same_decision_is_recorded_once_and_other_words_are_refused() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("reviews");
    let mut store = ReviewStore::open(&path, key(dir.path())?)?;
    let first = store.keep(kept("op-1", "grant-1", "still needed", 5))?;
    let again = store.keep(kept("op-1", "grant-1", "still needed", 9))?;
    assert_eq!(again, first, "asked again it answers what was recorded");
    assert_eq!(store.decisions().len(), 1);

    let other_words = store.keep(kept("op-1", "grant-1", "other words", 9));
    assert!(matches!(other_words, Err(ServerError::ReviewReused { .. })));
    let other_grant = store.keep(kept("op-1", "grant-2", "still needed", 9));
    assert!(matches!(other_grant, Err(ServerError::ReviewReused { .. })));
    assert_eq!(store.decisions().len(), 1, "nothing refused is recorded");
    drop(store);

    let store = ReviewStore::open(&path, key(dir.path())?)?;
    assert_eq!(store.decisions(), std::slice::from_ref(&first));
    Ok(())
}
