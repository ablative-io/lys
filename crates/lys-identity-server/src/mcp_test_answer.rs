#![cfg(test)]
//! Transport assertions read the one visible JSON answer, including its kept leaf.
use serde_json::Value;

pub(crate) fn body(result: &Value) -> Value {
    assert!(
        result.get("structuredContent").is_none(),
        "the answer was duplicated"
    );
    assert_eq!(
        result["content"]
            .as_array()
            .expect("MCP content is an array")
            .len(),
        1
    );
    serde_json::from_str(
        result["content"][0]["text"]
            .as_str()
            .expect("the JSON answer is text content"),
    )
    .expect("the visible answer is JSON")
}
