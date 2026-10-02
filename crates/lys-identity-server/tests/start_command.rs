#![cfg(test)]

//! The start-command route: an agent's start command is rendered from its
//! kept profile for a chosen machine, names the agent and its handle ids and
//! never a credential's value, runs on its runner, and each refusal is by name.

#[path = "support/runner_start.rs"]
pub mod support;
use support::{PLANTED, Table, operation};

use std::error::Error;

use identity_contract::fake_issuer::Login;
use identity_contract::harness::StageTimer;
use lys_home::harness::claude_code::template::parse_template;
use lys_identity::AgentId;
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

const BEA: &str = "bea-subject";

fn refused(answer: &(u16, Value), status: u16, name: &str) {
    assert_eq!(answer.0, status, "{}", answer.1);
    assert_eq!(answer.1["refusal"], name, "{}", answer.1);
}

impl Table {
    async fn placed_machine(
        &self,
        runtime: Option<&str>,
        may_run: &[String],
    ) -> Result<String, Box<dyn Error>> {
        let id = operation()?;
        let body = json!({
            "operation": id, "name": "Build box", "kind": "server", "runtime": runtime,
            "slots": u32::from(runtime.is_some()), "may_run": may_run, "may_reach": ["cambium.example.test"],
        });
        let (status, named) = self
            .service
            .post("/network/machines", Some(&self.ada), &body)
            .await?;
        assert_eq!(status, 200, "{named}");
        if runtime.is_some() {
            self.ok(
                &format!("/network/machines/{id}/runner"),
                &json!({"runner": {"kind": "lys"}}),
            )
            .await?;
        }
        Ok(id)
    }

    async fn launch_profile(&self) -> TestResult {
        let skill = json!({ "name": "review", "text": "Read the change against its brief.\n" });
        let (status, kept) = self
            .service
            .post("/skills", Some(&self.ada), &skill)
            .await?;
        assert_eq!(status, 200, "{kept}");
        let body = json!({
            "operation": operation()?, "from_version": 0,
            "model_access": ["claude-fable-5-1"], "tools": ["read"], "skills": ["review"],
            "mcp_servers": [{ "name": "cambium", "url": "https://cambium.example.test/mcp" }],
            "harness": self.harness(),
            "instructions": "Build what the brief says.", "note": "First setup.",
            "permissions": {"default_mode": "plan"},
        });
        let path = format!("/agents/{}/provisioning", self.agent());
        let (status, set) = self.service.post(&path, Some(&self.ada), &body).await?;
        assert_eq!(status, 200, "{set}");
        let reviewed = json!({ "operation": operation()? });
        let (status, set) = self
            .service
            .post(&format!("{path}/1/review"), Some(&self.ada), &reviewed)
            .await?;
        assert_eq!(status, 200, "{set}");
        assert!(set["profile"]["reviewed_by"].is_string(), "{set}");
        Ok(())
    }

    async fn ask(
        &self,
        agent: &str,
        machine: &str,
        cookie: &str,
    ) -> Result<(u16, Value), Box<dyn Error>> {
        let path = format!("/agents/{agent}/start-command");
        let body = json!({ "machine": machine, "operation": operation()? });
        self.service.post(&path, Some(cookie), &body).await
    }
}

#[tokio::test]
async fn each_refusal_is_by_name() -> TestResult {
    let timing = StageTimer::new("scenario.total");
    let stage = StageTimer::new("scenario.table_open");
    let table = Table::unprofiled_with(true, Some(BEA)).await?;
    drop(stage);
    let driver = StageTimer::new("scenario.driver");
    let bea = table
        .service
        .sign_in(Login {
            subject: BEA.to_owned(),
            email: "shared@example.test".to_owned(),
        })
        .await?;
    let agent = table.agent();
    let unheld = AgentId::generate()?.to_string();
    let open = table
        .placed_machine(Some("manifold"), std::slice::from_ref(&agent))
        .await?;

    refused(
        &table.ask(&unheld, &open, &table.ada).await?,
        404,
        "AgentNotVisible",
    );
    refused(&table.ask(&agent, &open, &bea).await?, 403, "NotAdmitted");
    refused(
        &table.ask(&agent, &open, &table.ada).await?,
        404,
        "LaunchRecordMissing",
    );

    table.launch_profile().await?;
    refused(
        &table.ask(&agent, &operation()?, &table.ada).await?,
        404,
        "MachineUnknown",
    );
    let bare = table.placed_machine(None, &[]).await?;
    refused(
        &table.ask(&agent, &bare, &table.ada).await?,
        409,
        "MachineWithoutRuntime",
    );
    let elsewhere = table.placed_machine(Some("manifold"), &[]).await?;
    refused(
        &table.ask(&agent, &elsewhere, &table.ada).await?,
        403,
        "MachineNotForAgent",
    );
    let (status, retired) = table
        .service
        .post(
            &format!("/network/machines/{open}/retire"),
            Some(&table.ada),
            &json!({}),
        )
        .await?;
    assert_eq!(status, 200, "{retired}");
    refused(
        &table.ask(&agent, &open, &table.ada).await?,
        409,
        "MachineRetired",
    );

    let (status, none) = table
        .service
        .post(
            &format!("/agents/{agent}/start-command"),
            Some(&table.ada),
            &json!({ "operation": operation()? }),
        )
        .await?;
    assert_eq!(status, 400, "{none}");
    assert_eq!(none["refusal"], "RequestMalformed");
    drop(driver);
    let stage = StageTimer::new("scenario.table_close");
    let result = table.close();
    drop(stage);
    drop(timing);
    result
}

#[tokio::test]
async fn the_command_names_the_agent_and_its_handles_and_never_a_value() -> TestResult {
    let timing = StageTimer::new("scenario.total");
    let stage = StageTimer::new("scenario.table_open");
    let table = Table::unprofiled_with(true, None).await?;
    drop(stage);
    let driver = StageTimer::new("scenario.driver");
    let agent = table.agent();
    table.launch_profile().await?;
    let machine = table
        .placed_machine(Some("manifold"), std::slice::from_ref(&agent))
        .await?;

    let (status, start) = table.ask(&agent, &machine, &table.ada).await?;
    assert_eq!(status, 200, "{start}");
    let whole = start.to_string();
    assert!(
        !whole.contains(PLANTED),
        "a credential value reached the answer: {whole}"
    );
    assert_eq!(start["executed"], false);
    assert_eq!(start["agent"], agent);
    assert_eq!(start["runtime"], "manifold");
    assert_eq!(start["provisioning_version"], 1);
    assert_eq!(
        start["handles"],
        json!([{ "id": "h-live", "secret": "git-host token", "env": "LYS_HANDLE_GIT_HOST_TOKEN" }]),
        "a dropped handle is not named"
    );
    let command = start["command"].as_str().ok_or("no command")?;
    let session = start["session"].as_str().ok_or("no session")?;
    assert!(command.starts_with("env "), "{command}");
    assert!(
        command.contains(
            table.harness()["program"]
                .as_str()
                .ok_or("no declared program")?
        ),
        "{command}"
    );
    assert!(!command.contains("manifold"), "{command}");
    for named in [
        format!("LYS_AGENT={agent}"),
        format!("LYS_SESSION={session}"),
        format!("LYS_MACHINE={machine}"),
        "LYS_HANDLES=h-live".to_owned(),
    ] {
        assert!(command.contains(&named), "{command} does not carry {named}");
    }

    let template = start["template"].as_str().ok_or("no template")?;
    let parsed = parse_template(template.as_bytes())?;
    assert_eq!(start["template_sha256"], parsed.hash.as_str());
    assert!(command.contains(&format!("LYS_LAUNCH_TEMPLATE={}", parsed.hash.as_str())));
    assert_eq!(parsed.use_only.len(), 1);
    assert_eq!(parsed.use_only[0].handle, "h-live");
    assert_eq!(parsed.env.get("LYS_AGENT"), Some(&agent));
    assert_eq!(parsed.instructions, "Build what the brief says.");
    assert_eq!(start["left_out"], json!([]));
    assert_eq!(parsed.skills.len(), 1);
    assert_eq!(
        parsed.skills[0].text,
        "Read the change against its brief.\n"
    );

    let (status, again) = table.ask(&agent, &machine, &table.ada).await?;
    assert_eq!(status, 200, "{again}");
    assert_ne!(
        again["session"], start["session"],
        "each start is its own session"
    );
    drop(driver);
    let stage = StageTimer::new("scenario.table_close");
    let result = table.close();
    drop(stage);
    drop(timing);
    result
}

#[tokio::test]
async fn a_start_is_kept_once_under_its_operation() -> TestResult {
    let timing = StageTimer::new("scenario.total");
    let stage = StageTimer::new("scenario.table_open");
    let table = Table::unprofiled_with(true, None).await?;
    drop(stage);
    let driver = StageTimer::new("scenario.driver");
    let agent = table.agent();
    table.launch_profile().await?;
    let machine = table
        .placed_machine(Some("manifold"), std::slice::from_ref(&agent))
        .await?;
    let path = format!("/agents/{agent}/start-command");
    let body = json!({ "machine": machine, "operation": operation()? });

    let (status, first) = table.service.post(&path, Some(&table.ada), &body).await?;
    assert_eq!(status, 200, "{first}");
    let (status, again) = table.service.post(&path, Some(&table.ada), &body).await?;
    assert_eq!(status, 200, "{again}");
    assert_eq!(first["session"], body["operation"]);
    assert_eq!(again["session"], first["session"]);

    let (status, held) = table
        .service
        .get(
            &format!("/agents/{agent}/runtime/sessions"),
            Some(&table.ada),
        )
        .await?;
    assert_eq!(status, 200, "{held}");
    let sessions = held["sessions"].as_array().ok_or("no sessions")?;
    assert_eq!(sessions.len(), 1, "{held}");
    assert_eq!(sessions[0]["session"], first["session"]);
    assert_eq!(sessions[0]["shown"], "running");
    assert_eq!(first["runner"]["session"], first["session"]);
    assert_eq!(again["runner"]["session"], first["session"]);
    assert_eq!(first["runner"]["state"], "running");
    assert_eq!(again["runner"]["state"], "running");
    drop(driver);
    let stage = StageTimer::new("scenario.table_close");
    let result = table.close();
    drop(stage);
    drop(timing);
    result
}

#[tokio::test]
async fn a_machine_that_cannot_reach_the_profile_is_refused() -> TestResult {
    let timing = StageTimer::new("scenario.total");
    let stage = StageTimer::new("scenario.table_open");
    let table = Table::unprofiled_with(true, None).await?;
    drop(stage);
    let driver = StageTimer::new("scenario.driver");
    let agent = table.agent();
    table.launch_profile().await?;
    let machine = operation()?;
    let body = json!({
        "operation": machine, "name": "closed", "kind": "laptop", "runtime": "manifold",
        "slots": 1, "may_run": [agent], "may_reach": ["elsewhere.example.test"],
    });
    let (status, named) = table
        .service
        .post("/network/machines", Some(&table.ada), &body)
        .await?;
    assert_eq!(status, 200, "{named}");
    refused(
        &table.ask(&agent, &machine, &table.ada).await?,
        409,
        "MachineCannotReach",
    );
    drop(driver);
    let stage = StageTimer::new("scenario.table_close");
    let result = table.close();
    drop(stage);
    drop(timing);
    result
}
