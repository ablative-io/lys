//! A computer's agent allowance changes once while retaining its creation and other permissions.

#[path = "support/harness_description.rs"]
mod harness_description;

use std::collections::BTreeMap;
use std::error::Error;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, GRANT_MODEL, Service};
use lys_identity::{AgentId, OperationId};
use lys_identity_server::dev_seed::{Seeded, seed_configured};
use lys_identity_server::network_store::{Machine, NetworkStore, TeamRecorded};
use lys_identity_server::runner_client::RunnerRecord;
use serde::Serialize;
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;
const BEA: &str = "bea-subject";

fn operation() -> Result<String, Box<dyn Error>> {
    Ok(OperationId::generate()?.to_string())
}

fn login(subject: &str) -> Login {
    Login {
        subject: subject.to_owned(),
        email: "shared@example.test".to_owned(),
    }
}

fn refused(answer: &(u16, Value), status: u16, name: &str) {
    assert_eq!(answer.0, status, "{}", answer.1);
    assert_eq!(answer.1["refusal"], name, "{}", answer.1);
}

fn member<'a>(bytes: &'a [u8], field: &str) -> Result<&'a str, Box<dyn Error>> {
    let text = std::str::from_utf8(bytes)?;
    let marker = format!("\n  \"{field}\": ");
    let start = text.find(&marker).ok_or("missing stored member")? + marker.len();
    let remaining = &text[start..];
    let end = remaining
        .find("\n  \"")
        .or_else(|| remaining.find("\n}"))
        .ok_or("unterminated stored member")?;
    Ok(remaining[..end].trim_end_matches(','))
}

fn machine_bytes<'a>(bytes: &'a [u8], id: &str) -> Result<&'a str, Box<dyn Error>> {
    let text = std::str::from_utf8(bytes)?;
    let marker = format!("    {{\n      \"id\": \"{id}\",");
    let start = text.find(&marker).ok_or("missing stored computer")?;
    let remaining = &text[start..];
    let ending = "\n    }";
    let end = remaining.find(ending).ok_or("unterminated computer")? + ending.len();
    Ok(&remaining[..end])
}

struct Table {
    service: Service,
    seeded: Seeded,
    ada: String,
    bea: String,
    path: PathBuf,
    before: Vec<u8>,
    machine: Machine,
    other: String,
    naming: Value,
}

#[derive(Serialize)]
struct Before {
    machines: Vec<Machine>,
    runners: BTreeMap<String, RunnerRecord>,
    team_changes: BTreeMap<String, TeamRecorded>,
}

impl Table {
    async fn set() -> Result<Self, Box<dyn Error>> {
        let id = operation()?;
        let other = operation()?;
        let (service, (seeded, path, before, machine)) = Service::start_with(|config| {
            let seeded = seed_configured(config, [ADMINISTRATOR, BEA])?;
            let path = config.network_file.clone().ok_or("no network file")?;
            let machine = Machine {
                id: id.clone(),
                name: "Build box".to_owned(),
                kind: "server".to_owned(),
                runtime: Some("norn".to_owned()),
                slots: 4,
                may_run: Vec::new(),
                may_run_roles: Vec::new(),
                may_reach: vec!["git.example.test".to_owned()],
                named_by: seeded.people[0].id.to_string(),
                named_at: 5,
                retired: None,
                team: None,
                creation_team: None,
            };
            let other = Machine {
                id: other.clone(),
                name: "Other box".to_owned(),
                ..machine.clone()
            };
            let recorded = TeamRecorded {
                operation: operation()?,
                machine: id.clone(),
                team: Some(operation()?),
                by: machine.named_by.clone(),
                at: 6,
            };
            let machine = Machine {
                team: recorded.team.clone(),
                ..machine
            };
            let before = serde_json::to_vec_pretty(&Before {
                machines: vec![machine.clone(), other],
                runners: BTreeMap::from([(id.clone(), RunnerRecord::Lys)]),
                team_changes: BTreeMap::from([(recorded.operation.clone(), recorded)]),
            })?;
            std::fs::write(&path, &before)?;
            let held: Value = serde_json::from_slice(&before)?;
            assert!(held.get("agent_changes").is_none(), "{held}");
            Ok((seeded, path, before, machine))
        })
        .await?;
        assert_eq!(std::fs::read(&path)?, before, "open rewrote the old store");
        let ada = service.sign_in(login(ADMINISTRATOR)).await?;
        let bea = service.sign_in(login(BEA)).await?;
        let naming = json!({
            "operation": id, "name": "Build box", "kind": "server", "runtime": "norn",
            "slots": 4, "may_run": [], "may_reach": ["git.example.test"],
        });
        Ok(Self {
            service,
            seeded,
            ada,
            bea,
            path,
            before,
            machine,
            other,
            naming,
        })
    }

    fn agent(&self) -> String {
        self.seeded.people[0].agents[0].id.to_string()
    }

    async fn sent(&self, path: &str, body: &Value) -> Result<Value, Box<dyn Error>> {
        let (status, answer) = self.service.post(path, Some(&self.ada), body).await?;
        assert_eq!(status, 200, "{path}: {answer}");
        Ok(answer)
    }

    async fn creation_replays(&self, allowed: bool) -> TestResult {
        let bytes = std::fs::read(&self.path)?;
        let inode = std::fs::metadata(&self.path)?.ino();
        let answer = self.sent("/network/machines", &self.naming).await?;
        assert_eq!(answer["id"], self.machine.id);
        assert_eq!(answer["named_at"], self.machine.named_at);
        assert_eq!(answer["named_by"], self.machine.named_by);
        assert_eq!(
            answer["may_run"].as_array().map(Vec::len),
            Some(usize::from(allowed))
        );
        assert_eq!(std::fs::read(&self.path)?, bytes);
        assert_eq!(
            std::fs::metadata(&self.path)?.ino(),
            inode,
            "creation replay replaced the file"
        );
        Ok(())
    }

    async fn profile(&self) -> TestResult {
        let path = format!("/agents/{}/provisioning", self.agent());
        self.sent(
            &path,
            &json!({
                "operation": operation()?, "from_version": 0, "model_access": ["claude-fable-5-1"],
                "tools": [], "skills": [], "mcp_servers": [], "instructions": "", "note": "",
                "harness": harness_description::declared(),
            }),
        )
        .await?;
        self.sent(
            &format!("{path}/1/review"),
            &json!({"operation": operation()?}),
        )
        .await?;
        Ok(())
    }
}

fn unchanged(path: &Path, bytes: &[u8], inode: u64) -> TestResult {
    assert_eq!(std::fs::read(path)?, bytes);
    assert_eq!(
        std::fs::metadata(path)?.ino(),
        inode,
        "replay or refusal replaced the file"
    );
    Ok(())
}

#[tokio::test]
async fn allowance_is_admin_only_retained_once_and_removal_refuses_the_next_start() -> TestResult {
    let mut table = Table::set().await?;
    let path = format!("/network/machines/{}/agents", table.machine.id);
    let grant = json!({"operation": operation()?, "agent": table.agent(), "allow": true});
    let inode = std::fs::metadata(&table.path)?.ino();
    for caller in [None, Some(table.bea.as_str())] {
        let answer = table.service.post(&path, caller, &grant).await?;
        let (status, name) = if caller.is_some() {
            (403, "NotAdmitted")
        } else {
            (401, "NotSignedIn")
        };
        refused(&answer, status, name);
        unchanged(&table.path, &table.before, inode)?;
    }

    for (machine, agent, status, name) in [
        (
            table.machine.id.clone(),
            AgentId::generate()?.to_string(),
            404,
            "AgentNotVisible",
        ),
        (
            table.machine.id.clone(),
            table.seeded.people[0].id.to_string(),
            400,
            "IdentifierMalformed",
        ),
        (operation()?, table.agent(), 404, "MachineUnknown"),
    ] {
        let answer = table
            .service
            .post(
                &format!("/network/machines/{machine}/agents"),
                Some(&table.ada),
                &json!({"operation": operation()?, "agent": agent, "allow": true}),
            )
            .await?;
        refused(&answer, status, name);
        unchanged(&table.path, &table.before, inode)?;
    }

    let first = table.sent(&path, &grant).await?;
    assert_eq!(first["recorded"]["operation"], grant["operation"]);
    assert_eq!(first["recorded"]["machine"], table.machine.id);
    assert_eq!(first["recorded"]["agent"], table.agent());
    assert_eq!(first["recorded"]["allow"], true);
    assert_eq!(
        first["recorded"]["by"],
        table.seeded.people[0].id.to_string()
    );
    assert!(first["recorded"]["at"].is_u64(), "{first}");
    assert_eq!(first["recorded"]["original_may_run"], json!([]));
    assert_eq!(
        first["machine"]["may_run"].as_array().map(Vec::len),
        Some(1)
    );
    assert_eq!(first["machine"]["may_run"][0]["id"], table.agent());
    assert_eq!(
        first["machine"]["may_reach"],
        json!(table.machine.may_reach)
    );

    let written = std::fs::read(&table.path)?;
    let inode = std::fs::metadata(&table.path)?.ino();
    let held: Value = serde_json::from_slice(&written)?;
    assert_eq!(
        held["agent_changes"].as_object().map(serde_json::Map::len),
        Some(1)
    );
    let grant_id = grant["operation"].as_str().ok_or("no granting operation")?;
    assert_eq!(held["agent_changes"][grant_id], first["recorded"]);
    for field in ["runners", "team_changes"] {
        assert_eq!(member(&written, field)?, member(&table.before, field)?);
    }
    assert_eq!(
        machine_bytes(&written, &table.other)?,
        machine_bytes(&table.before, &table.other)?
    );
    let mut expected = table.machine.clone();
    expected.may_run.push(table.agent());
    let expected = format!(
        "    {}",
        serde_json::to_string_pretty(&expected)?.replace('\n', "\n    ")
    );
    assert_eq!(machine_bytes(&written, &table.machine.id)?, expected);

    assert_eq!(table.sent(&path, &grant).await?, first);
    unchanged(&table.path, &written, inode)?;
    for (agent, allow) in [
        (table.agent(), false),
        (table.seeded.people[1].agents[0].id.to_string(), true),
    ] {
        let answer = table
            .service
            .post(
                &path,
                Some(&table.ada),
                &json!({"operation": grant["operation"], "agent": agent, "allow": allow}),
            )
            .await?;
        refused(&answer, 409, "MachineAgentsReused");
        unchanged(&table.path, &written, inode)?;
    }
    let answer = table
        .service
        .post(
            &format!("/network/machines/{}/agents", table.other),
            Some(&table.ada),
            &grant,
        )
        .await?;
    refused(&answer, 409, "MachineAgentsReused");
    unchanged(&table.path, &written, inode)?;
    table.creation_replays(true).await?;
    table.profile().await?;
    let admitted = table
        .service
        .post(
            &format!("/agents/{}/start-command", table.agent()),
            Some(&table.ada),
            &json!({"operation": operation()?, "machine": table.machine.id}),
        )
        .await?;
    refused(&admitted, 502, "SecretsUnavailable");
    assert_ne!(
        admitted.1["refusal"], "MachineNotForAgent",
        "grant did not admit past placement"
    );

    let removal = json!({"operation": operation()?, "agent": table.agent(), "allow": false});
    let removed = table.sent(&path, &removal).await?;
    assert_eq!(removed["recorded"]["allow"], false);
    assert!(removed["recorded"].get("original_may_run").is_none());
    assert_eq!(removed["machine"]["may_run"], json!([]));
    assert_eq!(
        removed["machine"]["may_reach"],
        json!(table.machine.may_reach)
    );
    let bytes = std::fs::read(&table.path)?;
    let inode = std::fs::metadata(&table.path)?.ino();
    let again = table.sent(&path, &grant).await?;
    assert_eq!(again["recorded"], first["recorded"]);
    assert_eq!(
        again["machine"]["may_run"],
        json!([]),
        "old grant replay must not grant again"
    );
    assert_eq!(table.sent(&path, &removal).await?, removed);
    unchanged(&table.path, &bytes, inode)?;
    table.creation_replays(false).await?;
    let answer = table
        .service
        .post(
            &format!("/agents/{}/start-command", table.agent()),
            Some(&table.ada),
            &json!({"operation": operation()?, "machine": table.machine.id}),
        )
        .await?;
    refused(&answer, 403, "MachineNotForAgent");

    table.service.restart().await?;
    assert_eq!(
        table.sent(&path, &grant).await?["recorded"],
        first["recorded"]
    );
    unchanged(&table.path, &bytes, inode)?;
    table.creation_replays(false).await?;
    let held: Value = serde_json::from_slice(&bytes)?;
    assert_eq!(
        held["agent_changes"].as_object().map(serde_json::Map::len),
        Some(2)
    );
    assert_eq!(
        held["machines"][0]["may_reach"],
        json!(table.machine.may_reach)
    );
    table
        .sent(
            &format!("/network/machines/{}/retire", table.other),
            &json!({}),
        )
        .await?;
    let bytes = std::fs::read(&table.path)?;
    let inode = std::fs::metadata(&table.path)?.ino();
    let retired = table
        .service
        .post(
            &format!("/network/machines/{}/agents", table.other),
            Some(&table.ada),
            &json!({"operation": operation()?, "agent": table.agent(), "allow": true}),
        )
        .await?;
    refused(&retired, 409, "MachineRetired");
    unchanged(&table.path, &bytes, inode)?;
    Ok(())
}

#[test]
fn a_pre_change_network_file_opens_without_rewriting_any_bytes() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("network.json");
    let before = br#"{"machines":[{"id":"old-machine","name":"Build box","kind":"server","runtime":"norn","slots":4,"may_run":[],"may_reach":["git.example.test"],"named_by":"person","named_at":5,"retired":null}],"runners":{"old-machine":{"kind":"lys"}},"team_changes":{}}"#;
    std::fs::write(&path, before)?;
    let inode = std::fs::metadata(&path)?.ino();
    for _ in 0..2 {
        let store = NetworkStore::open(&path)?;
        let machine = store.machine("old-machine").ok_or("old computer missing")?;
        assert_eq!(machine.may_reach, ["git.example.test"]);
        assert!(machine.may_run.is_empty());
        assert_eq!(store.runner("old-machine"), Some(&RunnerRecord::Lys));
        unchanged(&path, before, inode)?;
    }
    Ok(())
}

#[test]
fn the_agents_route_describes_its_body_receipt_and_conflict() -> TestResult {
    let document = lys_identity_server::openapi::document()?;
    let route = &document["paths"]["/network/machines/{id}/agents"]["post"];
    assert!(route["requestBody"].is_object(), "{route}");
    assert!(route["responses"]["200"].is_object(), "{route}");
    assert!(route["responses"]["default"].is_object(), "{route}");
    assert!(route.to_string().contains("MachineAgentsReused"), "{route}");
    Ok(())
}

#[tokio::test]
async fn a_retired_agent_is_not_allowed_onto_a_computer() -> TestResult {
    let table = Table::set().await?;
    let retire = json!({
        "operation": operation()?,
        "transition": "retire",
        "reason": "Agent withdrawn",
    });
    table
        .sent(
            &format!("/identities/{}/transitions", table.agent()),
            &retire,
        )
        .await?;
    let inode = std::fs::metadata(&table.path)?.ino();
    let path = format!("/network/machines/{}/agents", table.machine.id);
    let grant = json!({"operation": operation()?, "agent": table.agent(), "allow": true});
    refused(
        &table.service.post(&path, Some(&table.ada), &grant).await?,
        403,
        "inactive",
    );
    unchanged(&table.path, &table.before, inode)?;
    Ok(())
}

#[tokio::test]
async fn without_a_network_file_the_agents_route_says_so() -> TestResult {
    let (service, seeded) = Service::start_adjusted(
        GRANT_MODEL,
        None,
        None,
        None,
        |config| config.network_file = None,
        |config| Ok(seed_configured(config, [ADMINISTRATOR, BEA])?),
    )
    .await?;
    let ada = service.sign_in(login(ADMINISTRATOR)).await?;
    let grant = json!({
        "operation": operation()?,
        "agent": seeded.people[0].agents[0].id.to_string(),
        "allow": true,
    });
    refused(
        &service
            .post(
                &format!("/network/machines/{}/agents", operation()?),
                Some(&ada),
                &grant,
            )
            .await?,
        503,
        "NetworkUnavailable",
    );
    Ok(())
}
