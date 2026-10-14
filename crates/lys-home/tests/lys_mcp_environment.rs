#![cfg(test)]
//! Each native harness receives the run identity and pass for its MCP children.

use std::error::Error;

use lys_home::harness::launch_fields::InstructionsMode;
use lys_home::harness::lys_mcp::{self, LysMcp};
use lys_home::harness::rendering_launch;
use serde_json::{Value, json};

#[test]
fn a_rendered_claude_launch_exports_both_agent_and_agent_pass() -> Result<(), Box<dyn Error>> {
    let mut template: Value = serde_json::from_str(include_str!("fixtures/launch/template.json"))?;
    template["slots"]["env"]["LYS_AGENT"] = json!("agent-fixture");
    let mut launch = rendering_launch::render(
        "claude-code/template-v1",
        "/usr/bin/claude",
        &template.to_string(),
        InstructionsMode::Keep,
    )?;
    lys_mcp::render(
        &mut launch.files,
        &mut launch.arguments,
        &mut launch.environment,
        &LysMcp {
            url: "https://service.invalid/api/mcp".to_owned(),
            pass: "fixture-run-pass".to_owned(),
            seat: None,
        },
    )?;
    assert!(
        launch
            .environment
            .get("LYS_AGENT")
            .is_some_and(|agent| agent == "agent-fixture")
    );
    assert!(
        launch
            .environment
            .get("LYS_AGENT_PASS")
            .is_some_and(|pass| pass == "fixture-run-pass")
    );
    assert!(
        launch
            .arguments
            .iter()
            .all(|argument| !argument.contains("fixture-run-pass"))
    );
    Ok(())
}
