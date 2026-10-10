//! HOME-037 R2: every model a profile lists is carried or refused by name.
//! Claude Code starts on the first and falls back through the rest in
//! order; a model the declared harness cannot carry is refused when the
//! profile is recorded, and nothing is reported left out.

#[path = "support/runner_start.rs"]
pub mod support;
use support::{Table, operation};

use std::error::Error;

use lys_home::harness::claude_code::template::parse_template;
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

impl Table {
    /// Record a profile of `models` declaring `harness`, from `from`.
    async fn record(
        &self,
        from: u32,
        harness: &Value,
        models: &Value,
    ) -> Result<(u16, Value), Box<dyn Error>> {
        let body = json!({
            "operation": operation()?, "from_version": from, "working_folder": "/tmp",
            "model_access": models, "tools": [], "skills": [],
            "mcp_servers": [], "instructions": "", "note": "", "permissions": {"default_mode": "plan"}, "harness": harness,
        });
        let path = format!("/agents/{}/provisioning", self.agent());
        self.service.post(&path, Some(&self.ada), &body).await
    }

    /// Record `models` for `harness`, review them, and ask for a start on the table's machine.
    async fn start_profile(
        &self,
        from: u32,
        harness: &Value,
        models: &Value,
    ) -> Result<(u16, Value), Box<dyn Error>> {
        let (status, set) = self.record(from, harness, models).await?;
        assert_eq!(status, 200, "{set}");
        let path = format!("/agents/{}/provisioning/{}/review", self.agent(), from + 1);
        let reviewed = json!({ "operation": operation()? });
        let (status, set) = self.service.post(&path, Some(&self.ada), &reviewed).await?;
        assert_eq!(status, 200, "{set}");
        let machine = self.lys_machine(&format!("Box {from}")).await?;
        let path = format!("/agents/{}/start-command", self.agent());
        let body = json!({ "machine": machine, "operation": operation()? });
        self.service.post(&path, Some(&self.ada), &body).await
    }
}

fn harness(table: &Table, kind: &str) -> Value {
    let mut harness = table.harness();
    harness["name"] = json!(kind);
    if kind == "codex" {
        harness["description"]["models"]["maximum"] = json!(1);
        harness["description"]["models"]["further_encoding"] = json!({"kind": "array"});
        harness["description"]["rendering_contract"] = json!("codex/template-v1");
    }
    harness
}

#[tokio::test]
async fn three_models_start_claude_code_on_the_first_and_fall_back_through_the_rest() -> TestResult
{
    let table = Table::unprofiled().await?;
    let models = json!(["claude-fable-5-1", "claude-opus-5-5", "claude-sonnet-5"]);
    let (status, start) = table
        .start_profile(0, &harness(&table, "claude_code"), &models)
        .await?;
    assert_eq!(status, 200, "{start}");
    let template = parse_template(start["template"].as_str().ok_or("no template")?.as_bytes())?;
    let at = template
        .flags
        .iter()
        .position(|flag| flag == "--model")
        .ok_or("no --model")?;
    assert_eq!(
        template.flags[at..at + 4],
        [
            "--model",
            "claude-fable-5-1",
            "--fallback-model",
            "claude-opus-5-5,claude-sonnet-5"
        ]
    );
    assert_eq!(start["left_out"], json!([]));
    table.close()
}

#[tokio::test]
async fn a_model_the_declared_harness_cannot_carry_is_refused_when_recorded() -> TestResult {
    let table = Table::unprofiled().await?;
    let models = json!(["gpt-seat-1", "gpt-seat-2"]);
    let (status, refused) = table.record(0, &harness(&table, "codex"), &models).await?;
    assert_eq!(status, 400, "{refused}");
    assert_eq!(refused["refusal"], "ModelUnrepresentable");
    assert!(
        refused["reason"]
            .as_str()
            .is_some_and(|reason| reason.contains("gpt-seat-2"))
    );
    let listed = json!(["claude-fable-5-1", "claude-opus-5-5,claude-sonnet-5"]);
    let (status, refused) = table
        .record(0, &harness(&table, "claude_code"), &listed)
        .await?;
    assert_eq!(status, 400, "{refused}");
    assert_eq!(refused["refusal"], "ModelUnrepresentable");
    let (status, refused) = table
        .record(0, &harness(&table, "codex"), &json!([]))
        .await?;
    assert_eq!(status, 400, "{refused}");
    assert_eq!(
        refused["refusal"], "ModelUnrepresentable",
        "a Codex profile names its model"
    );
    let (status, set) = table
        .record(0, &harness(&table, "codex"), &json!(["gpt-seat-1"]))
        .await?;
    assert_eq!(status, 200, "{set}");
    table.close()
}
