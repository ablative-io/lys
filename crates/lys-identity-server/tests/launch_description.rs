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

#[test]
fn an_unknown_rendering_contract_is_refused_by_identifier() -> Result<(), Box<dyn Error>> {
    let mut version = version("Claude Code")?;
    version
        .settings
        .harness
        .as_mut()
        .ok_or("harness missing")?
        .description
        .rendering_contract = "missing/contract-v9".to_owned();
    let start = Start {
        agent: "agent",
        session: "session",
        machine: "machine",
        runtime: "runtime",
        version: &version,
        skills: &[],
        policy: None,
    };
    let error = render(&start, &[])
        .err()
        .ok_or("unregistered contract rendered")?;
    assert!(error.to_string().contains("missing/contract-v9"), "{error}");
    Ok(())
}

#[test]
fn model_limits_come_from_the_description_even_when_the_name_is_unchanged()
-> Result<(), Box<dyn Error>> {
    let mut version = version("The same name")?;
    let harness = version.settings.harness.as_mut().ok_or("harness missing")?;
    lys_identity_server::launch_fields::models(harness, &version.settings.model_access)?;
    harness.description.models.maximum = Some(1);
    let error = lys_identity_server::launch_fields::models(harness, &version.settings.model_access)
        .err()
        .ok_or("second model admitted")?;
    assert!(error.to_string().contains("model-b"), "{error}");
    Ok(())
}

#[test]
fn a_missing_capability_never_falls_back_to_the_display_name() -> Result<(), Box<dyn Error>> {
    let mut raw = serde_json::to_value(version("Claude Code")?)?;
    raw["settings"]["harness"]["description"]["mcp"]
        .as_object_mut()
        .ok_or("mcp missing")?
        .remove("handle_variables");
    let error = serde_json::from_value::<Version>(raw)
        .err()
        .ok_or("incomplete description admitted")?;
    assert!(error.to_string().contains("handle_variables"), "{error}");
    Ok(())
}

#[test]
fn mcp_capabilities_are_checked_before_a_render() -> Result<(), Box<dyn Error>> {
    let mut version = version("An ordinary label")?;
    version.settings.mcp_servers = serde_json::from_value(json!([
        {"name": "messages", "url": "https://example.test/mcp", "channel": "wake"}
    ]))?;
    let harness = version.settings.harness.as_mut().ok_or("harness missing")?;
    lys_identity_server::launch_fields::mcp(harness, &version.settings.mcp_servers)?;
    harness.description.mcp.channel_policies = vec![lys_home::harness::launch_fields::Channel::Off];
    let refused = lys_identity_server::launch_fields::mcp(harness, &version.settings.mcp_servers)
        .err()
        .ok_or("undeclared wake admitted")?;
    assert!(
        refused.to_string().contains("channel_policies"),
        "{refused}"
    );
    harness
        .description
        .mcp
        .channel_policies
        .push(lys_home::harness::launch_fields::Channel::Wake);
    harness.description.mcp.transports.clear();
    let refused = lys_identity_server::launch_fields::mcp(harness, &version.settings.mcp_servers)
        .err()
        .ok_or("undeclared transport admitted")?;
    assert!(refused.to_string().contains("transports"), "{refused}");
    Ok(())
}

#[test]
fn permission_modes_come_from_the_description() -> Result<(), Box<dyn Error>> {
    use lys_identity_server::launch_permissions::{Permissions, checked};
    let version = version("Claude Code")?;
    let mut contract = version
        .settings
        .harness
        .ok_or("harness missing")?
        .description
        .permissions;
    let given = Permissions {
        default_mode: Some("operator-mode".to_owned()),
        ..Permissions::default()
    };
    let refused = checked(given.clone(), &contract)
        .err()
        .ok_or("undeclared mode admitted")?;
    assert!(refused.to_string().contains("operator-mode"), "{refused}");
    contract.modes.push("operator-mode".to_owned());
    assert_eq!(checked(given.clone(), &contract)?, given);
    Ok(())
}
