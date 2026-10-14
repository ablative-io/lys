#![cfg(test)]
//! The calls page's kept outcome includes the upstream refusal's HTTP status.

use super::{line, read, usage};
use serde_json::json;

#[test]
fn an_upstream_refusal_keeps_its_http_status_in_the_call_row() {
    let mut refused = line("openai-responses", "upstream_refused", None);
    refused["http"] = json!(429);
    let bodies = read(&refused);
    let records = usage(&bodies);
    assert_eq!(records.len(), 1);
    let kept = records[0].record.as_ref().expect("the call's record");
    assert_eq!(kept.status, "upstream_refused (HTTP 429)");
}
