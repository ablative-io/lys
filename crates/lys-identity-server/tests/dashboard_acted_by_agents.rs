//! The person sees every agent an agent registered and every start an agent
//! made (AGENTS-016 R5): `GET /dashboard` answers them in `acted_by_agents`,
//! newest first, read from the records already written: the registration
//! event's actor in the directory log and the launch record's giver.

#[path = "support/agent_starts_agent.rs"]
mod support;

#[path = "support/harness_description.rs"]
mod harness_description;

use serde_json::{Value, json};
use sha2::Digest;
use support::{TestResult, World, key_for, operation, refused_as};

/// Lowercase hex of `bytes`.
fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut text = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        text.push(char::from(DIGITS[usize::from(byte >> 4)]));
        text.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    text
}

fn ok(answer: &(u16, Value)) -> Result<&Value, String> {
    if answer.0 == 200 {
        Ok(&answer.1)
    } else {
        Err(format!("status {}: {}", answer.0, answer.1))
    }
}

/// The lines `cookie`'s dashboard lists as acted for them by their agents.
async fn acted(world: &World, cookie: &str) -> Result<Vec<Value>, Box<dyn std::error::Error>> {
    let (status, dashboard) = world.service.get("/dashboard", Some(cookie)).await?;
    assert_eq!(status, 200, "{dashboard}");
    Ok(dashboard["acted_by_agents"]
        .as_array()
        .ok_or_else(|| format!("no acted_by_agents: {dashboard}"))?
        .clone())
}

#[tokio::test]
async fn the_dashboard_lists_what_an_agent_registered_and_started_for_its_person() -> TestResult {
    let world = World::new().await?;
    // Nothing yet: every agent of Bea's was registered by a person.
    assert!(acted(&world, &world.bea).await?.is_empty());

    // The creator registers an agent, gives it its first certificate, sets
    // it up and, once reviewed and allowed, starts it.
    world
        .give(
            &world.creator.id,
            &world.bea_resource(),
            "agent.create",
            false,
        )
        .await?;
    let body = json!({"operation": operation()?, "display_name": "Assistant"});
    let registered = world.signed(&world.creator, "/agents", &body).await?;
    let agent = ok(&registered)?["agent"]
        .as_str()
        .ok_or("no agent")?
        .to_owned();
    let rights = world
        .beas_grants()
        .await?
        .into_iter()
        .filter(|grant| grant["resource"]["id"] == agent.as_str())
        .count();
    assert_eq!(rights, 5, "the creator holds its five rights on it");
    let (assistant, request) = key_for(&world.service, &agent, "first")?;
    let certified = world
        .signed(
            &world.creator,
            &format!("/agents/{agent}/certificates"),
            &json!({"operation": operation()?, "request": request}),
        )
        .await?;
    ok(&certified)?;
    let asked = json!({"jsonrpc": "2.0", "id": 1, "method": "tools/call",
        "params": {"name": "what-can-I-do", "arguments": {}}});
    let reply = world.mcp(&assistant, &asked).await?;
    assert!(reply["error"].is_null(), "{reply}");

    let profile = json!({
        "working_folder": world.service.dir.path(),
        "operation": operation()?, "from_version": 0,
        "model_access": ["primary-model"], "tools": [], "skills": [],
        "mcp_servers": [], "instructions": "Help Bea.", "note": "",
        "harness": harness_description::declared(),
        "permissions": {"default_mode": "plan"},
    });
    let path = format!("/agents/{agent}/provisioning");
    ok(&world.signed(&world.creator, &path, &profile).await?)?;
    world
        .give(
            &world.reviewer.id,
            &json!({"kind": "agent", "id": agent}),
            "agent.provisioning.review",
            false,
        )
        .await?;
    let reviewed = world
        .signed(
            &world.reviewer,
            &format!("{path}/1/review"),
            &json!({"operation": operation()?}),
        )
        .await?;
    let version = ok(&reviewed)?["profile"]["operation"].clone();
    let machine = world.beas_computer().await?;
    world
        .give(
            &world.creator.id,
            &json!({"kind": "machine", "id": machine}),
            "machine.agent.set",
            false,
        )
        .await?;
    ok(&world
        .signed(
            &world.creator,
            &format!("/network/machines/{machine}/agents"),
            &json!({"operation": operation()?, "agent": agent, "allow": true}),
        )
        .await?)?;
    let started = world
        .signed(
            &world.creator,
            &format!("/agents/{agent}/start"),
            &json!({"profile_version": version, "machine": machine}),
        )
        .await?;
    assert!(!refused_as(&started, "not_permitted"), "{}", started.1);
    let record = ok(&started)?["launch_record"].clone();

    // Bea's dashboard: the start, then the registration, newest first. The
    // acts the creator took with its grants, its certificate issue among
    // them, are listed beside these (AGENTS-019 R5); this test reads the two.
    let every = acted(&world, &world.bea).await?;
    let lines: Vec<Value> = every
        .iter()
        .filter(|line| line["kind"] == "started" || line["kind"] == "registered")
        .cloned()
        .collect();
    assert_eq!(lines.len(), 2, "{lines:?}");
    let (start, registration) = (&lines[0], &lines[1]);
    assert_eq!(start["kind"], "started", "{lines:?}");
    assert_eq!(start["acting_agent"], world.creator.id.as_str(), "{start}");
    assert_eq!(start["agent"], agent.as_str(), "{start}");
    assert_eq!(start["at"], record["given_at"], "{start} {record}");
    // AGENTS-017 R2: the started line carries the session its launch record
    // runs as, so the Dashboard's Stop has a run to end: the launch session
    // derived from the record's id, as the runner sessions name it.
    let record_id = record["id"].as_str().ok_or("the launch record has no id")?;
    let digest = sha2::Sha256::digest(format!("lys-identity/launch-session/v1\n{record_id}"));
    let expected = hex(&digest);
    assert_eq!(
        start["session"],
        format!("op-{}", &expected[..32]).as_str(),
        "{start} {record}"
    );
    assert_eq!(registration["kind"], "registered", "{lines:?}");
    assert_eq!(
        registration["acting_agent"],
        world.creator.id.as_str(),
        "{registration}"
    );
    assert_eq!(registration["agent"], agent.as_str(), "{registration}");
    assert!(registration["at"].as_u64().is_some(), "{registration}");
    assert!(registration.get("session").is_none_or(Value::is_null));
    assert!(
        start["at"].as_u64() >= registration["at"].as_u64(),
        "{lines:?}"
    );

    // The stranger acted on nothing: no line names it.
    assert!(
        every
            .iter()
            .all(|line| line["acting_agent"] != world.stranger.id.as_str()),
        "{every:?}"
    );
    // Ada's agents were all registered and started by people: no lines.
    assert!(acted(&world, &world.ada).await?.is_empty());
    Ok(())
}
