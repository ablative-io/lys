//! Missing policy refuses a launch before an agent can run. Every registered
//! agent is given a default-deny policy, so a launch finds none only on a
//! service that keeps no policies.

#[path = "support/runner_start.rs"]
pub mod support;

use std::error::Error;
use std::sync::Arc;

use lys_runner::{Act, Answer, Client};
use serde_json::json;
use support::Table;

#[tokio::test(flavor = "multi_thread")]
async fn an_agent_without_a_policy_cannot_start() -> Result<(), Box<dyn Error>> {
    let mut table = Table::set().await?;
    table
        .service
        .restart_adjusted(|config| config.policies_dir = None)
        .await?;
    let agent = table.agent();
    let machine = table
        .machine(
            &json!({
                "operation": support::operation()?, "name": "Policy check", "kind": "laptop",
                "runtime": "local", "slots": 1, "may_run": [agent], "may_reach": [],
            }),
            Some(json!({"kind": "lys"})),
        )
        .await?;
    let answer = table
        .start(
            &agent,
            &json!({"machine": machine, "operation": support::operation()?}),
        )
        .await;
    let client = Client::new(
        table.dir.path().join("runner.sock"),
        Arc::clone(&table.server_key),
    );
    let runner_status = client.ask(&Act::Status { session: None });
    table.close()?;
    let (status, refused) = answer?;
    assert_eq!(status, 403, "{refused}");
    assert_eq!(refused["refusal"], "AgentHasNoPolicy");
    let Answer::Status { status } = runner_status? else {
        return Err("runner answered no status".into());
    };
    assert!(
        status.sessions.is_empty(),
        "a refused start created a session"
    );
    Ok(())
}
