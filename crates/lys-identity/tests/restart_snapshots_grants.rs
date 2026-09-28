//! A restart of the grants loads the snapshot of the book and receipts and
//! folds only the leaves after it, answering the same book.

#![cfg(unix)]

mod support;

use std::error::Error;
use std::num::NonZeroU64;

use lys_core::Ed25519Identity;
use lys_identity::IdentityId;
use lys_identity::OperationId;
use lys_identity::grants::{
    Grants, MemoryRelationships, Model, PassOn, RecipientKind, Relation, RevokeRequest, Route,
};
use lys_identity::log::Reopen;
use lys_log_store::{FileLeafStore, Start};
use support::{T0, World, actions, alpha, pass};

type TestResult = Result<(), Box<dyn Error>>;

fn model() -> Result<Model, Box<dyn Error>> {
    Ok(Model::new(
        1,
        [
            (
                Relation::new("kite")?,
                actions(&["delete", "read", "write"])?,
            ),
            (Relation::new("heron")?, actions(&["read", "write"])?),
            (Relation::new("tern")?, actions(&["read"])?),
        ],
    )?)
}

/// The world's grants, opened again over the same log, snapshotting every entry.
fn open(world: &World) -> Result<Grants<FileLeafStore, MemoryRelationships>, Box<dyn Error>> {
    let path = world.dir.path().join("grants");
    let reopen: Reopen<FileLeafStore> = Box::new(move || FileLeafStore::open(&path));
    let key = Ed25519Identity::load(&world.dir.path().join("service.key"))?;
    let every = NonZeroU64::new(1).ok_or("one is not zero")?;
    Ok(Grants::open_with(
        reopen,
        key,
        MemoryRelationships::default(),
        model()?,
        world.admin,
        every,
    )?)
}

#[test]
fn a_restart_of_the_grants_folds_only_the_leaves_after_the_snapshot() -> TestResult {
    let mut world = World::new()?;
    let (dana, tom) = (
        IdentityId::Person(world.dana),
        IdentityId::Person(world.tom),
    );
    let root = world.root(
        world.dana,
        "kite",
        pass(&["read"], &[RecipientKind::Person, RecipientKind::Agent])?,
        None,
    )?;
    let lent = world.request(dana, root, tom, "tern", PassOn::UseOnly, None)?;
    let lent = world.delegate(&lent)?.event.grant();
    world.exercise(tom, "read", Route::Tool)?;
    let revoke = RevokeRequest {
        operation: OperationId::generate()?,
        caller: IdentityId::Person(world.admin),
        route: Route::Api,
        grant: lent,
        reason: "the root authority withdrew it".to_owned(),
    };
    world.grants.revoke(&revoke, T0 + 20)?;
    let size = world.events();

    let first = open(&world)?;
    assert!(
        matches!(first.ledger().start(), Start::Resumed { .. }),
        "{}",
        first.ledger().start()
    );
    drop(first);

    let restarted = open(&world)?;
    assert_eq!(
        restarted.ledger().start(),
        &Start::Resumed { size, replayed: 0 },
        "the first reopen wrote a snapshot at the whole log"
    );
    assert_eq!(restarted.book(), world.grants.book());
    assert_eq!(restarted.receipts(), world.grants.receipts());
    assert_eq!(restarted.revision(), size);
    assert_eq!(restarted.ledger().snapshot_failure(), None);
    assert_eq!(restarted.book().on_resource(&alpha()?).count(), 2);
    drop(restarted);

    world.reopen(MemoryRelationships::default())?;
    assert_eq!(
        world.grants.ledger().start(),
        &Start::Resumed { size, replayed: 0 },
        "the world's own reopen starts from the same snapshot"
    );
    assert_eq!(world.events(), size);
    Ok(())
}
