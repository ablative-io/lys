#![cfg(test)]
#[path = "../tests/support/harness_description.rs"]
mod harness_description;
use super::build;
use crate::agent_pass_store::Passes;
use crate::launch_template::{Start, from_template, render};
use crate::provisioning_store::{ProvisioningStore, Version};
use lys_core::Ed25519Identity;
use lys_identity::AgentId;
use lys_identity::start::LaunchRecord;
use lys_runner::{Act, Answer, Client, Options, Runner};
use serde_json::json;
use std::error::Error;
use std::os::unix::fs::PermissionsExt;
use std::sync::Arc;

#[test]
fn a_launch_record_start_writes_the_pass_into_the_seat_config() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let program = dir.path().join("seat");
    std::fs::write(&program, "#!/bin/sh\nexit 0\n")?;
    std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o700))?;
    let agent = AgentId::from_bytes([1; 16]);
    let agent_text = agent.to_string();
    let mut harness = harness_description::declared();
    harness["program"] = json!(program);
    let version: Version = serde_json::from_value(json!({
        "number":1, "operation":"op-fixture-profile", "settings":{
            "model_access":["fixture-model"], "tools":[], "skills":[], "mcp_servers":[],
            "instructions":"", "note":"", "harness":harness
        }, "set_by":"fixture-person", "set_at":1,
        "reviewed":{"operation":"op-fixture-review", "by":"fixture-person", "at":1}
    }))?;
    let rendered = render(
        &Start {
            agent: &agent_text,
            session: "session-fixture",
            machine: "machine-fixture",
            runtime: "sh",
            version: &version,
            skills: &[],
            policy: None,
        },
        &[],
    )?;
    let native = from_template(&version, &rendered.template, &rendered.template_sha256)?;
    let record = LaunchRecord {
        id: "launch-fixture".to_owned(),
        agent: agent_text.clone(),
        machine: "machine-fixture".to_owned(),
        executable: native.program,
        arguments: native.arguments,
        working_directory: dir.path().display().to_string(),
        profile_version: version.operation.clone(),
        credential_ids: Vec::new(),
        given_by: "fixture-person".to_owned(),
        given_at: 1,
        copied_from: None,
    };
    let mut store = ProvisioningStore::open(&dir.path().join("profiles.json"))?;
    let mut later = version.clone();
    later.number = 2;
    later.operation = "op-fixture-later".to_owned();
    later.settings.instructions = "a later profile must not replace this signed start".to_owned();
    store.set(&agent_text, 0, version)?;
    store.set(&agent_text, 1, later)?;
    let mut unknown = record.clone();
    unknown.profile_version = "op-fixture-absent".to_owned();
    assert!(matches!(
        build(&store, &unknown, "session-fixture".to_owned(), "sh", None),
        Err(crate::error::ServerError::LaunchUnrenderable { .. })
    ));
    let mut foreign = record.clone();
    foreign.agent = AgentId::from_bytes([2; 16]).to_string();
    assert!(matches!(
        build(&store, &foreign, "session-fixture".to_owned(), "sh", None),
        Err(crate::error::ServerError::LaunchUnrenderable { .. })
    ));
    let mut changed = record.clone();
    changed.executable = "/bin/false".to_owned();
    assert!(matches!(
        build(&store, &changed, "session-fixture".to_owned(), "sh", None),
        Err(crate::error::ServerError::LaunchUnrenderable { .. })
    ));
    let launch = build(&store, &record, "session-fixture".to_owned(), "sh", None)?;
    let mut passes = Passes::open(dir.path().join("passes.json"))?;
    let act = crate::runner_start_pass::act(&mut passes, agent, "http://fixture.test", launch)?;
    if !matches!(&act, Act::Start { launch, lys_mcp:Some(_) } if launch.config.is_some()) {
        return Err("the record start carries no native config or run pass".into());
    }
    let key = Arc::new(Ed25519Identity::load_or_generate(
        &dir.path().join("runner.key"),
    )?);
    let socket = dir.path().join("runner.sock");
    let state = dir.path().join("runner-state");
    let runner = Runner::open(&Options {
        socket: socket.clone(),
        state: state.clone(),
        server_key: key.public_key_bytes(),
        scrollback: 4096,
    })?;
    let serving = runner.spawn();
    let client = Client::new(socket, key);
    let tested = (|| -> Result<(), Box<dyn Error>> {
        if !matches!(client.ask(&act)?, Answer::Started { .. }) {
            return Err("the runner refused the record's native start".into());
        }
        let config: serde_json::Value = serde_json::from_slice(&std::fs::read(
            state.join("sessions/session-fixture/config/mcp.json"),
        )?)?;
        let pass = config["mcpServers"]["lys"]["headers"]["lys-agent-pass"]
            .as_str()
            .ok_or("no pass in seat config")?;
        if passes.lookup(pass)? != agent {
            return Err("the seat config pass resolves to another agent".into());
        }
        if config["mcpServers"]["lys"]["url"] != "http://fixture.test/api/mcp" {
            return Err("the seat config names another MCP endpoint".into());
        }
        if std::fs::read(state.join("sessions/session-fixture/config/instructions.txt"))? != b"" {
            return Err("the native config came from a later profile".into());
        }
        Ok(())
    })();
    serving.stop()?;
    tested
}
