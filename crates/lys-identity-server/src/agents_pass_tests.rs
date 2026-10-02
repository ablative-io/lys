use std::error::Error;

use serde_json::{Value, json};

#[path = "agents_pass_fixture.rs"]
mod fixture;
#[path = "agents_pass_runner.rs"]
mod runner;
use fixture::{Table, operation};

type TestResult = Result<(), Box<dyn Error>>;

fn refused(answer: &(u16, Value), status: u16, name: &str) {
    assert_eq!(answer.0, status, "{}", answer.1);
    assert_eq!(answer.1["refusal"], name, "{}", answer.1);
}

#[tokio::test]
async fn start_command_requires_its_action_before_checking_the_launch_profile() -> TestResult {
    let mut table = Table::fresh().await?;
    let path = format!("/agents/{}/start-command", table.target);
    let body = json!({"machine":operation()?, "operation":operation()?});
    refused(&table.call(&path, &body).await?, 403, "NotHeld");
    table.grant("starter", "agent.start-command").await?;
    refused(&table.call(&path, &body).await?, 404, "LaunchRecordMissing");
    let stop = format!("/agents/{}/stop", table.target);
    refused(
        &table
            .call(
                &stop,
                &json!({"operation":operation()?, "reason":"End work"}),
            )
            .await?,
        403,
        "NotHeld",
    );
    let other = format!("/agents/{}/start-command", table.agent);
    refused(&table.call(&other, &body).await?, 403, "NotHeld");
    assert_eq!(table.stopped().await?["stops"], json!([]));
    let running = table.launch_ready().await?;
    let body = json!({"machine":running.machine, "operation":operation()?});
    let (status, launched) = table.call(&path, &body).await?;
    assert_eq!(status, 200, "{launched}");
    assert_eq!(launched["agent"], table.target);
    assert_eq!(launched["runner"]["state"], "running");
    assert_eq!(launched["session"], body["operation"]);
    assert_eq!(
        table.runtime().await?["sessions"][0]["reported_by"],
        table.agent
    );
    table.grant("stopper", "agent.stop").await?;
    let (status, stopped) = table
        .call(
            &stop,
            &json!({"operation":operation()?, "reason":"End work"}),
        )
        .await?;
    assert_eq!(status, 200, "{stopped}");
    assert_eq!(stopped["by"], table.agent);
    assert_eq!(
        stopped["sessions_confirmed"][0]["session"],
        body["operation"]
    );
    running.close()
}

#[tokio::test]
async fn stop_requires_its_action_and_keeps_the_acting_agent_on_retry() -> TestResult {
    let mut table = Table::fresh().await?;
    let path = format!("/agents/{}/stop", table.target);
    let body = json!({"operation":operation()?, "reason":"End work"});
    refused(&table.call(&path, &body).await?, 403, "NotHeld");
    assert_eq!(table.stopped().await?["stops"], json!([]));
    table.grant("stopper", "agent.stop").await?;
    let (status, answer) = table.call(&path, &body).await?;
    assert_eq!(status, 200, "{answer}");
    assert_eq!(answer["by"], table.agent);
    assert_eq!(answer["agent"], table.target);
    assert_eq!(answer["operation"], body["operation"]);
    assert_eq!(answer["state"], "suspended");
    assert_eq!(answer["done"], true);
    assert!(
        answer["credentials_refused"]
            .as_str()
            .ok_or("missing broker refusal")?
            .starts_with("SecretsUnavailable"),
        "{answer}"
    );
    let (status, again) = table.call(&path, &body).await?;
    assert_eq!(status, 200, "{again}");
    assert_eq!(again, answer);
    table.service.restart().await?;
    let (status, restarted) = table.call(&path, &body).await?;
    assert_eq!(status, 200, "{restarted}");
    assert_eq!(restarted, answer);
    let kept = table.stopped().await?;
    assert_eq!(kept["stops"], json!([answer]));
    let start = format!("/agents/{}/start-command", table.target);
    refused(
        &table
            .call(
                &start,
                &json!({"machine":operation()?, "operation":operation()?}),
            )
            .await?,
        403,
        "NotHeld",
    );
    Ok(())
}
