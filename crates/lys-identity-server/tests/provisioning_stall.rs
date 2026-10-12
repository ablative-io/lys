//! An older provisioning caller gets an explicit durable session allowance.

#[path = "support/runner_start.rs"]
pub mod support;

use serde_json::json;
use support::{Table, operation};

#[tokio::test(flavor = "multi_thread")]
async fn provisioning_without_stall_after_saves_the_written_value_across_restart()
-> Result<(), Box<dyn std::error::Error>> {
    let mut table = Table::set().await?;
    let path = format!("/agents/{}/provisioning", table.agent());
    let request = json!({"operation":operation()?, "from_version":1,
        "model_access":["claude-fable-5-1"], "tools":[], "skills":[], "mcp_servers":[],
        "instructions":"", "note":"", "harness":table.harness(),
        "working_folder":table.dir.path(), "session":{"compact":"/compact", "sensitive":false}});
    let saved = table.ok(&path, &request).await?;
    assert_eq!(saved["profile"]["session"]["stall_after"], 300);
    assert_eq!(saved["profile"]["session"]["compact"], "/compact");
    let repeated = table.ok(&path, &request).await?;
    assert_eq!(repeated["recorded"], saved["recorded"]);
    table.service.restart().await?;
    let (status, reopened) = table.service.get(&path, Some(&table.ada)).await?;
    assert_eq!(status, 200, "{reopened}");
    assert_eq!(reopened["profile"]["session"], saved["profile"]["session"]);
    table.close()
}
