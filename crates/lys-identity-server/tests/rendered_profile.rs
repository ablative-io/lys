#![cfg(test)]
//! Reviewed native process inputs carry every profile member and replay by version.

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
            "permissions": {"default_mode": "plan"},
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

#[test]
fn the_signed_launch_carries_the_full_profile_and_replays_exactly() -> Result<(), Box<dyn Error>> {
    use lys_home::harness::skills::SkillFile;
    use lys_home::record::blocks::Hash;
    use lys_identity_server::launch_api::kept_launch;
    use lys_identity_server::launch_template::HandleName;
    use lys_identity_server::provisioning_store::ProvisioningStore;
    let skill = SkillFile {
        name: "checked".to_owned(),
        text: "Read the whole change.\n".to_owned(),
        sha256: Hash::of(b"Read the whole change.\n").as_str().to_owned(),
    };
    let version: Version = serde_json::from_value(json!({
        "number": 1, "operation": "profile-one", "set_by": "operator", "set_at": 1,
        "reviewed": {"operation": "review-one", "by": "reviewer", "at": 2},
        "settings": {
            "harness": harness_description::declared(),
            "model_access": ["primary-model", "fallback-model"], "tools": ["Edit"],
            "skills": [skill.name], "skill_pins": [{"name": skill.name, "len": skill.text.len(), "sha256": skill.sha256}],
            "mcp_servers": [{"name": "declared-server", "command": {"program": "/opt/server", "args": ["--serve"],
                "env": {"LITERAL": true, "TOKEN": {"handle": "server-token"}}}, "channel": "wake"}],
            "permissions": {"allow": ["Read"], "deny": ["Bash(rm *)"], "default_mode": "plan"},
            "instructions": "Follow these exact instructions.\n", "note": ""
        }
    }))?;
    let handles = vec![HandleName {
        id: "h-kept".to_owned(),
        secret: "server-token".to_owned(),
        env: "LYS_HANDLE_SERVER_TOKEN".to_owned(),
    }];
    let rendered = render(
        &Start {
            agent: "agent",
            session: "session",
            machine: "machine",
            runtime: "unused-runtime",
            version: &version,
            skills: std::slice::from_ref(&skill),
            policy: None,
        },
        &handles,
    )?;
    let view = json!({"agent": "agent", "session": "session", "provisioning_version": 1,
        "template": rendered.template, "template_sha256": rendered.template_sha256, "handles": handles});
    let dir = tempfile::tempdir()?;
    let mut store = ProvisioningStore::open(&dir.path().join("profiles.json"))?;
    store.set("agent", 0, version.clone())?;
    let first = kept_launch(&store, &view)?;
    let bytes = serde_json::to_vec(&first)?;
    assert_eq!(bytes, serde_json::to_vec(&kept_launch(&store, &view)?)?);
    assert_eq!(first.program, "/opt/seat/bin/claude");
    assert!(
        first
            .arguments
            .windows(2)
            .any(|args| args == ["--model", "primary-model"])
    );
    assert!(
        first
            .arguments
            .windows(2)
            .any(|args| args == ["--fallback-model", "fallback-model"])
    );
    assert!(
        first
            .arguments
            .windows(2)
            .any(|args| args == ["--channels", "server:declared-server"])
    );
    assert_eq!(
        first
            .environment
            .get("LYS_HANDLE_SERVER_TOKEN")
            .map(String::as_str),
        Some("h-kept")
    );
    let config = first.config.as_ref().ok_or("no config")?;
    let file = |name: &str| {
        config
            .files
            .iter()
            .find(|file| file.path == name)
            .ok_or("no native file")
    };
    let settings: serde_json::Value = serde_json::from_str(&file("settings.json")?.text)?;
    assert_eq!(settings["permissions"]["allow"], json!(["Read", "Edit"]));
    assert_eq!(settings["permissions"]["deny"], json!(["Bash(rm *)"]));
    assert_eq!(settings["permissions"]["defaultMode"], "plan");
    let mcp: serde_json::Value = serde_json::from_str(&file("mcp.json")?.text)?;
    assert_eq!(
        mcp["mcpServers"]["declared-server"]["env"]["TOKEN"],
        "h-kept"
    );
    assert_eq!(
        mcp["mcpServers"]["declared-server"]["env"]["LITERAL"],
        "true"
    );
    assert_eq!(
        file("instructions.txt")?.text,
        version.settings.instructions
    );
    assert_eq!(file("skills/checked/SKILL.md")?.text, skill.text);
    let mut newer = version;
    newer.operation = "profile-two".to_owned();
    newer.settings.instructions = "Different instructions.".to_owned();
    newer.settings.instructions_mode = lys_home::harness::launch_fields::InstructionsMode::Replace;
    newer.settings.harness.as_mut().ok_or("no harness")?.program = "/opt/new/program".to_owned();
    store.set("agent", 1, newer)?;
    assert_eq!(bytes, serde_json::to_vec(&kept_launch(&store, &view)?)?);
    let mut missing = view.clone();
    missing["provisioning_version"] = json!(99);
    assert!(matches!(
        kept_launch(&store, &missing),
        Err(lys_identity_server::error::ServerError::ProfileVersionUnknown { version: 99 })
    ));
    let mut corrupt = view;
    corrupt["template"] = json!("changed bytes");
    assert!(matches!(
        kept_launch(&store, &corrupt),
        Err(lys_identity_server::error::ServerError::LaunchUnrenderable { .. })
    ));
    Ok(())
}

#[test]
fn instructions_mode_renders_only_the_reviewed_claude_prompt_flag() -> Result<(), Box<dyn Error>> {
    use lys_identity_server::launch_template::from_template;
    for (mode, expected) in [
        ("keep", None),
        ("append", Some("--append-system-prompt-file")),
        ("replace", Some("--system-prompt-file")),
    ] {
        let version: Version = serde_json::from_value(json!({
            "number": 1, "operation": "record-profile", "set_by": "operator", "set_at": 1,
            "settings": {
                "harness": harness_description::declared(),
                "model_access": ["primary-model"], "tools": [], "skills": [], "mcp_servers": [],
                "permissions": {"default_mode": "plan"},
                "instructions": "Exact reviewed instructions.\n", "instructions_mode": mode, "note": ""
            }
        }))?;
        let rendered = render(
            &Start {
                agent: "agent",
                session: "session",
                machine: "machine",
                runtime: "unused",
                version: &version,
                skills: &[],
                policy: None,
            },
            &[],
        )?;
        let launch = from_template(&version, &rendered.template, &rendered.template_sha256)?;
        let prompt_flags: Vec<&str> = launch
            .arguments
            .iter()
            .map(String::as_str)
            .filter(|argument| argument.ends_with("system-prompt-file"))
            .collect();
        assert_eq!(
            prompt_flags,
            expected.into_iter().collect::<Vec<_>>(),
            "{mode}"
        );
        let instruction_bindings: Vec<usize> = launch
            .argument_files
            .iter()
            .filter_map(|(index, path)| (path == "instructions.txt").then_some(*index))
            .collect();
        match expected {
            Some(flag) => {
                assert_eq!(instruction_bindings.len(), 1, "{mode}");
                let index = instruction_bindings[0];
                assert_eq!(launch.arguments[index - 1], flag);
                assert_eq!(launch.arguments[index], "instructions.txt");
                assert_eq!(
                    launch
                        .files
                        .iter()
                        .find(|file| file.path == "instructions.txt")
                        .ok_or("no instructions file")?
                        .text,
                    version.settings.instructions
                );
            }
            None => assert!(instruction_bindings.is_empty()),
        }
    }
    Ok(())
}
