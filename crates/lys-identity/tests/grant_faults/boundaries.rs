//! Failure boundaries before durability and during permission projection.

use super::{
    EngineFault, FaultEngine, FaultWorld, FileLeafStore, GrantError, GrantId, IdentityId, Log,
    LogFault, Plan, RelationshipStore, Route, TestResult, past_its_source, set,
};
use lys_identity::grants::DelegateRequest;

pub(super) fn assert_log_boundary(
    world: &mut FaultWorld,
    plan: &Plan<LogFault>,
    request: &DelegateRequest,
    leg: usize,
    name: &str,
) -> TestResult {
    let (tom, lee) = (IdentityId::Person(world.tom), IdentityId::Person(world.lee));
    let control = |world: &mut FaultWorld| world.exercise(lee, "read", Route::Api).map(|_| ());
    match leg {
        0 => {
            set(plan, LogFault::BeforeLeaf);
            assert!(
                matches!(
                    world.delegate(request),
                    Err(GrantError::AppendRefused { .. })
                ),
                "{name}"
            );
            assert_eq!(world.events(), 3, "{name}: nothing recorded");
        }
        1 | 2 => {
            set(
                plan,
                if leg == 1 {
                    LogFault::LeafStoredWriteFailed
                } else {
                    LogFault::AfterLeaf
                },
            );
            let recorded = world.delegate(request)?;
            assert_eq!(
                recorded.index, 3,
                "{name}: the stored leaf is found committed once"
            );
            assert_eq!(
                world.delegate(request)?,
                recorded,
                "{name}: the retry answers the same"
            );
        }
        3 => {
            set(plan, LogFault::AfterLeafUnreadable);
            let unresolved = world.delegate(request);
            let Err(GrantError::OperationUnresolved { operation, .. }) = &unresolved else {
                return Err(format!("{name}: {unresolved:?}").into());
            };
            assert_eq!(operation, &request.operation.to_string());
            let before = world.events();
            let dependent = world.exercise(tom, "read", Route::Api);
            let unrelated = control(world);
            assert!(
                matches!(&dependent, Err(GrantError::LogUnavailable { .. })),
                "{name}: a reconciliation failure keeps its original name"
            );
            assert_eq!(unrelated.err(), dependent.err());
            assert_eq!(
                world.events(),
                before,
                "{name}: failed settlement records no use"
            );
            set(plan, LogFault::None);
            assert_eq!(
                world.delegate(request)?.index,
                3,
                "{name}: resolved as committed"
            );
        }
        _ => return Err("unknown log boundary".into()),
    }
    Ok(())
}

pub(super) fn assert_projection_boundary(
    world: &mut FaultWorld,
    engine: &FaultEngine,
    root: GrantId,
    request: &DelegateRequest,
    leg: usize,
    name: &str,
) -> TestResult {
    let (tom, lee) = (IdentityId::Person(world.tom), IdentityId::Person(world.lee));
    let control = |world: &mut FaultWorld| world.exercise(lee, "read", Route::Api).map(|_| ());
    match leg {
        4 => {
            set(&engine.plan, EngineFault::WriteRefused);
            assert!(
                matches!(
                    world.delegate(request),
                    Err(GrantError::ProjectionPending { index: 3, .. })
                ),
                "{name}"
            );
            set(&engine.plan, EngineFault::Held);
            assert!(
                matches!(
                    world.exercise(tom, "read", Route::Api),
                    Err(GrantError::ProjectionPending { index: 3, .. })
                ),
                "{name}: the pending read names its operation"
            );
            control(world)?;
            set(&engine.plan, EngineFault::None);
            assert_eq!(
                world.delegate(request)?.index,
                3,
                "{name}: the retry answers once projected"
            );
        }
        5 => {
            set(&engine.plan, EngineFault::StoredReportedFailed);
            assert_eq!(
                world.delegate(request)?.index,
                3,
                "{name}: the stored write is found"
            );
        }
        6 => {
            set(&engine.plan, EngineFault::Held);
            assert!(
                matches!(
                    world.delegate(request),
                    Err(GrantError::ProjectionPending { .. })
                ),
                "{name}"
            );
            set(&engine.plan, EngineFault::None);
            world.reopen(engine.clone())?;
            assert_eq!(
                world.delegate(request)?.index,
                3,
                "{name}: the restart projects what was committed"
            );
        }
        7 => {
            let bytes = past_its_source(world, root)?;
            let mut log = Log::open(FileLeafStore::open(&world.dir.path().join("grants"))?)?;
            log.append(&bytes)?;
            world.reopen(engine.clone())?;
            assert!(
                matches!(
                    world.grants.book().refused().get(&2),
                    Some((_, GrantError::ExpiryBeyondSource { .. }))
                ),
                "{name}: refused by name"
            );
            assert_eq!(
                world.grants.relationships().read()?.len(),
                6,
                "{name}: no relationship is written for it"
            );
            assert!(matches!(
                world.exercise(tom, "read", Route::Api),
                Err(GrantError::NotHeld { .. })
            ));
        }
        _ => return Err("unknown projection boundary".into()),
    }
    Ok(())
}
