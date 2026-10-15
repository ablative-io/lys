#[test]
fn mutation_acknowledgement_uses_the_real_frontier_and_preserves_the_original_receipt() -> TestResult
{
    let engine = Engine::default();
    let mut world = world(&engine)?;
    let capture = Observations::default();
    let reading =
        tracing::subscriber::with_default(capture.clone(), || -> Result<_, Box<dyn Error>> {
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
            engine.faults.write.store(true, Ordering::Relaxed);
            let pending = world.delegate(&request);
            let at_index = engine.inner.revision()?;
            let before = world.events();
            engine.faults.write_then_fail.store(true, Ordering::Relaxed);
            let answered = world.delegate(&request)?;
            let projected = engine.inner.revision()?;
            let repeated = world.delegate(&request)?;
            world.reopen(engine.clone())?;
            let reopened = world.delegate(&request)?;
            let mut changed = request;
            changed.relation = lys_identity::grants::Relation::new("kite")?;
            let refused = world.delegate(&changed);
            Ok((
                pending,
                at_index,
                answered,
                projected,
                repeated,
                reopened,
                refused,
                before,
                world.events(),
                capture.read()?,
            ))
        });
    engine.faults.write.store(false, Ordering::Relaxed);
    engine
        .faults
        .write_then_fail
        .store(false, Ordering::Relaxed);
    let (
        pending,
        at_index,
        answered,
        projected,
        repeated,
        reopened,
        refused,
        before,
        after,
        observations,
    ) = finish_world(world, reading)?;
    assert_eq!(at_index, answered.index);
    assert!(
        matches!(pending, Err(GrantError::ProjectionPending { index, operation, grant })
        if index == answered.index && operation == answered.event.operation().to_string()
            && grant == answered.event.grant().to_string())
    );
    assert_eq!(projected, answered.index + 1);
    let marker = answered
        .degraded
        .as_deref()
        .ok_or("acknowledgement lost its projection cause")?;
    assert_eq!(marker.revision(), projected);
    assert_eq!(
        marker.error(),
        &GrantError::PermissionEngineUnavailable {
            reason: WRITE_FAILURE.to_owned(),
        }
    );
    assert!(observed(&observations, "project", WRITE_FAILURE, at_index));
    assert!(observed(&observations, "project", WRITE_FAILURE, projected));
    for retry in [repeated, reopened] {
        assert_eq!(retry.event, answered.event);
        assert_eq!(retry.index, answered.index);
        assert_eq!(retry.receipt, answered.receipt);
        assert!(retry.degraded.is_none());
    }
    assert!(matches!(refused, Err(GrantError::OperationReused { .. })));
    assert_eq!(before, after);
    Ok(())
}

struct ChangedReading {
    result: Result<(), GrantError>,
    projected: u64,
    actual: u64,
    before: u64,
    after: u64,
}

fn changed_reading(change_on_read: bool) -> Result<ChangedReading, Box<dyn Error>> {
    let engine = Engine::default();
    let mut world = world(&engine)?;
    let reading = (|| -> Result<_, Box<dyn Error>> {
        world.root(world.lee, "tern", PassOn::UseOnly, None)?;
        engine.faults.write.store(true, Ordering::Relaxed);
        let pending = world.root(world.dana, "kite", PassOn::UseOnly, None);
        if !matches!(
            pending
                .as_ref()
                .err()
                .and_then(|error| error.downcast_ref::<GrantError>()),
            Some(GrantError::ProjectionPending { .. })
        ) {
            return Err(format!(
                "the reading fixture did not leave projection pending: {pending:?}"
            )
            .into());
        }
        let projected = engine.inner.revision()?;
        let before = world.events();
        let directory = world.directory.projection()?;
        let result = if change_on_read {
            engine.faults.advance_on_read.store(true, Ordering::Relaxed);
            world.grants.frame(directory, None).map(drop)
        } else {
            let frame = world.grants.frame(directory, None)?;
            let mut writer = engine.inner.clone();
            writer.write(projected + 1, &[], &[])?;
            let request = ExerciseRequest {
                caller: IdentityId::Person(world.lee),
                route: Route::Api,
                resource: alpha()?,
                action: lys_identity::grants::Action::new("read")?,
            };
            world
                .grants
                .explain_in(&frame, &request, world.now)
                .map(drop)
        };
        Ok(ChangedReading {
            result,
            projected,
            actual: engine.inner.revision()?,
            before,
            after: world.events(),
        })
    })();
    engine.faults.write.store(false, Ordering::Relaxed);
    engine
        .faults
        .advance_on_read
        .store(false, Ordering::Relaxed);
    finish_world(world, reading)
}

#[test]
fn degraded_relationships_that_move_during_the_read_make_no_frame() -> TestResult {
    let reading = changed_reading(true)?;
    assert_eq!(reading.actual, reading.projected + 1);
    assert_eq!(
        reading.result,
        Err(GrantError::StaleDecision {
            required: reading.actual,
            projected: reading.projected,
        })
    );
    assert_eq!(reading.before, reading.after);
    Ok(())
}

#[test]
fn degraded_frame_cannot_be_explained_after_the_relationship_revision_moves() -> TestResult {
    let reading = changed_reading(false)?;
    assert_eq!(reading.actual, reading.projected + 1);
    assert_eq!(
        reading.result,
        Err(GrantError::StaleDecision {
            required: reading.actual,
            projected: reading.projected,
        })
    );
    assert_eq!(reading.before, reading.after);
    Ok(())
}
