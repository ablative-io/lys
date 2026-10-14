//! Observed terminal boundaries remain bounded even without transcript tracking.

#[test]
fn an_observed_busy_boundary_without_tracking_arms_its_expiry_guard()
-> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let sessions = crate::Sessions::open(dir.path(), 4096)?;
    sessions.start(serde_json::from_value(serde_json::json!({
        "session":"boundary", "program":"/bin/cat", "arguments":[], "directory":"/",
        "environment":{}, "columns":80, "rows":24
    }))?)?;
    let checked = (|| {
        let mut table = sessions.lock()?;
        let session = table.sessions.get_mut("boundary").ok_or("no session")?;
        assert!(session.guard.tracking.is_none() && session.managed.is_none());
        session.guard.boundary_observed = true;
        session.guard.idle = false;
        let operation = crate::operations::Operation {
            operation: "boundary-notice".to_owned(),
            session: "boundary".to_owned(),
            request: crate::operations::OperationRequest::Notice {
                text: "queued notice".to_owned(),
            },
        };
        let answer = super::super::accept(&mut table, operation)?;
        assert_eq!(answer.state, crate::operations::OperationState::Accepted);
        assert!(
            table
                .operations
                .boundary_waiters
                .contains_key("boundary-notice"),
            "an observed busy boundary was left without its expiry guard"
        );
        let cancel = table
            .operations
            .boundary_waiters
            .remove("boundary-notice")
            .ok_or("no waiter")?;
        cancel.send(())?;
        Ok::<_, Box<dyn std::error::Error>>(())
    })();
    sessions.stop_all()?;
    checked?;
    dir.close()?;
    Ok(())
}
