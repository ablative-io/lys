//! Native process inputs preserve the profile's recorded choices.

use std::error::Error;

use lys_home::harness::launch_fields::{DeclaredHarness, LaunchFields, LaunchIdentity};
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
    let launch =
        rendering_launch::render("codex/template-v1", &fields.harness.program, &rendered.text)?;
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
    assert!(launch.files.iter().any(|file| file.path == "config.toml"));
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
