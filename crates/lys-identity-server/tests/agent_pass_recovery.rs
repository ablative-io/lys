#![cfg(test)]
//! Ending routes revoke run passes even when their cache cannot be saved.
#[path = "support/pass_records.rs"]
mod support;
use serde_json::json;
use support::{Fixture, TestResult, fail_save, operation, restore};

#[tokio::test]
async fn agent_pass_startup_keeps_a_live_bound_run() -> TestResult {
    let mut fixture = Fixture::open().await?;
    assert!(fixture.contains()?);
    fixture.service.restart().await?;
    assert!(fixture.contains()?);
    Ok(())
}

#[tokio::test]
async fn agent_pass_failed_end_report_is_pruned_before_reopened_service_answers() -> TestResult {
    let mut fixture = Fixture::open().await?;
    let file = fixture.file();
    let earlier = fail_save(&file)?;
    let path = format!(
        "/agents/{}/runtime/sessions/{}/reports",
        fixture.agent, fixture.session
    );
    let (status, refusal) = fixture.service.post(&path, Some(&fixture.cookie), &json!({
        "operation":operation()?, "machine":fixture.machine, "state":"stopped", "what":"process ended", "confirmation":"observed exit"
    })).await?;
    assert_eq!(status, 401);
    assert_eq!(refusal["refusal"], "AgentPassRefused");
    assert!(!refusal.to_string().contains(fixture.pass.as_str()));
    restore(&file, &earlier)?;
    fixture.service.restart().await?;
    assert!(!fixture.contains()?);
    fixture.service.restart().await?;
    assert!(!fixture.contains()?);
    Ok(())
}

#[tokio::test]
async fn agent_pass_failed_stop_remains_ended_after_reinstatement_and_reopen() -> TestResult {
    let mut fixture = Fixture::open().await?;
    let file = fixture.file();
    let earlier = fail_save(&file)?;
    let path = format!("/agents/{}/stop", fixture.agent);
    let (status, refusal) = fixture
        .service
        .post(
            &path,
            Some(&fixture.cookie),
            &json!({"operation":operation()?, "reason":"end this run"}),
        )
        .await?;
    assert_eq!(status, 401);
    assert_eq!(refusal["refusal"], "AgentPassRefused");
    restore(&file, &earlier)?;
    let path = format!("/identities/{}/transitions", fixture.agent);
    let (status, _) = fixture.service.post(&path, Some(&fixture.cookie), &json!({"operation":operation()?, "transition":"reinstate", "reason":"permit a fresh run"})).await?;
    assert_eq!(status, 200);
    fixture.service.restart().await?;
    assert!(!fixture.contains()?);
    Ok(())
}

#[tokio::test]
async fn agent_pass_failed_withdrawal_remains_ended_after_reopen() -> TestResult {
    let mut fixture = Fixture::open().await?;
    let file = fixture.file();
    let earlier = fail_save(&file)?;
    let path = format!("/launch-records/{}/withdraw", fixture.launch);
    let (status, refusal) = fixture
        .service
        .post(&path, Some(&fixture.cookie), &json!({}))
        .await?;
    assert_eq!(status, 503);
    assert_eq!(refusal["error"], "launch_records_unavailable");
    assert!(!refusal.to_string().contains(fixture.pass.as_str()));
    restore(&file, &earlier)?;
    fixture.service.restart().await?;
    assert!(!fixture.contains()?);
    let (status, _) = fixture
        .service
        .post(&path, Some(&fixture.cookie), &json!({}))
        .await?;
    assert_eq!(status, 200);
    assert!(!fixture.contains()?);
    Ok(())
}
