#![cfg(test)]
//! A seat's optional conversation is validated without changing existing start admission.

#[path = "support/runner_start.rs"]
pub mod support;

use serde_json::json;
use std::error::Error;
use support::{Table, operation};

#[tokio::test(flavor = "multi_thread")]
async fn optional_native_ids_validate_and_a_named_conversation_is_resumed_by_its_id()
-> Result<(), Box<dyn Error>> {
    let table = Table::set().await?;
    let machine = table.lys_machine("Fixture").await?;
    table
        .ok(
            "/seats",
            &json!({
                "operation":operation()?, "name":"conversation-seat", "agent":table.agent(),
                "profile_version":1, "machine":machine,
            }),
        )
        .await?;
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
        assert_eq!(status, 200, "{reply}");
        assert_eq!(reply["admission"]["by"], "administrator");
        assert_eq!(reply["admission"]["action"], "seat.start");
        let native = reply["start"]["native_conversation"]
            .as_str()
            .ok_or("the conversation was not recorded")?;
        if let Some(conversation) = conversation {
            // A named conversation goes to the program to resume, whether or not Lys can see its transcript.
            assert_eq!(reply["start"]["conversation"], conversation);
            assert_eq!(native, conversation);
            assert_ne!(reply["start"]["runner"]["started_fresh"], true);
        } else {
            assert!(reply["start"]["conversation"].is_null());
        }
        assert_eq!(reply["seat"]["transport"], "pty");
        let session = reply["session"].as_str().ok_or("no admitted session")?;
        let live = table
            .serving
            .as_ref()
            .ok_or("no runner")?
            .sessions()
            .liveness(Some(session))?;
        assert_eq!(live.len(), 1);
        assert_eq!(live[0].harness_session.as_deref(), Some(native));
        assert!(live[0].alive && !live[0].harness.managed && !live[0].ended);
        let (status, repeated) = table.service.post(path, Some(&table.ada), &request).await?;
        assert_eq!(status, 200, "{repeated}");
        assert_eq!(repeated["session"], reply["session"]);
        table
            .ok(
                "/seats/conversation-seat/stop",
                &json!({"operation":operation()?, "force":true}),
            )
            .await?;
    }
    let (_, held) = table
        .service
        .get("/seats/conversation-seat", Some(&table.ada))
        .await?;
    assert_eq!(held["revision"], 5);
    assert_eq!(held["transport"], "pty");
    table.close()
}
