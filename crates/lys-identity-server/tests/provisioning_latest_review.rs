//! A review must name the current profile before it can authorize a start.
//! AGENTS-017 R3: a person's review carries by content, so the same settings
//! start on another agent of theirs with the carried review named; a changed
//! setting needs review again; a review an agent made never carries.

#[path = "support/runner_start.rs"]
pub mod support;

#[path = "support/agent_starts_agent.rs"]
pub mod world;

use std::error::Error;

use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_identity::OperationId;
use lys_identity_server::dev_seed::seed_configured;
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

fn profile(from: u32) -> Result<Value, Box<dyn Error>> {
    Ok(json!({
        "operation": OperationId::generate()?.to_string(),
        "from_version": from,
        "model_access": ["model-a"],
        "tools": [],
        "skills": [],
        "mcp_servers": [],
        "instructions": "",
        "note": ""
    }))
}

#[tokio::test]
async fn a_replaced_profile_is_refused_naming_the_latest_version() -> TestResult {
    let (service, seeded) = Service::start_with(|config| {
        Ok(seed_configured(
            config,
            [ADMINISTRATOR, "reviewer-subject"],
        )?)
    })
    .await?;
    let cookie = service
        .sign_in(Login {
            subject: ADMINISTRATOR.to_owned(),
            email: "operator@example.test".to_owned(),
        })
        .await?;
    let path = format!("/agents/{}/provisioning", seeded.people[0].agents[0].id);
    for from in 0..2 {
        let (status, body) = service.post(&path, Some(&cookie), &profile(from)?).await?;
        assert_eq!(status, 200, "{body}");
        assert_eq!(body["profile"]["version"], from + 1);
    }
    let review = json!({ "operation": OperationId::generate()?.to_string() });
    let (status, body) = service
        .post(&format!("{path}/1/review"), Some(&cookie), &review)
        .await?;
    assert_eq!(status, 409, "{body}");
    assert_eq!(body["refusal"], "ProfileVersionReplaced", "{body}");
    assert!(
        body["reason"]
            .as_str()
            .is_some_and(|reason| reason.contains("version 2")),
        "{body}"
    );
    let (status, current) = service.get(&path, Some(&cookie)).await?;
    assert_eq!(status, 200, "{current}");
    assert_eq!(current["profile"]["reviewed_by"], Value::Null);
    let (status, reviewed) = service
        .post(&format!("{path}/2/review"), Some(&cookie), &review)
        .await?;
    assert_eq!(status, 200, "{reviewed}");
    assert!(reviewed["profile"]["reviewed_by"].is_string());
    Ok(())
}

/// The same settings for every agent of the table, the declared fixture
/// program's, working in the table's folder; `model` is the one setting a
/// test changes.
fn same_settings(table: &support::Table, from: u32, model: &str) -> Result<Value, Box<dyn Error>> {
    Ok(json!({
        "operation": OperationId::generate()?.to_string(), "from_version": from,
        "model_access": [model], "tools": [], "skills": [], "mcp_servers": [],
        "instructions": "", "note": "",
        "harness": table.harness(), "permissions": {"default_mode": "plan"},
        "working_folder": table.dir.path(),
    }))
}

/// An agent the administrator registers and activates for themself.
async fn own_agent(table: &support::Table, name: &str) -> Result<String, Box<dyn Error>> {
    let registered = table
        .ok(
            "/agents",
            &json!({"operation": support::operation()?, "display_name": name}),
        )
        .await?;
    let agent = registered["agent"].as_str().ok_or("no agent")?.to_owned();
    table
        .ok(
            &format!("/identities/{agent}/transitions"),
            &json!({"operation": support::operation()?, "transition": "activate", "reason": ""}),
        )
        .await?;
    Ok(agent)
}

async fn carried_start(table: &support::Table) -> TestResult {
    let first = table.agent();
    let second = own_agent(table, "Second").await?;
    let third = own_agent(table, "Third").await?;
    for (agent, model) in [
        (&first, "claude-fable-5-1"),
        (&second, "claude-fable-5-1"),
        (&third, "claude-opus-5-5"),
    ] {
        let settings = same_settings(table, 0, model)?;
        table
            .ok(&format!("/agents/{agent}/provisioning"), &settings)
            .await?;
    }
    let review_answer = table
        .ok(
            &format!("/agents/{first}/provisioning/1/review"),
            &json!({"operation": support::operation()?}),
        )
        .await?;
    let reviewer = review_answer["profile"]["reviewed_by"].clone();
    assert_eq!(reviewer, json!(table.person(0)), "{review_answer}");
    let machine = table
        .machine(
            &json!({
                "operation": support::operation()?, "name": "This computer", "kind": "Computer",
                "runtime": "lys-runner", "slots": 0, "may_run": [first, second, third],
                "may_run_roles": [], "may_reach": [],
            }),
            Some(json!({"kind": "lys"})),
        )
        .await?;

    // The second agent's version has no review of its own; the first's carries.
    let (status, started) = table
        .start(
            &second,
            &json!({"machine": machine, "operation": support::operation()?}),
        )
        .await?;
    assert_eq!(status, 200, "{started}");
    assert_eq!(started["provisioning_version"], 1, "{started}");
    assert_eq!(
        started["carried_review"]["agent"],
        json!(first),
        "{started}"
    );
    assert_eq!(started["carried_review"]["version"], 1, "{started}");
    assert_eq!(started["carried_review"]["by"], reviewer, "{started}");
    assert_eq!(
        started["carried_review"]["at"], review_answer["profile"]["reviewed_at"],
        "{started}"
    );

    // One setting changed is other content: refused, by name, as before.
    let (status, refused) = table
        .start(
            &third,
            &json!({"machine": machine, "operation": support::operation()?}),
        )
        .await?;
    assert_eq!(
        refused["refusal"], "ProfileNotReviewed",
        "{status}: {refused}"
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn a_persons_review_carries_by_content_to_another_agent_of_theirs() -> TestResult {
    let table = support::Table::unprofiled().await?;
    let walked = carried_start(&table).await;
    let closed = table.close();
    walked?;
    closed
}

#[tokio::test]
async fn a_review_an_agent_made_carries_to_no_other_agent() -> TestResult {
    let world = world::World::new().await?;
    world
        .give(
            &world.reviewer.id,
            &world.bea_resource(),
            "agent.provisioning.review",
            false,
        )
        .await?;
    // Bea sets the same settings on two of her agents.
    for agent in [&world.creator.id, &world.stranger.id] {
        let (status, set) = world
            .service
            .post(
                &format!("/agents/{agent}/provisioning"),
                Some(&world.bea),
                &profile(0)?,
            )
            .await?;
        assert_eq!(status, 200, "{set}");
    }
    // Her reviewer agent reviews the creator's version: a review an agent made.
    let review = world
        .signed(
            &world.reviewer,
            &format!("/agents/{}/provisioning/1/review", world.creator.id),
            &json!({"operation": world::operation()?}),
        )
        .await?;
    assert_eq!(review.0, 200, "{}", review.1);
    assert_eq!(review.1["profile"]["reviewed_by"], json!(world.reviewer.id));

    // It carries to no other agent: the stranger's version reads neither, and its start is refused.
    let (status, read) = world
        .service
        .get(
            &format!("/agents/{}/provisioning", world.stranger.id),
            Some(&world.bea),
        )
        .await?;
    assert_eq!(status, 200, "{read}");
    assert_eq!(read["versions"][0]["reviewed"], "neither", "{read}");
    assert!(
        read["versions"][0].get("carried_review").is_none(),
        "{read}"
    );
    let (status, refused) = world
        .service
        .post(
            &format!("/agents/{}/start-command", world.stranger.id),
            Some(&world.bea),
            &json!({"machine": world::operation()?, "operation": world::operation()?}),
        )
        .await?;
    assert_eq!(
        refused["refusal"], "ProfileNotReviewed",
        "{status}: {refused}"
    );
    Ok(())
}
