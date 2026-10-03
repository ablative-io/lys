//! A Claude Code run is given Lys's model proxy as its base URL, and a
//! profile's own base is refused by name rather than replaced.

use std::error::Error;

use lys_home::harness::launch_fields::{DeclaredHarness, LaunchFields, LaunchIdentity};
use lys_home::harness::rendering::{PROXY_VARIABLE, SecretBinding, render};
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

const PROXY: &str = "http://127.0.0.1:18484/anthropic";

fn fields(model_proxy: Option<&str>) -> Result<LaunchFields, Box<dyn Error>> {
    let catalogue: Value = serde_json::from_str(include_str!(
        "../../../docs/harness/catalogue/claude-code.json"
    ))?;
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
            program: "/opt/fixture/claude".to_owned(),
            package: "claude-fixture".to_owned(),
        },
        instructions: String::new(),
        models: Vec::new(),
        mcp_servers: Vec::new(),
        skills: Vec::new(),
        model_proxy: model_proxy.map(str::to_owned),
    })
}

fn permissions() -> Value {
    json!({"allow": [], "deny": [], "ask": [], "hard_rules": [],
        "additional_directories": [], "default_mode": "plan"})
}

fn env(text: &str) -> Result<Value, Box<dyn Error>> {
    let template: Value = serde_json::from_str(text)?;
    Ok(template["slots"]["env"].clone())
}

#[test]
fn the_proxy_is_the_run_s_base_url_and_absent_without_one() -> TestResult {
    let plain = render(&fields(None)?, &[], &permissions(), &[])?;
    assert!(env(&plain.text)?.get(PROXY_VARIABLE).is_none());
    let proxied = render(&fields(Some(PROXY))?, &[], &permissions(), &[])?;
    let proxied_env = env(&proxied.text)?;
    assert_eq!(proxied_env[PROXY_VARIABLE], PROXY);
    let mut without = proxied_env.clone();
    without
        .as_object_mut()
        .ok_or("env is not an object")?
        .remove(PROXY_VARIABLE);
    assert_eq!(without, env(&plain.text)?);
    Ok(())
}

#[test]
fn a_profile_naming_its_own_base_is_refused_by_name_under_the_proxy() -> TestResult {
    let own = [SecretBinding {
        env: PROXY_VARIABLE.to_owned(),
        handle: "handle-fixture".to_owned(),
    }];
    let Err(refusal) = render(&fields(Some(PROXY))?, &[], &permissions(), &own) else {
        return Err("a profile's own ANTHROPIC_BASE_URL was replaced by the proxy".into());
    };
    let words = refusal.to_string();
    assert!(words.contains("secrets.ANTHROPIC_BASE_URL"), "{words}");
    assert!(words.contains("would replace it"), "{words}");
    render(&fields(None)?, &[], &permissions(), &own)?;
    Ok(())
}
