//! Restoring the membership indexes (ACCESS-006 R5) against the grant log's
//! recovery as it stands: a damaged or foreign checkpoint is refused by its
//! name and never read, the live indexes come back only from the verified
//! log, the same as the book that applied every event; a damaged tail leaf
//! refuses the open by name while the directory beside it still answers;
//! and a revocation interrupted at each boundary of its append resumes to
//! one authoritative generation, never a mix of allow and deny.

#![cfg(unix)]

mod support;

use std::error::Error;
use std::num::NonZeroU64;
use std::path::Path;
use std::sync::{Arc, Mutex};

use lys_core::Ed25519Identity;
use lys_identity::grants::{
    GrantBook, GrantError, GrantId, Grants, MemoryRelationships, Model, PassOn, RecipientKind,
    Relation, RevokeRequest, Route,
};
use lys_identity::log::Reopen;
use lys_identity::{IdentityId, OperationId};
use lys_log_store::{FileLeafStore, LeafStore, PinnedRoot, Start, StoreError, StoreResult};
use support::{T0, World, actions, pass};

type TestResult = Result<(), Box<dyn Error>>;

const BOTH: [RecipientKind; 2] = [RecipientKind::Person, RecipientKind::Agent];

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

/// `world`'s grants opened again over their log, a snapshot owed at every
/// entry, so the first open writes one at the whole log.
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

/// Dana's root passed on to Tom, Lee's own root, and Tom's grant revoked:
/// answered with the revoked grant.
fn history(world: &mut World) -> Result<GrantId, Box<dyn Error>> {
    let dana = IdentityId::Person(world.dana);
    let root = world.root(world.dana, "kite", pass(&["read"], &BOTH)?, None)?;
    world.root(world.lee, "tern", PassOn::UseOnly, None)?;
    let lent = world.request(
        dana,
        root,
        IdentityId::Person(world.tom),
        "tern",
        PassOn::UseOnly,
        None,
    )?;
    let lent = world.delegate(&lent)?.event.grant();
    revoke(world, lent)?;
    Ok(lent)
}

fn revoke(world: &mut World<impl LeafStore>, grant: GrantId) -> Result<(), GrantError> {
    let request = RevokeRequest {
        operation: OperationId::generate().map_err(|error| GrantError::LogUnavailable {
            reason: error.to_string(),
        })?,
        caller: IdentityId::Person(world.admin),
        route: Route::Api,
        grant,
        reason: "left the ward".to_owned(),
    };
    world.grants.revoke(&request, T0 + 20).map(drop)
}

fn live(book: &GrantBook, holder: IdentityId) -> Vec<GrantId> {
    book.live_held_by(holder)
        .map(|record| record.grant().id())
        .collect()
}

/// Open once to write the snapshot at the whole log, and check a second
/// open resumes from it.
fn snapshotted(world: &World) -> TestResult {
    drop(open(world)?);
    let resumed = open(world)?;
    assert!(
        matches!(resumed.ledger().start(), Start::Resumed { replayed: 0, .. }),
        "{}",
        resumed.ledger().start()
    );
    Ok(())
}

/// The refused checkpoint is named, the indexes are the applied book's.
fn refused_and_rebuilt(world: &World) -> TestResult {
    let reopened = open(world)?;
    let start = reopened.ledger().start();
    let refusal = start
        .refusal()
        .ok_or_else(|| format!("the checkpoint was read: {start}"))?;
    assert!(refusal.to_string().starts_with("Snapshot"), "{refusal}");
    assert_eq!(
        reopened.book(),
        world.grants.book(),
        "the verified log's book"
    );
    assert_eq!(
        reopened.book().live_entries(),
        world.grants.book().live_entries()
    );
    let tom = IdentityId::Person(world.tom);
    assert!(
        live(reopened.book(), tom).is_empty(),
        "the revoked chain stays pruned"
    );
    Ok(())
}

#[test]
fn a_damaged_checkpoint_is_named_and_never_read() -> TestResult {
    let mut world = World::new()?;
    history(&mut world)?;
    snapshotted(&world)?;
    let path = world.dir.path().join("grants").join("snapshot.bin");
    let mut bytes = std::fs::read(&path)?;
    let last = bytes.len().checked_sub(1).ok_or("the snapshot is empty")?;
    bytes[last] ^= 0xff;
    std::fs::write(&path, bytes)?;
    refused_and_rebuilt(&world)
}

#[test]
fn a_foreign_checkpoint_is_named_and_never_read() -> TestResult {
    let mut world = World::new()?;
    history(&mut world)?;
    let mut foreign = World::new()?;
    foreign.root(foreign.lee, "tern", PassOn::UseOnly, None)?;
    snapshotted(&foreign)?;
    snapshotted(&world)?;
    std::fs::copy(
        foreign.dir.path().join("grants").join("snapshot.bin"),
        world.dir.path().join("grants").join("snapshot.bin"),
    )?;
    refused_and_rebuilt(&world)
}

#[test]
fn a_damaged_tail_leaf_refuses_the_open_and_the_directory_still_answers() -> TestResult {
    let mut world = World::new()?;
    history(&mut world)?;
    let segment = world
        .dir
        .path()
        .join("grants")
        .join("leaves")
        .join("segments")
        .join(format!("{:020}", 0));
    let mut bytes = std::fs::read(&segment)?;
    // Inside the last record's leaf, before its flag, pin and checksum.
    let at = bytes
        .len()
        .checked_sub(64)
        .ok_or("the segment is too short")?;
    bytes[at] ^= 0xff;
    std::fs::write(&segment, bytes)?;
    let refused = world.reopen(MemoryRelationships::default());
    assert!(
        refused.is_err(),
        "a damaged tail is never read as authority"
    );
    assert!(
        !refused
            .err()
            .map(|error| error.to_string())
            .unwrap_or_default()
            .is_empty(),
        "the refusal is named"
    );
    assert!(
        world.directory.projection().is_ok(),
        "the directory is another authority domain and still answers"
    );
    Ok(())
}

/// Where the grant log's next append fails, if anywhere.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Fault {
    None,
    BeforeLeaf,
    AfterLeaf,
    AfterLeafUnreadable,
}

type Plan = Arc<Mutex<Fault>>;

fn get(plan: &Plan) -> Result<Fault, StoreError> {
    plan.lock()
        .map(|held| *held)
        .map_err(|error| injected(&error.to_string()))
}

fn set(plan: &Plan, next: Fault) -> TestResult {
    *plan.lock().map_err(|error| error.to_string())? = next;
    Ok(())
}

fn injected(context: &str) -> StoreError {
    StoreError::Io {
        context: context.to_owned(),
        source: std::io::Error::other("injected"),
    }
}

/// A file store whose next append fails where its plan says.
struct FaultStore {
    inner: FileLeafStore,
    plan: Plan,
}

impl LeafStore for FaultStore {
    fn origin(&self) -> &str {
        self.inner.origin()
    }
    fn extent(&self) -> u64 {
        self.inner.extent()
    }
    fn leaf(&self, index: u64) -> StoreResult<Option<Vec<u8>>> {
        self.inner.leaf(index)
    }
    fn append(&mut self, index: u64, leaves: &[&[u8]], pin: PinnedRoot) -> StoreResult<()> {
        match get(&self.plan)? {
            Fault::None => self.inner.append(index, leaves, pin),
            Fault::BeforeLeaf => {
                *self
                    .plan
                    .lock()
                    .map_err(|error| injected(&error.to_string()))? = Fault::None;
                Err(injected("leaf write"))
            }
            Fault::AfterLeaf => {
                *self
                    .plan
                    .lock()
                    .map_err(|error| injected(&error.to_string()))? = Fault::None;
                self.inner.append(index, leaves, pin)?;
                Err(injected("pin write"))
            }
            Fault::AfterLeafUnreadable => {
                self.inner.append(index, leaves, pin)?;
                Err(injected("pin write"))
            }
        }
    }
    fn pinned(&self) -> PinnedRoot {
        self.inner.pinned()
    }
    fn pin(&mut self, pin: PinnedRoot) -> StoreResult<()> {
        match get(&self.plan)? {
            Fault::AfterLeafUnreadable => Err(injected("pin write")),
            Fault::None | Fault::BeforeLeaf | Fault::AfterLeaf => self.inner.pin(pin),
        }
    }
    fn snapshot(&self) -> StoreResult<Option<Vec<u8>>> {
        self.inner.snapshot()
    }
    fn put_snapshot(&mut self, bytes: &[u8]) -> StoreResult<()> {
        self.inner.put_snapshot(bytes)
    }
}

fn faulty(plan: &Plan) -> Result<World<FaultStore>, Box<dyn Error>> {
    let plan = Arc::clone(plan);
    World::with(
        Box::new(move |path: &Path| -> Reopen<FaultStore> {
            let (path, plan) = (path.to_owned(), Arc::clone(&plan));
            Box::new(move || {
                if get(&plan)? == Fault::AfterLeafUnreadable {
                    return Err(injected("reopen"));
                }
                Ok(FaultStore {
                    inner: FileLeafStore::open(&path)?,
                    plan: Arc::clone(&plan),
                })
            })
        }),
        MemoryRelationships::default(),
    )
}

/// Restart and refuse unless the restarted book, live indexes and all, is
/// the one answered before it.
fn one_generation(world: &mut World<FaultStore>) -> TestResult {
    let answered = world.grants.book().clone();
    world.reopen(MemoryRelationships::default())?;
    assert_eq!(
        world.grants.book(),
        &answered,
        "the restart is the answered generation"
    );
    Ok(())
}

#[test]
fn a_revocation_interrupted_at_each_boundary_resumes_to_one_generation() -> TestResult {
    for fault in [
        Fault::BeforeLeaf,
        Fault::AfterLeaf,
        Fault::AfterLeafUnreadable,
    ] {
        let plan = Arc::new(Mutex::new(Fault::None));
        let mut world = faulty(&plan)?;
        let (dana, tom) = (
            IdentityId::Person(world.dana),
            IdentityId::Person(world.tom),
        );
        let root = world.root(world.dana, "kite", pass(&["read"], &BOTH)?, None)?;
        world.root(world.lee, "tern", PassOn::UseOnly, None)?;
        let lent = world.request(dana, root, tom, "tern", PassOn::UseOnly, None)?;
        let lent = world.delegate(&lent)?.event.grant();
        let entries = world.grants.book().live_entries();
        set(&plan, fault)?;
        let revoked = revoke(&mut world, lent);
        match fault {
            Fault::BeforeLeaf => {
                assert!(
                    matches!(revoked, Err(GrantError::AppendRefused { .. })),
                    "{fault:?}: {revoked:?}"
                );
                assert_eq!(
                    live(world.grants.book(), tom),
                    [lent],
                    "{fault:?}: still live"
                );
                assert_eq!(world.grants.book().live_entries(), entries);
            }
            Fault::AfterLeaf => {
                revoked?;
                assert!(
                    live(world.grants.book(), tom).is_empty(),
                    "{fault:?}: pruned once"
                );
                assert_eq!(
                    world.grants.book().live_entries(),
                    (entries.0 - 1, entries.1 - 1)
                );
            }
            Fault::AfterLeafUnreadable => {
                assert!(
                    matches!(revoked, Err(GrantError::OperationUnresolved { .. })),
                    "{fault:?}: {revoked:?}"
                );
                // Nothing is answered from a half-known generation: Tom's
                // read is refused by the log's name, neither allowed nor
                // denied as if the revocation were settled.
                assert!(
                    matches!(
                        world.exercise(tom, "read", Route::Api),
                        Err(GrantError::LogUnavailable { .. })
                    ),
                    "{fault:?}"
                );
                set(&plan, Fault::None)?;
                let settled = revoke(&mut world, lent);
                assert!(
                    settled.is_ok() || matches!(settled, Err(GrantError::AlreadyRevoked { .. })),
                    "{fault:?}: the stored revocation is found: {settled:?}"
                );
                assert!(
                    live(world.grants.book(), tom).is_empty(),
                    "{fault:?}: pruned"
                );
                assert_eq!(
                    world.grants.book().live_entries(),
                    (entries.0 - 1, entries.1 - 1)
                );
            }
            Fault::None => {}
        }
        one_generation(&mut world)?;
    }
    Ok(())
}
