//! Observed boundaries hold input until the harness reports idle.

#[test]
fn an_observed_busy_boundary_without_tracking_waits_for_its_event()
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
        assert!(super::note(&table, "boundary").is_none());
        table
            .sessions
            .get_mut("boundary")
            .ok_or("no session")?
            .guard
            .idle = true;
        assert!(super::note(&table, "boundary").is_some());
        super::super::deliver(&mut table, "boundary");
        drop(table);
        let delivered =
            sessions.until_any(&std::sync::atomic::AtomicBool::new(false), |table| {
                table
                    .operations
                    .get("boundary-notice")
                    .filter(|held| held.state == crate::operations::OperationState::Delivered)
                    .cloned()
            })?;
        assert!(!delivered.words.contains("boundary_unsignalled"));
        Ok::<_, Box<dyn std::error::Error>>(())
    })();
    sessions.stop_all()?;
    checked
}
