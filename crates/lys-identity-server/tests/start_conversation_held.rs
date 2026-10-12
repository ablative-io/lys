#![cfg(test)]
//! A live writer or an unfinished stop prevents another resume.

#[path = "support/runner_start.rs"]
pub mod support;

use serde_json::{Value, json};
use std::error::Error;
use support::{Table, operation};

type TestResult = Result<(), Box<dyn Error>>;
const CONVERSATION: &str = "019c6e27-e55b-73d1-87d8-4e01f1f75043";

async fn profiled() -> Result<(Table, String), Box<dyn Error>> {
    let table = Table::unprofiled().await?;
    let version = operation()?;
    let path = format!("/agents/{}/provisioning", table.agent());
    table
        .ok(
            &path,
            &json!({
                "operation": version, "from_version": 0,
                "model_access": ["claude-fable-5-1"], "tools": [], "skills": [],
                "mcp_servers": [], "instructions": "", "note": "", "harness": table.harness(),
                "permissions": {"default_mode": "plan"}, "working_folder": table.dir.path(),
                "session": {"resume": CONVERSATION, "config_dir": table.dir.path().join("config-home")}
            }),
        )
        .await?;
    table
        .ok(
            &format!("{path}/1/review"),
            &json!({"operation": operation()?}),
        )
        .await?;
    Ok((table, version))
}

async fn start(
    table: &Table,
    version: &str,
    machine: &str,
) -> Result<(u16, Value), Box<dyn Error>> {
    Ok(table
        .service
        .post(
            &format!("/agents/{}/start", table.agent()),
            Some(&table.ada),
            &json!({"profile_version": version, "machine": machine}),
        )
        .await?)
}

#[tokio::test(flavor = "multi_thread")]
async fn start_held_conversation_names_the_live_session_and_records_refusal() -> TestResult {
    let (table, version) = profiled().await?;
    let machine = table.lys_machine("Writer").await?;
    let (status, first) = start(&table, &version, &machine).await?;
    assert_eq!(status, 200, "{first}");
    let first_session = first["runner"]["session"]
        .as_str()
        .ok_or("first start has no session")?;
    let (status, second) = start(&table, &version, &machine).await?;
    assert_eq!(status, 409, "{second}");
    assert_eq!(second["error"], "conversation_held", "{second}");
    assert!(
        second["words"]
            .as_str()
            .is_some_and(|words| words.contains(first_session)),
        "{second}"
    );
    let (_, receipts) = table
        .service
        .get(
            &format!("/agents/{}/runtime/sessions", table.agent()),
            Some(&table.ada),
        )
        .await?;
    assert!(
        receipts.to_string().contains("conversation_held"),
        "{receipts}"
    );
    let index = second["start_record"]["index"]
        .as_u64()
        .ok_or("refused start has no receipt")?;
    let (status, record) = table
        .service
        .get(&format!("/runner-receipts/{index}"), Some(&table.ada))
        .await?;
    assert_eq!(status, 200, "{record}");
    assert_eq!(record, second["start_record"], "{record}");
    assert_eq!(record["act"]["act"], "start");
    assert!(
        record["act"]["outcome"]
            .as_str()
            .is_some_and(|outcome| outcome.starts_with("conversation_held:")
                && outcome.contains(first_session)),
        "{record}"
    );
    table.close()
}
