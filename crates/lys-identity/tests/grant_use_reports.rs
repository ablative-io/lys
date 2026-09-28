//! Observed grant use when a report is missing (`GRANT_LAST_USED`): a
//! permitted exercise whose use event cannot be recorded is named unreported,
//! so a missing report is never read as a count of zero, and the recorded
//! count and latest use survive a reopen.

mod support;

use std::error::Error;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use lys_identity::IdentityId;
use lys_identity::grants::{
    GrantError, GrantId, LastUse, MemoryRelationships, PassOn, RecipientKind, Route, Unreported,
    Usage,
};
use lys_identity::log::Reopen;
use lys_log_store::{FileLeafStore, LeafStore, PinnedRoot, StoreError, StoreResult};
use support::{T0, World, actions, alpha, pass};

type TestResult = Result<(), Box<dyn Error>>;

const BOTH: [RecipientKind; 2] = [RecipientKind::Person, RecipientKind::Agent];

/// A file store whose leaf writes fail while its switch is on.
struct Refusing {
    inner: FileLeafStore,
    refuse: Arc<AtomicBool>,
}

impl LeafStore for Refusing {
    fn origin(&self) -> &str {
        self.inner.origin()
    }
    fn extent(&self) -> u64 {
        self.inner.extent()
    }
    fn leaf(&self, index: u64) -> StoreResult<Option<Vec<u8>>> {
        self.inner.leaf(index)
    }
    fn put_leaf(&mut self, index: u64, bytes: &[u8]) -> StoreResult<()> {
        if self.refuse.load(Ordering::SeqCst) {
            return Err(StoreError::Io {
                context: "leaf write".to_owned(),
                source: std::io::Error::other("injected"),
            });
        }
        self.inner.put_leaf(index, bytes)
    }
    fn pinned(&self) -> PinnedRoot {
        self.inner.pinned()
    }
    fn pin(&mut self, pin: PinnedRoot) -> StoreResult<()> {
        self.inner.pin(pin)
    }
    fn snapshot(&self) -> StoreResult<Option<Vec<u8>>> {
        self.inner.snapshot()
    }
    fn put_snapshot(&mut self, bytes: &[u8]) -> StoreResult<()> {
        self.inner.put_snapshot(bytes)
    }
}

type RefusingWorld = World<Refusing, MemoryRelationships>;

fn world(refuse: &Arc<AtomicBool>) -> Result<RefusingWorld, Box<dyn Error>> {
    let refuse = Arc::clone(refuse);
    World::with(
        Box::new(move |path: &Path| -> Reopen<Refusing> {
            let (path, refuse) = (path.to_owned(), Arc::clone(&refuse));
            Box::new(move || {
                Ok(Refusing {
                    inner: FileLeafStore::open(&path)?,
                    refuse: Arc::clone(&refuse),
                })
            })
        }),
        MemoryRelationships::default(),
    )
}

fn usage<S: LeafStore>(
    world: &World<S, MemoryRelationships>,
    grant: GrantId,
) -> Result<Usage, Box<dyn Error>> {
    Ok(world.grants.usage(grant).ok_or("the grant is not held")?)
}

/// Dana's root and the use-only grant it lends to Tom.
fn lent<S: LeafStore>(
    world: &mut World<S, MemoryRelationships>,
) -> Result<(GrantId, GrantId), Box<dyn Error>> {
    let dana = IdentityId::Person(world.dana);
    let root = world.root(world.dana, "kite", pass(&["read"], &BOTH)?, None)?;
    let request = world.request(
        dana,
        root,
        IdentityId::Person(world.tom),
        "tern",
        PassOn::UseOnly,
        None,
    )?;
    Ok((root, world.delegate(&request)?.event.grant()))
}

#[test]
fn grant_last_used_a_missing_report_is_not_a_zero_count() -> TestResult {
    let refuse = Arc::new(AtomicBool::new(false));
    let mut world = world(&refuse)?;
    let tom = IdentityId::Person(world.tom);
    let (root, grant) = lent(&mut world)?;
    let untouched = Usage {
        last: LastUse::NotSeen,
        recorded: 0,
        unreported: None,
    };
    assert_eq!(
        usage(&world, grant)?,
        untouched,
        "a zero count, fully reported"
    );

    refuse.store(true, Ordering::SeqCst);
    let before = world.events();
    let permit = world.exercise(tom, "read", Route::Api)?;
    let refused = permit.use_event.ok_or("a check reports its use")?;
    assert!(
        matches!(refused, Err(GrantError::AppendRefused { .. })),
        "the use event is refused by name: {refused:?}"
    );
    assert_eq!(world.events(), before, "nothing was recorded");
    let missing = usage(&world, grant)?;
    assert_eq!(missing.last, LastUse::NotSeen);
    assert_eq!(missing.recorded, 0);
    let unreported = missing.unreported.ok_or("the missing report is named")?;
    assert_eq!(
        (unreported.count, unreported.at, unreported.route),
        (1, world.now, Route::Api)
    );
    assert!(
        unreported.reason.contains("injected"),
        "{}",
        unreported.reason
    );
    assert_eq!(
        usage(&world, root)?,
        untouched,
        "the source's own use is not missing"
    );

    refuse.store(false, Ordering::SeqCst);
    world.now += 60;
    let index = world
        .exercise(tom, "read", Route::Tool)?
        .use_event
        .ok_or("a check reports its use")??;
    let after = usage(&world, grant)?;
    assert_eq!(
        after,
        Usage {
            last: LastUse::Seen {
                at: world.now,
                route: Route::Tool,
                index
            },
            recorded: 1,
            unreported: Some(Unreported {
                count: 1,
                at: world.now - 60,
                route: Route::Api,
                reason: unreported.reason,
            }),
        },
        "a later recorded use does not make the count whole"
    );

    world.reopen(MemoryRelationships::default())?;
    let reopened = usage(&world, grant)?;
    assert_eq!(
        (reopened.last, reopened.recorded),
        (after.last, after.recorded),
        "the recorded attribution and count survive a reopen"
    );
    assert_eq!(
        reopened.unreported, None,
        "a missing report is in no log, so a reopen cannot know it"
    );
    Ok(())
}

#[test]
fn grant_last_used_the_recorded_count_is_every_use_event_across_a_reopen() -> TestResult {
    let mut world = World::new()?;
    let tom = IdentityId::Person(world.tom);
    let (_, grant) = lent(&mut world)?;
    let held = world
        .grants
        .book()
        .grant(grant)
        .ok_or("the grant is not held")?;
    assert_eq!(
        (
            held.resource(),
            held.actions(),
            held.parts().window.starts_at()
        ),
        (&alpha()?, &actions(&["read"])?, T0),
        "the grant exercised is the one lent"
    );
    let routes = [Route::Api, Route::Tool, Route::Browser];
    let mut last = None;
    for route in routes {
        world.now += 1;
        last = Some(
            world
                .exercise(tom, "read", route)?
                .use_event
                .ok_or("a check reports its use")??,
        );
    }
    let expected = Usage {
        last: LastUse::Seen {
            at: world.now,
            route: Route::Browser,
            index: last.ok_or("no use was recorded")?,
        },
        recorded: 3,
        unreported: None,
    };
    assert_eq!(usage(&world, grant)?, expected);
    world.reopen(MemoryRelationships::default())?;
    assert_eq!(usage(&world, grant)?, expected, "reopen from the snapshot");
    Ok(())
}
