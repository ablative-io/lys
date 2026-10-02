//! Native process inputs preserve the profile's recorded choices.

use std::error::Error;

use lys_home::harness::launch_fields::{
    Channel, DeclaredHarness, EnvText, HandleEnv, InstructionsMode, LaunchFields, LaunchIdentity,
    LaunchMcp, Transport,
};
use lys_home::harness::rendering::{registered, render};
use lys_home::harness::rendering_launch;
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

fn fields() -> Result<LaunchFields, Box<dyn Error>> {
    let catalogue: Value =
        serde_json::from_str(include_str!("../../../docs/harness/catalogue/codex.json"))?;
    Ok(LaunchFields {
        identity: LaunchIdentity {
            agent: "agent-fixture".to_owned(),
            session: "session-fixture".to_owned(),
            machine: "machine-fixture".to_owned(),
            version: 1,
        },
        harness: DeclaredHarness {
            name: "An operator label".to_owned(),
            description: serde_json::from_value(catalogue["description"].clone())?,
            program: "/opt/fixture/codex".to_owned(),
            package: "codex-fixture".to_owned(),
        },
        instructions: "Keep the quoted \"words\".\nThen continue.".to_owned(),
        models: vec!["gpt-6.1-sol".to_owned()],
        mcp_servers: Vec::new(),
        skills: Vec::new(),
    })
}

fn permissions(mode: &str) -> Value {
    json!({"allow": [], "deny": [], "ask": [], "hard_rules": [],
        "additional_directories": [], "default_mode": mode})
}

#[test]
fn codex_contract_is_registered_by_identifier() {
    assert!(registered("codex/template-v1"));
    assert!(!registered("Codex"));
}

#[test]
fn codex_native_launch_preserves_model_sandbox_and_instruction_text() -> TestResult {
    let fields = fields()?;
    let rendered = render(&fields, &[], &permissions("workspace-write"), &[])?;
    assert_eq!(rendered.harness, "codex");
    let launch = rendering_launch::render(
        "codex/template-v1",
        &fields.harness.program,
        &rendered.text,
        InstructionsMode::Append,
    )?;
    assert_eq!(launch.program, fields.harness.program);
    for pair in [
        ["--model", "gpt-6.1-sol"],
        ["--sandbox", "workspace-write"],
        ["--ask-for-approval", "on-request"],
        ["-C", "."],
    ] {
        assert!(launch.arguments.windows(2).any(|args| args == pair));
    }
    let encoded = format!(
        "developer_instructions={}",
        serde_json::to_string(&fields.instructions)?
    );
    assert!(
        launch
            .arguments
            .windows(2)
            .any(|args| args == ["-c", &encoded])
    );
    assert_eq!(launch.environment["LYS_AGENT"], "agent-fixture");
    assert_eq!(launch.environment_paths["CODEX_HOME"], "");
    let file = launch
        .files
        .iter()
        .find(|file| file.path == "config.toml")
        .ok_or("native config missing")?;
    let config: toml::Table = file.text.parse()?;
    assert_eq!(
        config["sandbox_workspace_write"]["network_access"].as_bool(),
        Some(false)
    );
    assert_eq!(
        config["sandbox_workspace_write"]["exclude_tmpdir_env_var"].as_bool(),
        Some(true)
    );
    assert_eq!(
        config["sandbox_workspace_write"]["exclude_slash_tmp"].as_bool(),
        Some(true)
    );
    assert_eq!(config["web_search"].as_str(), Some("disabled"));
    Ok(())
}

#[test]
fn codex_refuses_rules_and_further_models_by_member() -> TestResult {
    let mut fields = fields()?;
    let mut granted = permissions("read-only");
    granted["hard_rules"] = json!([{"id": "rule-fixture"}]);
    let error = render(&fields, &[], &granted, &[])
        .err()
        .ok_or("a hard rule was dropped")?;
    assert_eq!(error.member, "permissions");
    fields.models.push("gpt-6-astra".to_owned());
    let error = render(&fields, &[], &permissions("read-only"), &[])
        .err()
        .ok_or("a further model was dropped")?;
    assert_eq!(error.member, "models");
    Ok(())
}

#[test]
fn codex_instruction_modes_keep_append_and_replace_without_mixing() -> TestResult {
    let fields = fields()?;
    let rendered = render(&fields, &[], &permissions("read-only"), &[])?;
    for mode in [
        InstructionsMode::Keep,
        InstructionsMode::Append,
        InstructionsMode::Replace,
    ] {
        let launch = rendering_launch::render(
            "codex/template-v1",
            &fields.harness.program,
            &rendered.text,
            mode,
        )?;
        let added = launch
            .arguments
            .iter()
            .filter(|arg| arg.starts_with("developer_instructions="))
            .count();
        let replaced = launch
            .arguments
            .iter()
            .filter(|arg| arg.starts_with("model_instructions_file="))
            .count();
        assert_eq!(added, usize::from(mode == InstructionsMode::Append));
        assert_eq!(replaced, usize::from(mode == InstructionsMode::Replace));
        if mode == InstructionsMode::Replace {
            let file = launch
                .files
                .iter()
                .find(|file| file.path == "instructions.txt")
                .ok_or("replacement file missing")?;
            assert_eq!(file.text, fields.instructions);
            let binding = launch
                .arguments
                .iter()
                .find_map(|arg| arg.strip_prefix("model_instructions_file="))
                .ok_or("replacement binding missing")?;
            let parsed: toml::Table = format!("path = {binding}").parse()?;
            assert_eq!(parsed["path"].as_str(), Some(file.path.as_str()));
        }
    }
    Ok(())
}

#[test]
fn codex_blank_add_is_omitted_and_blank_replace_is_refused() -> TestResult {
    let mut fields = fields()?;
    fields.instructions = " \n\t".to_owned();
    let rendered = render(&fields, &[], &permissions("read-only"), &[])?;
    let launch = rendering_launch::render(
        "codex/template-v1",
        &fields.harness.program,
        &rendered.text,
        InstructionsMode::Append,
    )?;
    assert!(
        !launch
            .arguments
            .iter()
            .any(|arg| arg.starts_with("developer_instructions="))
    );
    let error = rendering_launch::render(
        "codex/template-v1",
        &fields.harness.program,
        &rendered.text,
        InstructionsMode::Replace,
    )
    .err()
    .ok_or("blank Replace accepted")?;
    assert!(error.starts_with("instructions:"));
    Ok(())
}

#[test]
fn codex_native_mcp_configuration_preserves_each_transport_and_handle() -> TestResult {
    let mut fields = fields()?;
    fields.mcp_servers = vec![
        LaunchMcp {
            name: "http.service".to_owned(),
            transport: Transport::Http {
                url: "https://example.test/mcp".to_owned(),
            },
            channel: Channel::Off,
        },
        LaunchMcp {
            name: "command".to_owned(),
            transport: Transport::Stdio {
                program: "/opt/fixture/mcp".to_owned(),
                args: vec!["quoted \"text\"\nnext".to_owned()],
                cwd: Some("/opt/fixture/work".to_owned()),
                env: vec![EnvText {
                    name: "SETTING".to_owned(),
                    text: "a\u{7f}\"b\n".to_owned(),
                }],
                handles: vec![HandleEnv {
                    name: "TOKEN".to_owned(),
                    handle_id: "handle-fixture".to_owned(),
                }],
            },
            channel: Channel::Off,
        },
    ];
    let rendered = render(&fields, &[], &permissions("workspace-write"), &[])?;
    let launch = rendering_launch::render(
        "codex/template-v1",
        &fields.harness.program,
        &rendered.text,
        InstructionsMode::Append,
    )?;
    let file = launch
        .files
        .iter()
        .find(|file| file.path == "config.toml")
        .ok_or("native config missing")?;
    let config: toml::Table = file.text.parse()?;
    assert_eq!(
        config["mcp_servers"]["http.service"]["url"].as_str(),
        Some("https://example.test/mcp")
    );
    let command = &config["mcp_servers"]["command"];
    assert_eq!(command["command"].as_str(), Some("/opt/fixture/mcp"));
    assert_eq!(command["cwd"].as_str(), Some("/opt/fixture/work"));
    assert_eq!(command["args"][0].as_str(), Some("quoted \"text\"\nnext"));
    assert_eq!(command["env"]["SETTING"].as_str(), Some("a\u{7f}\"b\n"));
    assert_eq!(command["env"]["TOKEN"].as_str(), Some("handle-fixture"));
    let binding = launch
        .arguments
        .iter()
        .find_map(|arg| arg.strip_prefix("developer_instructions="))
        .ok_or("Add binding missing")?;
    let parsed: toml::Table = format!("instructions = {binding}").parse()?;
    assert_eq!(
        parsed["instructions"].as_str(),
        Some(fields.instructions.as_str())
    );
    Ok(())
}

#[test]
fn codex_kept_template_refuses_unknown_members_and_invalid_native_values() -> TestResult {
    let fields = fields()?;
    let rendered = render(&fields, &[], &permissions("read-only"), &[])?;
    let original: Value = serde_json::from_str(&rendered.text)?;
    for (pointer, value) in [
        ("/harness", json!("claude-code")),
        ("/fields/models", json!([])),
        ("/permissions/default_mode", json!("unknown")),
    ] {
        let mut invalid = original.clone();
        *invalid
            .pointer_mut(pointer)
            .ok_or("fixture pointer missing")? = value;
        let text = serde_json::to_string(&invalid)?;
        assert!(
            rendering_launch::render(
                "codex/template-v1",
                &fields.harness.program,
                &text,
                InstructionsMode::Keep
            )
            .is_err()
        );
    }
    let mut unknown = original;
    unknown["extra"] = json!(true);
    let text = serde_json::to_string(&unknown)?;
    assert!(
        rendering_launch::render(
            "codex/template-v1",
            &fields.harness.program,
            &text,
            InstructionsMode::Keep
        )
        .is_err()
    );
    Ok(())
}

#[test]
fn codex_preserves_all_sandbox_choices_and_additional_directories() -> TestResult {
    let fields = fields()?;
    for mode in ["read-only", "workspace-write", "danger-full-access"] {
        let mut granted = permissions(mode);
        granted["additional_directories"] = json!(["/opt/fixture/extra"]);
        let rendered = render(&fields, &[], &granted, &[])?;
        let launch = rendering_launch::render(
            "codex/template-v1",
            &fields.harness.program,
            &rendered.text,
            InstructionsMode::Keep,
        )?;
        assert!(
            launch
                .arguments
                .windows(2)
                .any(|args| args == ["--sandbox", mode])
        );
        assert!(
            launch
                .arguments
                .windows(2)
                .any(|args| args == ["--add-dir", "/opt/fixture/extra"])
        );
    }
    Ok(())
}

#[test]
fn codex_skills_and_secret_handles_keep_their_hashes_and_bindings() -> TestResult {
    use lys_home::harness::rendering::SecretBinding;
    use lys_home::harness::skills::{SkillFile, check};
    use lys_home::record::blocks::Hash;

    let mut fields = fields()?;
    let skill = SkillFile {
        name: "fixture".to_owned(),
        text: "fixture text".to_owned(),
        sha256: Hash::of(b"fixture text").as_str().to_owned(),
    };
    fields.skills.push(check(&skill)?);
    let rendered = render(
        &fields,
        &[skill],
        &permissions("read-only"),
        &[SecretBinding {
            env: "FIXTURE_TOKEN".to_owned(),
            handle: "handle-fixture".to_owned(),
        }],
    )?;
    assert_eq!(Hash::of(rendered.text.as_bytes()).as_str(), rendered.sha256);
    let launch = rendering_launch::render(
        "codex/template-v1",
        &fields.harness.program,
        &rendered.text,
        InstructionsMode::Keep,
    )?;
    assert_eq!(launch.environment["FIXTURE_TOKEN"], "handle-fixture");
    let file = launch
        .files
        .iter()
        .find(|file| file.path == "skills/fixture/SKILL.md")
        .ok_or("skill missing")?;
    assert_eq!(file.text, "fixture text");
    for file in &launch.files {
        assert_eq!(Hash::of(file.text.as_bytes()).as_str(), file.sha256);
    }
    let error = render(&fields, &[], &permissions("read-only"), &[])
        .err()
        .ok_or("missing kept skill accepted")?;
    assert_eq!(error.member, "skills");
    Ok(())
}

#[test]
fn codex_refuses_wake_channels_and_colliding_command_variables() -> TestResult {
    let mut fields = fields()?;
    fields.mcp_servers.push(LaunchMcp {
        name: "fixture".to_owned(),
        transport: Transport::Http {
            url: "https://example.test/mcp".to_owned(),
        },
        channel: Channel::Wake,
    });
    let error = render(&fields, &[], &permissions("read-only"), &[])
        .err()
        .ok_or("wake silently dropped")?;
    assert_eq!(error.member, "mcp_servers");
    fields.mcp_servers[0].channel = Channel::Off;
    fields.mcp_servers[0].transport = Transport::Stdio {
        program: "/opt/fixture/mcp".to_owned(),
        args: vec![],
        cwd: None,
        env: vec![EnvText {
            name: "TOKEN".to_owned(),
            text: "literal".to_owned(),
        }],
        handles: vec![HandleEnv {
            name: "TOKEN".to_owned(),
            handle_id: "handle-fixture".to_owned(),
        }],
    };
    let error = render(&fields, &[], &permissions("read-only"), &[])
        .err()
        .ok_or("variable silently overwritten")?;
    assert_eq!(error.member, "mcp_servers");
    Ok(())
}
