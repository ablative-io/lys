#![cfg(test)]
//! A seat's optional conversation is validated without changing existing start admission.

#[path = "support/runner_start.rs"]
pub mod support;

use std::error::Error;
use serde_json::json;
use support::{Table, operation};

#[tokio::test(flavor = "multi_thread")]
async fn a_seat_start_accepts_optional_harness_ids_and_refuses_bad_ids_before_starting()
    -> Result<(), Box<dyn Error>> {
    let table = Table::set().await?;
    let machine = table.lys_machine("Fixture").await?;
    table.ok("/seats", &json!({
        "operation":operation()?, "name":"conversation-seat", "agent":table.agent(),
        "profile_version":1, "machine":machine,
    })).await?;
    let path = "/seats/conversation-seat/start";
    for conversation in ["", " padded ", "../thread", "not-a-uuid"] {
        let request = json!({"operation":operation()?, "conversation":conversation});
        let (status, reply) = table.service.post(path, Some(&table.ada), &request).await?;
        assert_eq!(status, 409, "{reply}");
        assert_eq!(reply["refusal"], "control_binding_invalid", "{reply}");
    }
    for conversation in [None, Some("019c6e27-e55b-73d1-87d8-4e01f1f75043")] {
        let mut request = json!({"operation":operation()?});
        if let Some(conversation) = conversation {
            request["conversation"] = json!(conversation);
        }
        let (status, reply) = table.service.post(path, Some(&table.ada), &request).await?;
        assert_eq!(status, 409, "{reply}");
        assert_eq!(reply["refusal"], "seat_not_managed", "{reply}");
    }
    let (_, live) = table.service.get("/runtime/live", Some(&table.ada)).await?;
    assert_eq!(live["sessions"].as_array().map(Vec::len), Some(0), "{live}");
    let (_, held) = table.service.get("/seats/conversation-seat", Some(&table.ada)).await?;
    assert_eq!(held["revision"], 1);
    assert!(held.get("session").is_none());
    table.close()
}
