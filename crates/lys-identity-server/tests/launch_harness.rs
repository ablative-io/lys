//! HOME-037 R1: the harness and its build are declared in the profile.
//! A profile records the declared kind, program and package and gives them
//! back as recorded; a start from a profile that declares none is refused
//! by name before anything is rendered.

#[path = "support/runner_start.rs"]
pub mod support;
use support::{Table, operation};

use std::error::Error;

use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

impl Table {
    /// Record a profile declaring `harness`, or none when it is null, from `from`.
    async fn record(&self, from: u32, harness: &Value) -> Result<(u16, Value), Box<dyn Error>> {
        let mut body = json!({
            "operation": operation()?, "from_version": from,
            "model_access": ["claude-fable-5-1"], "tools": [], "skills": [],
            "mcp_servers": [], "instructions": "", "note": "",
            "permissions": {"default_mode": "plan"},
        });
        if !harness.is_null() {
            body["harness"] = harness.clone();
        }
        let path = format!("/agents/{}/provisioning", self.agent());
        self.service.post(&path, Some(&self.ada), &body).await
    }

    /// Record `harness`, review it, and ask for a start on a new machine.
    async fn start_profile(
        &self,
        from: u32,
        harness: &Value,
    ) -> Result<(u16, Value), Box<dyn Error>> {
        let (status, set) = self.record(from, harness).await?;
        assert_eq!(status, 200, "{set}");
        let path = format!("/agents/{}/provisioning/{}/review", self.agent(), from + 1);
        let reviewed = json!({ "operation": operation()? });
        let (status, set) = self.service.post(&path, Some(&self.ada), &reviewed).await?;
        assert_eq!(status, 200, "{set}");
        let machine = operation()?;
        let body = json!({
            "operation": machine, "name": format!("Box {from}"), "kind": "server",
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

#[tokio::test]
async fn a_profile_records_its_declared_build_and_refuses_one_it_cannot_verify() -> TestResult {
    let table = Table::unprofiled().await?;
    let (status, set) = table.record(0, &table.harness()).await?;
    assert_eq!(status, 200, "{set}");
    assert_eq!(set["profile"]["harness"], table.harness());
    let mut relative = table.harness();
    relative["program"] = json!("bin/claude");
    let (status, refused) = table.record(1, &relative).await?;
    assert_eq!(
        (status, &refused["refusal"]),
        (400, &json!("RequestMalformed")),
        "{refused}"
    );
    let mut unnamed = table.harness();
    unnamed["package"] = json!(" ");
    let (status, refused) = table.record(1, &unnamed).await?;
    assert_eq!(
        (status, &refused["refusal"]),
        (400, &json!("RequestMalformed")),
        "{refused}"
    );
    table.close()
}

#[tokio::test]
async fn a_start_from_a_profile_with_no_declared_harness_is_refused_by_name() -> TestResult {
    let table = Table::unprofiled().await?;
    let (status, refused) = table.start_profile(0, &Value::Null).await?;
    assert_eq!(status, 409, "{refused}");
    assert_eq!(refused["refusal"], "HarnessUndeclared");
    let (status, start) = table.start_profile(1, &table.harness()).await?;
    assert_eq!(status, 200, "{start}");
    let mut codex = table.harness();
    codex["name"] = json!("Unregistered build");
    codex["description"]["rendering_contract"] = json!("unregistered/template-v1");
    let (status, refused) = table.start_profile(2, &codex).await?;
    assert_eq!(status, 409, "{refused}");
    assert_eq!(
        refused["refusal"], "LaunchUnrenderable",
        "no Codex start is rendered as Claude Code"
    );
    table.close()
}
