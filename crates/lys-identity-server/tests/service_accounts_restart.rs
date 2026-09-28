//! The service accounts start from their signed snapshot and read only the
//! leaves after it; what they fold to is the same across a restart; the same
//! act sent again writes nothing, and the same operation in other words is
//! refused by name.

use std::error::Error;
use std::path::Path;
use std::sync::Arc;

use lys_core::Ed25519Identity;
use lys_identity_server::error::ServerError;
use lys_identity_server::read_views::Login;
use lys_identity_server::service_accounts_state::{Account, Created, Retired};
use lys_identity_server::service_accounts_store::ServiceAccountStore;
use lys_log_store::Start;

type TestResult = Result<(), Box<dyn Error>>;

const OWNER: &str = "person-00000000000000000000000000000001";

fn key(dir: &Path) -> Result<Arc<Ed25519Identity>, Box<dyn Error>> {
    Ok(Arc::new(Ed25519Identity::load_or_generate(
        &dir.join("service-accounts.key"),
    )?))
}

fn by() -> Login {
    Login {
        provider: "https://issuer.example.test".to_owned(),
        subject: "ada-subject".to_owned(),
    }
}

fn created(id: &str, name: &str, at: u64) -> Created {
    Created {
        id: id.to_owned(),
        owner: OWNER.to_owned(),
        name: name.to_owned(),
        description: "deploys the documentation site".to_owned(),
        by: by(),
        at,
    }
}

fn retired(operation: &str, account: &str, at: u64) -> Retired {
    Retired {
        operation: operation.to_owned(),
        account: account.to_owned(),
        by: by(),
        at,
    }
}

#[test]
fn accounts_fold_the_same_across_a_restart_from_the_snapshot() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("service-accounts");
    let store = ServiceAccountStore::open(&path, key(dir.path())?)?;
    assert!(
        matches!(store.start(), Start::Rebuilt { replayed: 0, .. }),
        "{}",
        store.start()
    );
    drop(store);

    let mut store = ServiceAccountStore::open(&path, key(dir.path())?)?;
    assert_eq!(
        store.start(),
        &Start::Resumed {
            size: 0,
            replayed: 0
        }
    );
    store.create(created("op-1", "docs deploy", 5))?;
    store.create(created("op-2", "backup reader", 6))?;
    store.retire(retired("op-3", "op-2", 7))?;
    let before = store.accounts().to_vec();
    drop(store);

    let store = ServiceAccountStore::open(&path, key(dir.path())?)?;
    assert_eq!(
        store.start(),
        &Start::Resumed {
            size: 0,
            replayed: 3
        },
        "a start reads the snapshot and only the leaves after it"
    );
    assert_eq!(store.accounts(), before.as_slice());
    assert_eq!(store.snapshot_failure(), None);
    assert_eq!(store.account("op-1").map(Account::is_retired), Some(false));
    assert_eq!(store.account("op-2").map(Account::is_retired), Some(true));
    Ok(())
}

#[test]
fn a_snapshot_from_another_key_is_refused_and_every_leaf_is_read() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("service-accounts");
    let mut store = ServiceAccountStore::open(&path, key(dir.path())?)?;
    store.create(created("op-1", "docs deploy", 5))?;
    drop(store);

    let other = tempfile::tempdir()?;
    let store = ServiceAccountStore::open(&path, key(other.path())?)?;
    assert!(
        matches!(store.start(), Start::Rebuilt { replayed: 1, .. }),
        "{}",
        store.start()
    );
    assert_eq!(store.accounts().len(), 1);
    Ok(())
}

#[test]
fn the_same_act_again_writes_nothing_and_other_words_are_refused() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("service-accounts");
    let mut store = ServiceAccountStore::open(&path, key(dir.path())?)?;
    let first = store.create(created("op-1", "docs deploy", 5))?;
    let again = store.create(created("op-1", "docs deploy", 9))?;
    assert_eq!(again, first, "the same creation answers what was recorded");
    assert_eq!(store.accounts().len(), 1);

    let other_words = store.create(created("op-1", "another name", 9));
    assert!(matches!(
        other_words,
        Err(ServerError::ServiceAccountReused { .. })
    ));

    let unknown = store.retire(retired("op-2", "op-9", 10));
    assert!(matches!(unknown, Err(ServerError::ServiceAccountUnknown)));
    let as_retirement = store.retire(retired("op-1", "op-1", 10));
    assert!(
        matches!(as_retirement, Err(ServerError::ServiceAccountReused { .. })),
        "a creation's operation id never names a retirement"
    );

    let ended = store.retire(retired("op-3", "op-1", 10))?;
    assert!(ended.is_retired());
    let ended_again = store.retire(retired("op-3", "op-1", 11))?;
    assert_eq!(ended_again, ended, "the same retirement is kept once");
    let twice = store.retire(retired("op-4", "op-1", 12));
    assert!(matches!(
        twice,
        Err(ServerError::ServiceAccountRetired { .. })
    ));
    let reused = store.create(created("op-3", "docs deploy", 12));
    assert!(
        matches!(reused, Err(ServerError::ServiceAccountReused { .. })),
        "a retirement's operation id never names a creation"
    );
    drop(store);

    let store = ServiceAccountStore::open(&path, key(dir.path())?)?;
    assert_eq!(
        store.start(),
        &Start::Resumed {
            size: 0,
            replayed: 2
        },
        "only the creation and the one retirement were written"
    );
    Ok(())
}
