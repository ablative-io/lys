//! The native first-run boundary is carried into the settings file.
use std::error::Error;

use lys_home::harness::launch_fields::InstructionsMode;
use lys_home::harness::rendering_launch::{NO_SETTING_SOURCES, ONLY_GIVEN_MCP, render};
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

#[test]
fn a_claude_template_that_names_no_mode_is_refused_never_rendered_unconfined()
-> Result<(), Box<dyn Error>> {
    let base: Value = serde_json::from_str(include_str!("fixtures/launch/template.json"))?;
    let mut absent_permissions = base.clone();
    absent_permissions["slots"]
        .as_object_mut()
        .ok_or("slots absent")?
        .remove("permissions");
    let mut absent_mode = base.clone();
    absent_mode["slots"]["permissions"] = json!({"deny": ["WebFetch"]});
    let mut empty_mode = base;
    empty_mode["slots"]["permissions"] = json!({"defaultMode": ""});
    for (name, template) in [
        ("no permissions", absent_permissions),
        ("no mode", absent_mode),
        ("empty mode", empty_mode),
    ] {
        let error = render(
            "claude-code/template-v1",
            "/opt/seat/bin/claude",
            &template.to_string(),
            InstructionsMode::Keep,
        )
        .err()
        .ok_or_else(|| format!("{name}: rendered unconfined"))?;
        assert!(
            error.contains("choose how this agent is confined"),
            "{name}: {error}"
        );
    }
    Ok(())
}

#[test]
fn a_named_mode_renders_the_permissions_exactly_as_written() -> Result<(), Box<dyn Error>> {
    let mut template: Value = serde_json::from_str(include_str!("fixtures/launch/template.json"))?;
    let written = json!({"defaultMode": "acceptEdits", "deny": ["WebFetch"]});
    template["slots"]["permissions"] = written.clone();
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
    assert_eq!(settings["permissions"], written);
    assert!(settings.get("sandbox").is_none(), "{settings}");
    Ok(())
}

/// A person's own settings, hooks, plugins, MCP servers and connectors are
/// left out of every Claude Code start, in every prompt mode, and the two
/// files Lys passes keep the positions the runner binds to paths.
#[test]
fn every_start_leaves_the_persons_own_setup_out() -> Result<(), Box<dyn Error>> {
    let mut template: Value = serde_json::from_str(include_str!("fixtures/launch/template.json"))?;
    template["slots"]["permissions"] = json!({"defaultMode": "acceptEdits"});
    // The fixture's own flags name one of the two; it is still written once.
    assert_eq!(template["flags"], json!([ONLY_GIVEN_MCP]));
    for mode in [
        InstructionsMode::Keep,
        InstructionsMode::Append,
        InstructionsMode::Replace,
    ] {
        let launch = render(
            "claude-code/template-v1",
            "/opt/seat/bin/claude",
            &template.to_string(),
            mode,
        )?;
        for flag in [NO_SETTING_SOURCES, ONLY_GIVEN_MCP] {
            let count = launch.arguments.iter().filter(|one| *one == flag).count();
            assert_eq!(count, 1, "{flag} in {:?}", launch.arguments);
        }
        assert_eq!(NO_SETTING_SOURCES, "--setting-sources=");
        assert!(
            !launch.arguments.iter().any(String::is_empty),
            "an empty argument: {:?}",
            launch.arguments
        );
        for (index, path) in &launch.argument_files {
            assert_eq!(&launch.arguments[*index], path, "{:?}", launch.arguments);
        }
        assert_eq!(
            launch.arguments[..4],
            ["--mcp-config", "mcp.json", "--settings", "settings.json"]
        );
        assert!(
            !launch.environment.contains_key("CLAUDE_CONFIG_DIR"),
            "a config folder would sign the run out"
        );
    }
    Ok(())
}
