#[test]
fn absent_tail_capability_is_named_without_an_invented_operation() -> TestResult {
    let engine = Engine::default();
    let world = world(&engine)?;
    let result = world.grants.ledger().acquire_tail();
    finish_world(world, Ok(()))?;
    assert!(
        matches!(result, Err(GrantError::Identity(lys_identity::IdentityError::LogUnavailable { reason })) if reason.contains("tail witness capability is absent"))
    );
    Ok(())
}

#[test]
fn authenticated_tail_keeps_two_writers_and_never_waives_reconciliation() -> TestResult {
    let engine = Engine::default();
    let mut world = witnessed_world(&engine)?;
    let reading = (|| -> TestResult {
        world.root(world.lee, "tern", PassOn::UseOnly, None)?;
        let before = world.grants.revision();
        engine
            .faults
            .lost_acknowledgement
            .store(true, Ordering::Relaxed);
        let pending = world.root(world.dana, "kite", PassOn::UseOnly, None);
        assert!(pending.is_err());
        let held = world
            .grants
            .ledger()
            .uncertain()
            .cloned()
            .ok_or("no unresolved operation")?;
        let path = world.dir.path().join("grants");
        let key = lys_core::Ed25519Identity::load(&world.dir.path().join("service.key"))?;
        let (mut writer, opening) = lys_identity::grants::GrantLedger::open(
            Box::new(move || FileLeafStore::open(&path)),
            &key,
            lys_identity::restart::SNAPSHOT_EVERY,
        )?;
        drop(opening);
        let second = lys_identity::grants::GrantEvent::new(
            lys_identity::OperationId::generate()?,
            IdentityId::Person(world.admin),
            world.now,
            lys_identity::grants::GrantChange::Revoke {
                grant: held.grant,
                reason: "withdraw authority".to_owned(),
            },
        )?;
        writer.append(&lys_identity::grants::sign_grant_event(
            second.clone(),
            &key,
        )?)?;
        let tail = world.grants.ledger().acquire_tail()?;
        assert_eq!(tail.lower.tree_size, before);
        assert_eq!(tail.upper.tree_size, before + 2);
        world.grants.ledger().with_verified_tail(&tail, |events| {
            assert_eq!(events.len(), 2);
            assert_eq!(events[0].event(), &held.event);
            assert_eq!(events[1].event(), &second);
            Ok(())
        })?;
        let mut shortened = tail.clone();
        assert!(shortened.leaves.pop().is_some());
        let mut called = false;
        let refused = world
            .grants
            .ledger()
            .with_verified_tail(&shortened, |events| {
                called = true;
                Ok(events.len())
            });
        assert!(refused.is_err());
        assert!(!called);
        let after = tail.upper.tree_size;
        assert!(
            matches!(explain(&mut world), Err(GrantError::LogUnavailable { reason }) if reason.contains(LOG_FAILURE))
        );
        assert_eq!(
            FileLeafStore::open(&world.dir.path().join("grants"))?.extent(),
            after
        );
        assert_eq!(world.grants.ledger().uncertain(), Some(&held));
        Ok(())
    })();
    engine.faults.log_unreadable.store(false, Ordering::Relaxed);
    finish_world(world, reading)
}
