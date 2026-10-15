#[test]
fn projection_failure_keeps_unrelated_authority_and_names_its_actual_revision() -> TestResult {
    let engine = Engine::default();
    let mut world = world(&engine)?;
    world.root(world.lee, "tern", PassOn::UseOnly, None)?;
    let healthy = explain(&mut world)?;
    engine.faults.write.store(true, Ordering::Relaxed);
    let pending = world.root(world.dana, "kite", PassOn::UseOnly, None);
    let revision = engine.inner.revision()?;
    let writes = engine.faults.writes.load(Ordering::Relaxed);
    let capture = Observations::default();
    let (result, repeated) = tracing::subscriber::with_default(capture.clone(), || {
        (explain(&mut world), explain(&mut world))
    });
    engine.faults.write.store(false, Ordering::Relaxed);
    let events = capture.read()?;
    assert!(matches!(
        pending
            .as_ref()
            .err()
            .and_then(|error| error.downcast_ref::<GrantError>()),
        Some(GrantError::ProjectionPending { .. })
    ));
    assert_eq!(result?.grant, healthy.grant);
    assert_eq!(repeated?.grant, healthy.grant);
    assert!(observed(&events, "project", WRITE_FAILURE, revision));
    let failures = engine.faults.writes.load(Ordering::Relaxed) - writes;
    assert_eq!(failures, 2);
    assert_eq!(
        u64::try_from(
            events
                .iter()
                .filter(|fields| {
                    fields.get("step").is_some_and(|step| step == "project")
                        && fields.values().any(|value| value.contains(WRITE_FAILURE))
                })
                .count()
        )?,
        failures,
        "every failed attempt keeps its own observation"
    );
    Ok(())
}

#[test]
fn unreadable_reconciliation_refuses_unrelated_authority_with_the_original_cause() -> TestResult {
    let engine = Engine::default();
    let mut world = world(&engine)?;
    let kinds = [RecipientKind::Person, RecipientKind::Agent];
    let root = world.root(world.dana, "kite", pass(&["read"], &kinds)?, None)?;
    world.root(world.lee, "tern", PassOn::UseOnly, None)?;
    explain(&mut world)?;
    let request = world.request(
        IdentityId::Person(world.dana),
        root,
        IdentityId::Person(world.tom),
        "tern",
        PassOn::UseOnly,
        None,
    )?;
    engine
        .faults
        .lost_acknowledgement
        .store(true, Ordering::Relaxed);
    let pending = world.delegate(&request);
    let before = FileLeafStore::open(&world.dir.path().join("grants"))?.extent();
    let result = explain(&mut world);
    engine.faults.log_unreadable.store(false, Ordering::Relaxed);
    let after = FileLeafStore::open(&world.dir.path().join("grants"))?.extent();
    assert!(matches!(
        pending,
        Err(GrantError::OperationUnresolved { .. })
    ));
    assert!(
        matches!(result, Err(GrantError::LogUnavailable { reason }) if reason.contains(LOG_FAILURE))
    );
    assert_eq!(after, before, "a refused explanation appends no use event");
    Ok(())
}

#[test]
fn startup_retains_the_original_projection_failure_and_selected_revision() -> TestResult {
    let original = Engine::default();
    let mut world = world(&original)?;
    world.root(world.lee, "tern", PassOn::UseOnly, None)?;
    let replacement = Engine::default();
    replacement.faults.write.store(true, Ordering::Relaxed);
    let revision = replacement.inner.revision()?;
    let capture = Observations::default();
    let opened =
        tracing::subscriber::with_default(capture.clone(), || world.reopen(replacement.clone()));
    replacement.faults.write.store(false, Ordering::Relaxed);
    let reading = (|| -> Result<_, Box<dyn Error>> {
        opened?;
        let writes = replacement.faults.writes.load(Ordering::Relaxed);
        let degraded = world
            .grants
            .take_startup_degradation()
            .ok_or("opening failure was not retained")?;
        let consumed = world.grants.take_startup_degradation().is_none();
        Ok((
            capture.read()?,
            degraded,
            consumed,
            writes,
            replacement.faults.writes.load(Ordering::Relaxed),
        ))
    })();
    let (events, degraded, consumed, before, after) = finish_world(world, reading)?;
    assert!(observed(&events, "project", WRITE_FAILURE, revision));
    assert_eq!(degraded.revision(), revision);
    assert_eq!(
        degraded.error(),
        &GrantError::PermissionEngineUnavailable {
            reason: WRITE_FAILURE.to_owned(),
        }
    );
    assert!(consumed);
    assert_eq!(
        before, after,
        "consuming the opening failure retries no projection"
    );
    Ok(())
}

#[test]
fn healthy_settlement_keeps_no_degradation_and_emits_no_failure() -> TestResult {
    let mut world = World::new()?;
    let capture = Observations::default();
    let readings =
        tracing::subscriber::with_default(capture.clone(), || -> Result<_, Box<dyn Error>> {
            let grant = world.root(world.lee, "tern", PassOn::UseOnly, None)?;
            let before = world.events();
            let healthy = world
                .grants
                .frame(world.directory.projection()?, None)?
                .degradation()
                .is_none();
            let permit = world.exercise(IdentityId::Person(world.lee), "read", Route::Api)?;
            Ok((
                grant,
                permit,
                healthy,
                world.grants.take_startup_degradation().is_none(),
                world.now,
                before,
                world.events(),
                capture.read()?,
            ))
        });
    let (grant, permit, healthy, opening, now, before, after, events) =
        finish_world(world, readings)?;
    assert_eq!(permit.grant, grant);
    assert_eq!(permit.actions, actions(&["read"])?);
    assert_eq!(now, T0 + 10);
    assert_eq!(after, before + 1);
    assert!(healthy);
    assert!(opening);
    assert!(events.is_empty(), "{events:?}");
    Ok(())
}

#[test]
fn degraded_frame_retains_the_selected_cause_and_recovers_without_a_marker() -> TestResult {
    let engine = Engine::default();
    let mut world = world(&engine)?;
    let capture = Observations::default();
    let readings =
        tracing::subscriber::with_default(capture.clone(), || -> Result<_, Box<dyn Error>> {
            world.root(world.lee, "tern", PassOn::UseOnly, None)?;
            let healthy = world
                .grants
                .frame(world.directory.projection()?, None)?
                .degradation()
                .is_none();
            engine.faults.write.store(true, Ordering::Relaxed);
            let pending = world.root(world.dana, "kite", PassOn::UseOnly, None);
            let revision = engine.inner.revision()?;
            let before = world.events();
            let degraded = {
                let frame = world.grants.frame(world.directory.projection()?, None)?;
                Arc::clone(
                    frame
                        .degradation()
                        .ok_or("projection failure lost its reading marker")?,
                )
            };
            engine.faults.write.store(false, Ordering::Relaxed);
            let recovered = world
                .grants
                .frame(world.directory.projection()?, None)?
                .degradation()
                .is_none();
            Ok((
                pending,
                healthy,
                degraded,
                recovered,
                revision,
                before,
                world.events(),
                capture.read()?,
            ))
        });
    engine.faults.write.store(false, Ordering::Relaxed);
    let (pending, healthy, degraded, recovered, revision, before, after, events) =
        finish_world(world, readings)?;
    assert!(matches!(
        pending
            .as_ref()
            .err()
            .and_then(|error| error.downcast_ref::<GrantError>()),
        Some(GrantError::ProjectionPending { .. })
    ));
    assert!(healthy);
    assert!(recovered);
    assert_eq!(degraded.revision(), revision);
    assert_eq!(
        degraded.error(),
        &GrantError::PermissionEngineUnavailable {
            reason: WRITE_FAILURE.to_owned(),
        }
    );
    assert!(observed(&events, "project", WRITE_FAILURE, revision));
    assert_eq!(before, after);
    Ok(())
}

#[test]
fn failed_frame_revision_read_returns_the_read_cause_and_observes_both_failures() -> TestResult {
    let engine = Engine::default();
    let mut world = world(&engine)?;
    let capture = Observations::default();
    let readings = (|| -> Result<_, Box<dyn Error>> {
        world.root(world.lee, "tern", PassOn::UseOnly, None)?;
        engine.faults.write.store(true, Ordering::Relaxed);
        let pending = world.root(world.dana, "kite", PassOn::UseOnly, None);
        let before = world.events();
        engine
            .faults
            .read_after_write
            .store(true, Ordering::Relaxed);
        let directory = world.directory.projection()?;
        let frame = tracing::subscriber::with_default(capture.clone(), || {
            world
                .grants
                .frame(directory, None)
                .map(|frame| frame.revision())
        });
        Ok((pending, frame, before, world.events(), capture.read()?))
    })();
    engine.faults.write.store(false, Ordering::Relaxed);
    engine.faults.revision.store(false, Ordering::Relaxed);
    engine
        .faults
        .read_after_write
        .store(false, Ordering::Relaxed);
    let (pending, frame, before, after, events) = finish_world(world, readings)?;
    assert!(matches!(
        pending
            .as_ref()
            .err()
            .and_then(|error| error.downcast_ref::<GrantError>()),
        Some(GrantError::ProjectionPending { .. })
    ));
    assert!(
        matches!(frame, Err(GrantError::PermissionEngineUnavailable { reason }) if reason == READ_FAILURE)
    );
    assert_eq!(before, after);
    for (step, cause) in [("project", WRITE_FAILURE), ("revision", READ_FAILURE)] {
        assert!(
            events.iter().any(
                |fields| fields.get("step").is_some_and(|value| value == step)
                    && fields.values().any(|value| value.contains(cause))
            ),
            "{events:?}"
        );
    }
    Ok(())
}

#[test]
fn mutation_acknowledgement_retains_the_projection_cause_and_real_revision() -> TestResult {
    let engine = Engine::default();
    let mut world = world(&engine)?;
    let kinds = [RecipientKind::Person, RecipientKind::Agent];
    let root = world.root(world.dana, "kite", pass(&["read"], &kinds)?, None)?;
    let request = world.request(
        IdentityId::Person(world.dana),
        root,
        IdentityId::Person(world.tom),
        "tern",
        PassOn::UseOnly,
        None,
    )?;
    let revision = engine.inner.revision()?;
    let capture = Observations::default();
    engine.faults.write.store(true, Ordering::Relaxed);
    let answer = tracing::subscriber::with_default(capture.clone(), || world.delegate(&request));
    engine.faults.write.store(false, Ordering::Relaxed);
    let events = capture.read()?;
    assert!(matches!(
        answer,
        Err(GrantError::ProjectionPending { index: 1, .. })
    ));
    assert!(observed(&events, "project", WRITE_FAILURE, revision));
    Ok(())
}

#[test]
fn failed_mutation_revision_read_is_named_instead_of_inventing_revision_zero() -> TestResult {
    let engine = Engine::default();
    let mut world = world(&engine)?;
    let kinds = [RecipientKind::Person, RecipientKind::Agent];
    let root = world.root(world.dana, "kite", pass(&["read"], &kinds)?, None)?;
    let request = world.request(
        IdentityId::Person(world.dana),
        root,
        IdentityId::Person(world.tom),
        "tern",
        PassOn::UseOnly,
        None,
    )?;
    let capture = Observations::default();
    engine.faults.write.store(true, Ordering::Relaxed);
    engine
        .faults
        .read_after_write
        .store(true, Ordering::Relaxed);
    let answer = tracing::subscriber::with_default(capture.clone(), || world.delegate(&request));
    engine.faults.write.store(false, Ordering::Relaxed);
    engine
        .faults
        .read_after_write
        .store(false, Ordering::Relaxed);
    engine.faults.revision.store(false, Ordering::Relaxed);
    let events = capture.read()?;
    assert!(
        matches!(answer, Err(GrantError::PermissionEngineUnavailable { reason }) if reason == READ_FAILURE)
    );
    for cause in [WRITE_FAILURE, READ_FAILURE] {
        assert!(
            events
                .iter()
                .any(|fields| fields.values().any(|value| value.contains(cause)))
        );
    }
    assert_eq!(
        FileLeafStore::open(&world.dir.path().join("grants"))?.extent(),
        2
    );
    Ok(())
}
