use axum::response::IntoResponse;
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn std::error::Error>>;

pub(super) fn kept(result: &Value, key: &str) -> Result<Value, Box<dyn std::error::Error>> {
    let content = result["content"].as_array().ok_or("missing MCP content")?;
    for part in content {
        let text = part["text"]
            .as_str()
            .ok_or("the receipt is not text content")?;
        let leaf: Value = serde_json::from_str(text)?;
        if let Some(named) = leaf.get(key) {
            return Ok(named.clone());
        }
    }
    Err(format!("MCP content names no {key}").into())
}

#[test]
fn initialization_names_the_four_tools_and_discovery_as_the_first_call() -> TestResult {
    let result = super::initialize(&json!({"protocolVersion":"2025-11-25", "capabilities":{}, "clientInfo":{"name":"fixture","version":"1"}}))
        .map_err(|(_, reason)| reason)?;
    let words = result["instructions"]
        .as_str()
        .ok_or("no connection instructions")?;
    for tool in ["what-can-I-do", "read", "change", "drafts"] {
        assert!(words.contains(tool));
    }
    assert!(words.contains("first"));
    Ok(())
}

#[tokio::test]
async fn tool_schemas_are_distinct_and_discovery_requires_no_route_arguments() -> TestResult {
    let response = super::tools(&json!(1), &Value::Null);
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await?;
    let result: Value = serde_json::from_slice(&bytes)?;
    let tools = result["result"]["tools"]
        .as_array()
        .ok_or("missing tool list")?;
    assert_eq!(tools.len(), 4);
    assert_eq!(tools[0]["inputSchema"]["required"], json!([]));
    for left in 0..tools.len() {
        for right in left + 1..tools.len() {
            assert_ne!(tools[left]["inputSchema"], tools[right]["inputSchema"]);
        }
    }
    Ok(())
}

#[tokio::test]
async fn a_route_answer_is_carried_once_as_json_text_for_every_advertised_protocol_version()
-> TestResult {
    let result =
        crate::mcp_callers::rendered(axum::Json(json!({"answer":"one copy"})).into_response())
            .await
            .map_err(|(_, reason)| reason)?;
    assert_eq!(
        crate::mcp_test_answer::body(&result)["body"]["answer"],
        "one copy"
    );
    Ok(())
}
