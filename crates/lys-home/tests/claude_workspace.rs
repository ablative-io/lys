//! The native first-run boundary is carried into the settings file.
use std::error::Error;

use lys_home::harness::launch_fields::InstructionsMode;
use lys_home::harness::rendering_launch::render;
use serde_json::{Value, json};

#[test]
fn workspace_only_renders_claude_sandbox_and_file_permissions() -> Result<(), Box<dyn Error>> {
    let mut template: Value = serde_json::from_str(include_str!("fixtures/launch/template.json"))?;
    template["slots"]["permissions"] = json!({"defaultMode": "workspace-only"});
    let launch = render(
        "claude-code/template-v1",
        "/opt/seat/bin/claude",
        &template.to_string(),
        InstructionsMode::Keep,
    )?;
    let settings: Value = serde_json::from_str(
        &launch
            .files
            .iter()
            .find(|file| file.path == "settings.json")
            .ok_or("settings absent")?
            .text,
    )?;
    assert_eq!(settings["permissions"]["defaultMode"], "dontAsk");
    assert_eq!(
        settings["permissions"]["allow"],
        json!(["Read(./**)", "Edit(./**)"])
    );
    assert_eq!(
        settings["permissions"]["deny"],
        json!(["WebFetch", "WebSearch"])
    );
    assert_eq!(
        settings["permissions"]["blockReadsOutsideWorkingDirectories"],
        true
    );
    assert_eq!(settings["sandbox"]["enabled"], true);
    assert_eq!(settings["sandbox"]["failIfUnavailable"], true);
    assert_eq!(settings["sandbox"]["allowUnsandboxedCommands"], false);
    assert_eq!(settings["sandbox"]["network"]["allowedDomains"], json!([]));
    assert_eq!(settings["sandbox"]["network"]["strictAllowlist"], true);
    Ok(())
}

#[test]
fn workspace_only_refuses_an_extra_allowed_tool_or_directory() -> Result<(), Box<dyn Error>> {
    for field in ["allow", "additionalDirectories"] {
        let mut template: Value =
            serde_json::from_str(include_str!("fixtures/launch/template.json"))?;
        template["slots"]["permissions"] =
            json!({"defaultMode": "workspace-only", field: ["/elsewhere"]});
        let error = render(
            "claude-code/template-v1",
            "/opt/seat/bin/claude",
            &template.to_string(),
            InstructionsMode::Keep,
        )
        .err()
        .ok_or("wider workspace admitted")?;
        assert!(
            error.contains(&format!("slots.permissions.{field}")),
            "{error}"
        );
    }
    Ok(())
}
