//! The provisioning routes: the administrator sets an agent's profile, each
//! change a version over the one it saw; the administrator and the person
//! responsible for the agent read it; a late or reused change lands
//! nowhere; and what is kept is read back from one file.

use std::error::Error;

use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_home::harness::launch_fields::Channel;
use lys_identity::{AgentId, OperationId};
use lys_identity_server::dev_seed::{Seeded, seed_configured};
use lys_identity_server::error::ServerError;
use lys_identity_server::provisioning_store::{
    McpServer, Profile, ProvisioningStore, Settings, Version,
};
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

const BEA: &str = "bea-subject";

fn login(subject: &str) -> Login {
    Login {
        subject: subject.to_owned(),
        email: "shared@example.test".to_owned(),
    }
}

fn operation() -> Result<String, Box<dyn Error>> {
    Ok(OperationId::generate()?.to_string())
}

fn refused(answer: &(u16, Value), status: u16, name: &str) {
    assert_eq!(answer.0, status, "{}", answer.1);
    assert_eq!(answer.1["refusal"], name, "{}", answer.1);
}

fn profile(operation: &str, from: u32, note: &str) -> Value {
    json!({
        "operation": operation,
        "from_version": from,
        "model_access": ["claude-fable-5-1", " claude-fable-5-1 "],
        "tools": ["read", "edit"],
        "skills": ["review"],
        "mcp_servers": [{ "name": "cambium", "url": "https://cambium.example.test/mcp" }],
        "instructions": "Build what the brief says.",
        "note": note,
    })
}

/// A service with Ada as the administrator and Bea, and their cookies.
struct Table {
    service: Service,
    seeded: Seeded,
    ada: String,
    bea: String,
}

impl Table {
    async fn set() -> Result<Self, Box<dyn Error>> {
        let (service, seeded) =
            Service::start_with(|config| Ok(seed_configured(config, [ADMINISTRATOR, BEA])?))
                .await?;
        let ada = service.sign_in(login(ADMINISTRATOR)).await?;
        let bea = service.sign_in(login(BEA)).await?;
        let skill = json!({ "name": "review", "text": "Read the change against its brief.\n" });
        let (status, kept) = service.post("/skills", Some(&ada), &skill).await?;
        assert_eq!(status, 200, "{kept}");
        Ok(Self {
            service,
            seeded,
            ada,
            bea,
        })
    }

    /// The route of the first agent of the person at `person`.
    fn route(&self, person: usize) -> String {
        format!(
            "/agents/{}/provisioning",
            self.seeded.people[person].agents[0].id
        )
    }
}

#[tokio::test]
async fn the_administrator_sets_a_profile_and_each_change_is_a_version() -> TestResult {
    let table = Table::set().await?;
    let path = table.route(0);
    let agent = table.seeded.people[0].agents[0].id.to_string();

    let (status, none) = table.service.get(&path, Some(&table.ada)).await?;
    assert_eq!(status, 200, "{none}");
    assert_eq!(
        none,
        json!({
            "agent": agent,
            "profile": null,
            "versions": [],
            "enforced": false,
            "recorded": null
        })
    );

    let first = profile(&operation()?, 0, "First setup.");
    let (status, set) = table.service.post(&path, Some(&table.ada), &first).await?;
    assert_eq!(status, 200, "{set}");
    assert_eq!(set["profile"]["version"], 1);
    assert_eq!(set["profile"]["operation"], first["operation"]);
    assert_eq!(
        set["profile"]["model_access"],
        json!(["claude-fable-5-1"]),
        "a name given twice is kept once"
    );
    assert_eq!(set["profile"]["tools"], json!(["read", "edit"]));
    assert_eq!(
        set["profile"]["mcp_servers"],
        json!([{ "name": "cambium", "url": "https://cambium.example.test/mcp", "channel": "off" }])
    );
    assert_eq!(
        set["profile"]["set_by"],
        table.seeded.people[0].id.to_string()
    );
    assert_eq!(set["enforced"], false, "no runtime applies a profile");
    assert_eq!(
        set["recorded"],
        json!({ "operation": first["operation"], "version": 1 })
    );

    let (status, again) = table.service.post(&path, Some(&table.ada), &first).await?;
    assert_eq!(status, 200, "{again}");
    assert_eq!(again, set, "set again in the same words it is kept once");

    let second = profile(&operation()?, 1, "Adds nothing, says why.");
    let (status, next) = table.service.post(&path, Some(&table.ada), &second).await?;
    assert_eq!(status, 200, "{next}");
    assert_eq!(next["profile"]["version"], 2);
    let versions = next["versions"].as_array().ok_or("no versions")?;
    assert_eq!(versions.len(), 2);
    assert_eq!(versions[0]["note"], "First setup.");
    assert_eq!(versions[1]["version"], 2);

    assert_eq!(
        next["recorded"],
        json!({ "operation": second["operation"], "version": 2 })
    );

    let (status, retried) = table.service.post(&path, Some(&table.ada), &first).await?;
    assert_eq!(status, 200, "{retried}");
    assert_eq!(
        retried["recorded"],
        json!({ "operation": first["operation"], "version": 1 }),
        "the first change sent again answers the version it was recorded as"
    );
    assert_eq!(
        retried["profile"], next["profile"],
        "the profile is the latest"
    );
    assert_eq!(
        retried["versions"], next["versions"],
        "no version was added"
    );

    let (status, read) = table.service.get(&path, Some(&table.ada)).await?;
    assert_eq!(status, 200, "{read}");
    assert_eq!(read["recorded"], Value::Null, "a read records nothing");
    assert_eq!(read["profile"], next["profile"]);
    assert_eq!(read["versions"], next["versions"]);
    Ok(())
}

#[tokio::test]
async fn a_late_or_reused_change_lands_nowhere() -> TestResult {
    let table = Table::set().await?;
    let path = table.route(0);
    let first = profile(&operation()?, 0, "First setup.");
    let (status, set) = table.service.post(&path, Some(&table.ada), &first).await?;
    assert_eq!(status, 200, "{set}");

    let late = profile(&operation()?, 0, "Set over nothing, late.");
    let answer = table.service.post(&path, Some(&table.ada), &late).await?;
    refused(&answer, 409, "ProvisioningChanged");
    let mut other = first.clone();
    other["note"] = json!("Other words.");
    let answer = table.service.post(&path, Some(&table.ada), &other).await?;
    refused(&answer, 409, "ProvisioningReused");
    let elsewhere = table
        .service
        .post(&table.route(1), Some(&table.ada), &first)
        .await?;
    refused(&elsewhere, 409, "ProvisioningReused");

    let mut bad = profile(&operation()?, 1, "A server with no address.");
    bad["mcp_servers"] = json!([{ "name": "files", "url": "file:///etc" }]);
    let answer = table.service.post(&path, Some(&table.ada), &bad).await?;
    refused(&answer, 400, "RequestMalformed");
    let mut short = profile(&operation()?, 1, "No tools member.");
    short
        .as_object_mut()
        .ok_or("not an object")?
        .remove("tools");
    let answer = table.service.post(&path, Some(&table.ada), &short).await?;
    refused(&answer, 400, "RequestMalformed");

    let (status, read) = table.service.get(&path, Some(&table.ada)).await?;
    assert_eq!(status, 200, "{read}");
    assert_eq!(read["profile"], set["profile"], "nothing was changed");
    assert_eq!(read["versions"], set["versions"], "nothing was changed");
    Ok(())
}

#[tokio::test]
async fn a_profile_is_shown_to_the_administrator_and_whoever_answers_for_the_agent() -> TestResult {
    let table = Table::set().await?;
    let (adas, beas) = (table.route(0), table.route(1));
    let body = profile(&operation()?, 0, "First setup.");

    let answer = table.service.post(&adas, Some(&table.bea), &body).await?;
    refused(&answer, 403, "NotAdmitted");
    let answer = table.service.post(&adas, None, &body).await?;
    refused(&answer, 401, "NotSignedIn");
    let answer = table.service.get(&adas, None).await?;
    refused(&answer, 401, "NotSignedIn");
    let answer = table.service.get(&adas, Some(&table.bea)).await?;
    refused(&answer, 404, "AgentNotVisible");

    let (status, own) = table.service.get(&beas, Some(&table.bea)).await?;
    assert_eq!(status, 200, "{own}");
    assert_eq!(own["profile"], Value::Null);
    let (status, seen) = table.service.get(&beas, Some(&table.ada)).await?;
    assert_eq!(status, 200, "{seen}");

    let stranger = format!("/agents/{}/provisioning", AgentId::generate()?);
    let answer = table.service.get(&stranger, Some(&table.ada)).await?;
    refused(&answer, 404, "AgentNotVisible");
    let answer = table
        .service
        .post(&stranger, Some(&table.ada), &body)
        .await?;
    refused(&answer, 404, "AgentNotVisible");
    let answer = table
        .service
        .get("/agents/not-an-id/provisioning", Some(&table.ada))
        .await?;
    refused(&answer, 404, "AgentNotVisible");
    Ok(())
}

fn version(operation: &str, note: &str) -> Version {
    Version {
        number: 0,
        operation: operation.to_owned(),
        settings: Settings {
            model_access: vec!["claude-fable-5-1".to_owned()],
            tools: Vec::new(),
            skills: Vec::new(),
            mcp_servers: vec![McpServer {
                name: "cambium".to_owned(),
                url: "https://cambium.example.test/mcp".to_owned(),
                command: None,
                channel: Channel::Off,
            }],
            instructions: String::new(),
            note: note.to_owned(),
            session: None,
            harness: None,
            skill_pins: Vec::new(),
        },
        set_by: "person-a".to_owned(),
        set_at: 10,
        reviewed: None,
    }
}

#[test]
fn the_profiles_are_read_back_from_one_file_as_they_were_kept() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("provisioning.json");
    let mut store = ProvisioningStore::open(&path)?;
    assert_eq!(store.set("agent-1", 0, version("set-1", "first"))?, 1);
    assert_eq!(store.set("agent-1", 1, version("set-2", "second"))?, 2);
    assert_eq!(store.set("agent-1", 1, version("set-2", "second"))?, 2);
    assert_eq!(store.set("agent-2", 0, version("set-3", "first"))?, 1);
    let late = store.set("agent-1", 1, version("set-4", "late"));
    assert!(
        matches!(late, Err(ServerError::ProvisioningChanged { latest: 2 })),
        "{late:?}"
    );
    let kept = store.profiles().to_vec();
    drop(store);

    let store = ProvisioningStore::open(&path)?;
    assert_eq!(store.profiles(), kept);
    assert_eq!(store.profile("agent-1").map(Profile::latest), Some(2));
    assert_eq!(store.profile("agent-3"), None);
    let beside: Vec<_> = std::fs::read_dir(dir.path())?
        .filter_map(Result::ok)
        .map(|entry| entry.file_name())
        .collect();
    assert_eq!(
        beside,
        ["provisioning.json"],
        "nothing is left beside the file"
    );
    Ok(())
}
