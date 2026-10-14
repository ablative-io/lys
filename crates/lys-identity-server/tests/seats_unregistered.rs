#![cfg(test)]
//! An unread runner is named beside the sessions that other runners reported.

#[path = "support/runner_start.rs"]
pub mod support;

use serde_json::json;
use std::error::Error;
use support::{Table, operation};

#[tokio::test(flavor = "multi_thread")]
async fn an_unreachable_runner_is_named_without_hiding_a_reachable_runner()
-> Result<(), Box<dyn Error>> {
    let table = Table::set().await?;
    let reachable = table.machine(&json!({
        "operation": operation()?, "name": "Reachable laptop", "kind": "laptop", "runtime": "sh",
        "slots": 1, "may_run": [table.agent()], "may_reach": [],
    }), Some(json!({"kind": "lys"}))).await?;
    let unreachable = table
        .machine(
            &json!({
                "operation": operation()?, "name": "Dean laptop", "kind": "laptop", "runtime": "sh",
                "slots": 1, "may_run": [table.agent()], "may_reach": [],
            }),
            Some(json!({"kind": "socket", "path": table.dir.path().join("absent.sock")})),
        )
        .await?;
    let (status, answer) = table
        .service
        .get("/seats/unregistered", Some(&table.ada))
        .await?;
    assert_eq!(status, 200, "{answer}");
    let unanswered = answer["unanswered"]
        .as_array()
        .ok_or("no unanswered runners")?;
    assert_eq!(unanswered.len(), 1, "{answer}");
    assert_eq!(unanswered[0]["machine"], unreachable);
    assert_eq!(unanswered[0]["name"], "Dean laptop");
    assert_eq!(unanswered[0]["refusal"], "runner_unreachable");
    assert_ne!(unanswered[0]["machine"], reachable);
    table.close()
}
