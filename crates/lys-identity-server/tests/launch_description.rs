//! A harness name is a label. Its complete description governs the render.
use std::error::Error;

use lys_identity_server::launch_template::{Start, render};
use lys_identity_server::provisioning_store::Version;
use serde_json::{Value, json};

fn version(name: &str) -> Result<Version, serde_json::Error> {
    let description: Value = json!({
        "models": {"minimum": 1, "maximum": null,
            "further_encoding": {"kind": "delimited", "separator": ","}},
        "permissions": {"modes": ["plan"], "rule_forms": ["tool_specifier"]},
        "mcp": {"transports": ["http", "stdio"], "working_directory": false,
            "handle_variables": true, "channel_policies": ["off", "wake"]},
        "rendering_contract": "claude-code/template-v1"
    });
    serde_json::from_value(json!({
        "number": 1, "operation": "recorded-operation", "set_by": "person",
        "set_at": 1,
        "settings": {
            "model_access": ["model-a", "model-b"], "tools": [], "skills": [],
            "mcp_servers": [], "instructions": "same instructions", "note": "",
            "harness": {"name": name, "program": "/opt/seat/bin/harness",
                "package": "verified-package", "description": description}
        }
    }))
}

#[test]
fn changing_only_a_complete_descriptions_name_keeps_the_render_byte_identical()
-> Result<(), Box<dyn Error>> {
    let first = version("First recorded name")?;
    let second = version("A completely different name")?;
    let render_version = |version| {
        render(
            &Start {
                agent: "agent",
                session: "session",
                machine: "machine",
                runtime: "runtime",
                version,
                skills: &[],
                policy: None,
            },
            &[],
        )
    };
    let first = render_version(&first)?;
    let second = render_version(&second)?;
    assert_eq!(first.template, second.template);
    assert_eq!(first.template_sha256, second.template_sha256);
    assert_eq!(first.command, second.command);
    Ok(())
}
