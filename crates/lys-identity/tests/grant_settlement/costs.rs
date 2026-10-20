#[derive(Debug, PartialEq, Eq)]
struct OperationCounts {
    revisions: u64,
    reads: u64,
    writes: u64,
    appends: u64,
    pins: u64,
    snapshots: u64,
}

#[derive(Debug, Default, PartialEq, Eq)]
struct LogCounts {
    leaves: u64,
    snapshots: u64,
    reopens: u64,
    appends: u64,
    pins: u64,
    snapshot_writes: u64,
}

impl LogCounts {
    fn since(self, before: &Self) -> Result<Self, Box<dyn Error>> {
        let delta = |after: u64, before: u64| {
            after
                .checked_sub(before)
                .ok_or_else(|| "fixture log counter decreased".to_owned())
        };
        Ok(Self {
            leaves: delta(self.leaves, before.leaves)?,
            snapshots: delta(self.snapshots, before.snapshots)?,
            reopens: delta(self.reopens, before.reopens)?,
            appends: delta(self.appends, before.appends)?,
            pins: delta(self.pins, before.pins)?,
            snapshot_writes: delta(self.snapshot_writes, before.snapshot_writes)?,
        })
    }
}

impl Faults {
    fn log_counts(&self) -> LogCounts {
        LogCounts {
            leaves: self.leaves.load(Ordering::Relaxed),
            snapshots: self.snapshot_reads.load(Ordering::Relaxed),
            reopens: self.reopens.load(Ordering::Relaxed),
            appends: self.appends.load(Ordering::Relaxed),
            pins: self.pins.load(Ordering::Relaxed),
            snapshot_writes: self.snapshots.load(Ordering::Relaxed),
        }
    }

    fn counts(&self) -> OperationCounts {
        OperationCounts {
            revisions: self.revisions.load(Ordering::Relaxed),
            reads: self.reads.load(Ordering::Relaxed),
            writes: self.writes.load(Ordering::Relaxed),
            appends: self.appends.load(Ordering::Relaxed),
            pins: self.pins.load(Ordering::Relaxed),
            snapshots: self.snapshots.load(Ordering::Relaxed),
        }
    }
}

impl OperationCounts {
    fn since(self, before: &Self) -> Result<Self, Box<dyn Error>> {
        let delta = |after: u64, before: u64| {
            after
                .checked_sub(before)
                .ok_or_else(|| "fixture operation counter decreased".to_owned())
        };
        Ok(Self {
            revisions: delta(self.revisions, before.revisions)?,
            reads: delta(self.reads, before.reads)?,
            writes: delta(self.writes, before.writes)?,
            appends: delta(self.appends, before.appends)?,
            pins: delta(self.pins, before.pins)?,
            snapshots: delta(self.snapshots, before.snapshots)?,
        })
    }
}

#[test]
fn frame_metadata_shares_one_failure_and_keeps_healthy_and_degraded_work_bounded() -> TestResult {
    let engine = Engine::default();
    let mut world = world(&engine)?;
    let reading = (|| -> Result<_, Box<dyn Error>> {
        world.root(world.lee, "tern", PassOn::UseOnly, None)?;
        let request = ExerciseRequest {
            caller: IdentityId::Person(world.lee),
            route: Route::Api,
            resource: alpha()?,
            action: lys_identity::grants::Action::new("read")?,
        };
        let before = engine.faults.counts();
        let healthy = world.grants.frame(world.directory.projection()?, None)?;
        let permitted = world.grants.explain_in(&healthy, &request, world.now)?;
        let omitted = healthy.degradation().is_none() && permitted.degraded.is_none();
        let healthy_counts = engine.faults.counts().since(&before)?;
        engine.faults.write.store(true, Ordering::Relaxed);
        let pending = world.root(world.dana, "kite", PassOn::UseOnly, None);
        if !matches!(
            pending
                .as_ref()
                .err()
                .and_then(|error| error.downcast_ref::<GrantError>()),
            Some(GrantError::ProjectionPending { .. })
        ) {
            return Err(
                format!("counter fixture did not leave projection pending: {pending:?}").into(),
            );
        }
        let before = engine.faults.counts();
        let frame = world.grants.frame(world.directory.projection()?, None)?;
        let first = world.grants.explain_in(&frame, &request, world.now)?;
        let second = world.grants.explain_in(&frame, &request, world.now)?;
        let shared = frame
            .degradation()
            .zip(first.degraded.as_ref())
            .zip(second.degraded.as_ref())
            .is_some_and(|((frame, first), second)| {
                Arc::ptr_eq(frame, first)
                    && Arc::ptr_eq(frame, second)
                    && Arc::strong_count(frame) == 3
            });
        Ok((
            omitted,
            healthy_counts,
            shared,
            engine.faults.counts().since(&before)?,
        ))
    })();
    engine.faults.write.store(false, Ordering::Relaxed);
    let (omitted, healthy, shared, degraded) = finish_world(world, reading)?;
    assert!(omitted);
    assert!(shared);
    assert_eq!(
        healthy,
        OperationCounts {
            revisions: 1,
            reads: 1,
            writes: 0,
            appends: 0,
            pins: 0,
            snapshots: 0,
        }
    );
    assert_eq!(
        degraded,
        OperationCounts {
            revisions: 5,
            reads: 1,
            writes: 1,
            appends: 0,
            pins: 0,
            snapshots: 0,
        }
    );
    Ok(())
}

struct CostReading {
    root: lys_identity::grants::GrantId,
    permitted: lys_identity::grants::Permit,
    allowed: lys_identity::grants::Permit,
    stale: Result<lys_identity::grants::Permit, GrantError>,
    revoked: Result<lys_identity::grants::Permit, GrantError>,
    healthy_log: LogCounts,
    allowed_log: LogCounts,
    stale_log: LogCounts,
    revoked_log: LogCounts,
    healthy_cost: ObservationCosts,
    observed_cost: ObservationCosts,
    events: Vec<BTreeMap<String, String>>,
}

fn cost_reading(
    world: &mut World<Store, Engine>,
    engine: &Engine,
) -> Result<CostReading, Box<dyn Error>> {
    let root = world.root(world.lee, "tern", PassOn::UseOnly, None)?;
    let request = ExerciseRequest {
        caller: IdentityId::Person(world.lee),
        route: Route::Api,
        resource: alpha()?,
        action: lys_identity::grants::Action::new("read")?,
    };
    let healthy = Observations::default();
    let before = engine.faults.log_counts();
    let permitted = tracing::subscriber::with_default(healthy.clone(), || {
        world
            .grants
            .explain(world.directory.projection()?, &request, world.now, None)
    })?;
    let healthy_log = engine.faults.log_counts().since(&before)?;
    let healthy_cost = healthy.costs();
    engine.faults.write.store(true, Ordering::Relaxed);
    let pending = world.root(world.dana, "kite", PassOn::UseOnly, None);
    if !matches!(
        pending
            .as_ref()
            .err()
            .and_then(|error| error.downcast_ref::<GrantError>()),
        Some(GrantError::ProjectionPending { .. })
    ) {
        return Err(format!("cost fixture did not leave projection pending: {pending:?}").into());
    }
    let capture = Observations::default();
    let before = engine.faults.log_counts();
    let allowed = tracing::subscriber::with_default(capture.clone(), || {
        world
            .grants
            .explain(world.directory.projection()?, &request, world.now, None)
    })?;
    let allowed_log = engine.faults.log_counts().since(&before)?;
    let before = engine.faults.log_counts();
    let stale = tracing::subscriber::with_default(capture.clone(), || {
        world.grants.explain(
            world.directory.projection()?,
            &request,
            world.now,
            Some(world.grants.revision()),
        )
    });
    let stale_log = engine.faults.log_counts().since(&before)?;
    let revoke = world.grants.revoke(world.directory.projection()?, 
        &lys_identity::grants::RevokeRequest {
            operation: lys_identity::OperationId::generate()?,
            caller: IdentityId::Person(world.admin),
            route: Route::Api,
            grant: root,
            reason: "cost fixture revocation".to_owned(),
        },
        world.now,
    );
    if !matches!(revoke, Err(GrantError::ProjectionPending { .. })) {
        return Err(format!("cost fixture revocation was not pending: {revoke:?}").into());
    }
    let before = engine.faults.log_counts();
    let revoked = tracing::subscriber::with_default(capture.clone(), || {
        world
            .grants
            .explain(world.directory.projection()?, &request, world.now, None)
    });
    let revoked_log = engine.faults.log_counts().since(&before)?;
    let events = capture.read()?;
    Ok(CostReading {
        root,
        permitted,
        allowed,
        stale,
        revoked,
        healthy_log,
        allowed_log,
        stale_log,
        revoked_log,
        healthy_cost,
        observed_cost: capture.costs(),
        events,
    })
}

#[test]
fn observation_and_log_reads_are_counted_for_allowed_stale_and_revoked_answers() -> TestResult {
    let engine = Engine::default();
    let mut world = world(&engine)?;
    let reading = cost_reading(&mut world, &engine);
    engine.faults.write.store(false, Ordering::Relaxed);
    let CostReading {
        root,
        permitted: healthy,
        allowed,
        stale,
        revoked,
        healthy_log,
        allowed_log,
        stale_log,
        revoked_log,
        healthy_cost,
        observed_cost,
        events,
    } = finish_world(world, reading)?;
    assert_eq!(healthy.grant, root);
    assert_eq!(allowed.grant, root);
    assert_eq!(healthy.path, [root]);
    assert_eq!(allowed.path, [root]);
    assert!(matches!(stale, Err(GrantError::StaleDecision { .. })));
    assert!(matches!(revoked, Err(GrantError::Revoked { grant }) if grant == root.to_string()));
    for counts in [healthy_log, allowed_log, stale_log, revoked_log] {
        assert_eq!(counts, LogCounts::default());
    }
    assert_eq!(healthy_cost, ObservationCosts::default());
    assert_eq!(observed_cost.callbacks, 3);
    assert_eq!(observed_cost.locks, observed_cost.callbacks);
    assert_eq!(events.len(), 3);
    assert!(observed_cost.fields >= 3 * 3);
    assert!(observed_cost.bytes > 0);
    assert!(
        events
            .iter()
            .all(|fields| fields.get("step").is_some_and(|step| step == "project"))
    );
    eprintln!("grant observation fixture costs: {observed_cost:?}; physical syncs not measured");
    Ok(())
}
