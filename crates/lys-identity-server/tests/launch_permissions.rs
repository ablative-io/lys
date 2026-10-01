//! HOME-037 R5: the settings file a Claude Code launch names carries the
//! profile's permissions (allow, deny and ask rules, the permission mode and
//! the additional directories) with its tools allowed and each hard rule of
//! the agent's Tool policy denied, in place of an `--allowedTools` flag. A
//! rule the settings file cannot express is refused by name: a profile's
//! when it is recorded, a policy's by its id when the start is rendered.

#[path = "support/runner_start.rs"]
pub mod support;
use support::{Table, operation};

use std::error::Error;

use lys_home::harness::claude_code::launch_env::env_settings;
use lys_home::harness::claude_code::template::parse_template;
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

impl Table {
    /// Record a Claude Code profile of `tools` and `permissions`, from `from`.
    async fn record(
        &self,
        from: u32,
        tools: &Value,
        permissions: &Value,
    ) -> Result<(u16, Value), Box<dyn Error>> {
        let body = json!({
            "operation": operation()?, "from_version": from,
            "model_access": ["claude-fable-5-1"], "tools": tools, "skills": [],
            "mcp_servers": [], "instructions": "", "note": "", "permissions": permissions,
            "harness": self.harness(),
        });
        let path = format!("/agents/{}/provisioning", self.agent());
        self.service.post(&path, Some(&self.ada), &body).await
    }

    /// Keep `rules` as the agent's Tool policy.
    async fn policy(&self, rules: &Value) -> TestResult {
        let path = format!("/agents/{}/policy", self.agent());
        let body = json!({ "version": 1, "rules": rules });
        let (status, kept) = self.service.post(&path, Some(&self.ada), &body).await?;
        assert_eq!(status, 200, "{kept}");
        Ok(())
    }

    /// Review version `number` so it may start.
    async fn review(&self, number: u32) -> TestResult {
        let path = format!("/agents/{}/provisioning/{number}/review", self.agent());
        let reviewed = json!({ "operation": operation()? });
        let (status, set) = self.service.post(&path, Some(&self.ada), &reviewed).await?;
        assert_eq!(status, 200, "{set}");
        Ok(())
    }

    /// Ask for a start on a new machine named `name`.
    async fn start_profile(&self, name: &str) -> Result<(u16, Value), Box<dyn Error>> {
        let machine = operation()?;
        let body = json!({
            "operation": machine, "name": name, "kind": "server",
            "runtime": "manifold", "slots": 1, "may_run": [self.agent()], "may_reach": [],
        });
        let (status, named) = self
            .service
            .post("/network/machines", Some(&self.ada), &body)
            .await?;
        assert_eq!(status, 200, "{named}");
        self.ok(
            &format!("/network/machines/{machine}/runner"),
            &json!({"runner": {"kind": "lys"}}),
        )
        .await?;
        let path = format!("/agents/{}/start-command", self.agent());
        let body = json!({ "machine": machine, "operation": operation()? });
        self.service.post(&path, Some(&self.ada), &body).await
    }
}

fn hard(id: &str, tool: &str, kind: &str, target: Option<&str>) -> Value {
    let mut rule = json!({ "id": id, "tool": tool, "kind": kind, "authority": "hard" });
    if let Some(target) = target {
        rule["target"] = json!(target);
    }
    rule
}

fn permissions() -> Value {
    json!({
        "allow": ["Read"], "deny": ["WebFetch(domain:evil.example)"], "ask": ["Edit"],
        "default_mode": "acceptEdits", "additional_directories": ["/srv/a", "/srv/b"],
    })
}

#[tokio::test]
async fn the_settings_file_carries_the_union_of_the_profile_and_the_policy_twice_identical()
-> TestResult {
    let table = Table::unprofiled().await?;
    table
        .policy(&json!([hard(
            "no-keys",
            "Read",
            "path_prefix",
            Some("/srv/keys")
        )]))
        .await?;
    let (status, set) = table.record(0, &json!(["Bash"]), &permissions()).await?;
    assert_eq!(status, 200, "{set}");
    assert_eq!(set["profile"]["permissions"]["default_mode"], "acceptEdits");
    table.review(1).await?;
    let expected = json!({
        "allow": ["Read", "Bash"],
        "deny": ["WebFetch(domain:evil.example)", "Read(//srv/keys)", "Read(//srv/keys/**)"],
        "ask": ["Edit"],
        "additionalDirectories": ["/srv/a", "/srv/b"],
        "defaultMode": "acceptEdits",
    });
    let mut written = Vec::new();
    for name in ["Box one", "Box two"] {
        let (status, started) = table.start_profile(name).await?;
        assert_eq!(status, 200, "{started}");
        let text = started["template"].as_str().ok_or("no template")?;
        let template = parse_template(text.as_bytes())?;
        assert!(
            !template.flags.iter().any(|flag| flag == "--allowedTools"),
            "{:?}",
            template.flags
        );
        let settings: Value = serde_json::from_slice(&env_settings(&template)?)?;
        assert_eq!(settings["permissions"], expected);
        written.push(settings["permissions"].clone());
    }
    assert_eq!(written[0], written[1], "two renders write one permissions");
    table.close()
}

#[tokio::test]
async fn a_profile_rule_the_settings_file_cannot_express_is_refused_by_name() -> TestResult {
    let table = Table::unprofiled().await?;
    let cases = [
        (json!({ "deny": ["Bash(rm"] }), "Bash(rm"),
        (json!({ "deny": ["Read("] }), "Read("),
        (json!({ "deny": ["Read()"] }), "Read()"),
        (json!({ "default_mode": "default" }), "default"),
        (json!({ "allow": ["two words"] }), "two words"),
        (json!({ "default_mode": "yolo" }), "yolo"),
        (json!({ "additional_directories": ["srv/a"] }), "srv/a"),
    ];
    for (given, named) in cases {
        let (status, refused) = table.record(0, &json!([]), &given).await?;
        assert_eq!(
            (status, &refused["refusal"]),
            (400, &json!("PolicyUnrepresentable")),
            "{refused}"
        );
        assert!(
            refused["reason"]
                .as_str()
                .is_some_and(|reason| reason.contains(named)),
            "{refused}"
        );
    }
    table.close()
}

#[tokio::test]
async fn a_hard_policy_rule_the_settings_file_cannot_express_refuses_the_start_by_its_id()
-> TestResult {
    let table = Table::unprofiled().await?;
    table
        .policy(&json!([hard(
            "no-denied-writes",
            "Write",
            "path_prefix",
            Some("/probe")
        )]))
        .await?;
    let (status, set) = table.record(0, &json!([]), &json!({})).await?;
    assert_eq!(status, 200, "{set}");
    table.review(1).await?;
    let (status, refused) = table.start_profile("Box one").await?;
    assert_eq!(
        (status, &refused["refusal"]),
        (400, &json!("PolicyUnrepresentable")),
        "{refused}"
    );
    assert!(
        refused["reason"]
            .as_str()
            .is_some_and(|reason| reason.contains("no-denied-writes")),
        "{refused}"
    );
    table.close()
}

#[tokio::test]
async fn a_profile_with_no_permissions_writes_environment_only() -> TestResult {
    let table = Table::unprofiled().await?;
    table.policy(&json!([])).await?;
    let (status, set) = table.record(0, &json!([]), &Value::Null).await?;
    assert_eq!(status, 200, "{set}");
    table.review(1).await?;
    let (status, started) = table.start_profile("Box one").await?;
    assert_eq!(status, 200, "{started}");
    let text = started["template"].as_str().ok_or("no template")?;
    let template = parse_template(text.as_bytes())?;
    let settings: Value = serde_json::from_slice(&env_settings(&template)?)?;
    let members: Vec<&String> = settings.as_object().ok_or("no object")?.keys().collect();
    assert_eq!(members, ["env"]);
    table.close()
}

/// The template a start renders, without the per-start identity in its
/// environment slot (agent, session, machine and version).
fn rendered_without_identity(started: &Value) -> Result<Vec<u8>, Box<dyn Error>> {
    let text = started["template"].as_str().ok_or("no template")?;
    let mut template: Value = serde_json::from_str(text)?;
    let slots = template["slots"]
        .as_object_mut()
        .ok_or("template has no slots")?;
    slots.remove("env").ok_or("template has no env slot")?;
    Ok(serde_json::to_vec_pretty(&template)?)
}

#[tokio::test]
async fn a_tool_and_the_same_allow_rule_render_byte_identical_starts() -> TestResult {
    let table = Table::unprofiled().await?;
    let (status, set) = table.record(0, &json!(["Bash"]), &permissions()).await?;
    assert_eq!(status, 200, "{set}");
    table.review(1).await?;
    let (status, as_tool) = table.start_profile("Tool box").await?;
    assert_eq!(status, 200, "{as_tool}");
    let mut folded = permissions();
    folded["allow"] = json!(["Read", "Bash"]);
    let (status, set) = table.record(1, &json!([]), &folded).await?;
    assert_eq!(status, 200, "{set}");
    table.review(2).await?;
    let (status, as_rule) = table.start_profile("Rule box").await?;
    assert_eq!(status, 200, "{as_rule}");
    assert_eq!(as_tool["provisioning_version"], 1);
    assert_eq!(as_rule["provisioning_version"], 2);
    assert_eq!(
        rendered_without_identity(&as_tool)?,
        rendered_without_identity(&as_rule)?,
        "folding a tool into the allow rules changes nothing the start renders"
    );
    table.close()
}
