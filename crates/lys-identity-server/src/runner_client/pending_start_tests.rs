#![cfg(test)]
//! A status probe cannot cancel an admitted launch before its runner start.

use std::error::Error;

use crate::budgets_feed::entry_tests::fixture;
use crate::runner_client::RunnerRecord;
use crate::runner_sessions::Driven;
use crate::runtime_state::{Report, Reported};

#[tokio::test]
async fn starting_session_unknown_does_not_cancel_pending_start() -> Result<(), Box<dyn Error>> {
    let table = fixture().await?;
    let pending = table
        .state
        .runners
        .begin_start("machine", "pending-start")?;
    crate::runtime_api::with_runtime(&table.state, |store| {
        store.report(Report {
            operation: "admitted-start".to_owned(),
            session: "pending-start".to_owned(),
            agent: Some(table.agent.clone()),
            machine: "machine".to_owned(),
            state: Reported::Starting,
            what: "start admitted".to_owned(),
            confirmation: String::new(),
            reported_by: "owner".to_owned(),
            at: 1,
            launch: None,
        })
    })?;
    super::record_unknown(&table.state, "machine", "pending-start")?;
    crate::runtime_api::with_runtime(&table.state, |store| {
        let tracked = store
            .session("pending-start")
            .ok_or(crate::ServerError::RuntimeSessionUnknown)?;
        assert!(
            !tracked.stopped(),
            "a pre-spawn probe cancelled the admitted start"
        );
        assert_eq!(tracked.reports.len(), 1);
        Ok(())
    })?;
    let driven = Driven {
        session: "pending-start".to_owned(),
        agent: table.agent.clone(),
        machine: "machine".to_owned(),
        runner: RunnerRecord::Lys,
    };
    crate::runner_sessions::record_running(&table.state, &driven, 42)?;
    crate::runtime_api::with_runtime(&table.state, |store| {
        let tracked = store
            .session("pending-start")
            .ok_or(crate::ServerError::RuntimeSessionUnknown)?;
        assert_eq!(
            tracked.latest().map(|report| report.state),
            Some(Reported::Running)
        );
        Ok(())
    })?;
    drop(pending);
    super::record_unknown(&table.state, "machine", "pending-start")?;
    crate::runtime_api::with_runtime(&table.state, |store| {
        assert!(
            store
                .session("pending-start")
                .is_some_and(crate::runtime_state::Tracked::stopped)
        );
        Ok(())
    })?;
    table.finish().await
}
