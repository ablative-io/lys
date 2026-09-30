#![cfg(test)]

#[path = "support/harness_description.rs"]
mod harness_description;

use std::error::Error;

use lys_identity_server::launch_template::{Start, render};
use lys_identity_server::provisioning_store::Version;
use serde_json::json;

#[test]
fn the_start_command_carries_the_declared_program_and_models() -> Result<(), Box<dyn Error>> {
    let version: Version = serde_json::from_value(json!({
        "number": 1, "operation": "record-profile", "set_by": "operator", "set_at": 1,
        "settings": {
            "harness": harness_description::declared(),
            "model_access": ["primary-model", "fallback-model"],
            "tools": [], "skills": [], "mcp_servers": [],
            "instructions": "Follow the reviewed instructions.", "note": ""
        }
    }))?;
    let rendered = render(
        &Start {
            agent: "agent",
            session: "session",
            machine: "machine",
            runtime: "unrelated-machine-runtime",
            version: &version,
            skills: &[],
            policy: None,
        },
        &[],
    )?;
    assert!(
        rendered.command.contains("/opt/seat/bin/claude"),
        "{}",
        rendered.command
    );
    assert!(
        rendered.command.contains("--model primary-model"),
        "{}",
        rendered.command
    );
    assert!(
        rendered.command.contains("--fallback-model fallback-model"),
        "{}",
        rendered.command
    );
    assert!(!rendered.command.contains("unrelated-machine-runtime"));
    Ok(())
}
