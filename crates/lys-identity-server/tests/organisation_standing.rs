#![cfg(test)]

//! An agent's standing in its organisation admits it to what the
//! administrator does there (24 October 2026). A stranger holding nothing is
//! refused by the act's name; given the administrator relation on Ada's
//! organisation, the same agent reads every person and its own record,
//! issues another agent's certificate, and discovery names the routes the
//! standing opens; given reading alone, it reads and changes nothing. Each
//! admission records its use against the standing grant.

#[path = "support/agent_starts_agent.rs"]
pub mod support;

#[path = "../src/mcp_test_answer.rs"]
mod mcp_answer;

use serde_json::{Value, json};
use support::{Certified, TestResult, World, key_for, operation};

/// Ada gives `agent`, one of Bea's, `relation` on the organisation from her
/// own standing, Bea responsible for it, answering the grant's id.
async fn standing(
    world: &World,
    agent: &Certified,
    relation: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let (status, me) = world.service.get("/me", Some(&world.ada)).await?;
    assert_eq!(status, 200, "{me}");
    let body = json!({
        "operation": operation()?, "route": "api",
        "source": "administrator", "recipient": agent.id, "responsible": world.bea_person,
        "resource": {"kind": "organisation", "id": me["organisation"]["id"]},
        "relation": relation, "pass_on": {"kind": "use_only"},
        "window": {"starts_at": 0, "ends_at": null}
    });
    let (status, given) = world
        .service
        .post("/grants", Some(&world.ada), &body)
        .await?;
    assert_eq!(status, 200, "{given}");
    Ok(given["grant"].as_str().ok_or("a grant")?.to_owned())
}

/// `agent`'s MCP call of `tool` with `arguments`: the relayed status and answer.
async fn relayed(
    world: &World,
    agent: &Certified,
    tool: &str,
    arguments: Value,
) -> Result<(u64, Value), Box<dyn std::error::Error>> {
    let asked = json!({"jsonrpc": "2.0", "id": 1, "method": "tools/call",
        "params": {"name": tool, "arguments": arguments}});
    let answer = world.mcp(agent, &asked).await?;
    let relayed = mcp_answer::body(&answer["result"]);
    let status = relayed["status"]
        .as_u64()
        .ok_or_else(|| format!("no status: {answer}"))?;
    Ok((status, relayed["body"].clone()))
}

async fn read(
    world: &World,
    agent: &Certified,
    path: &str,
) -> Result<(u64, Value), Box<dyn std::error::Error>> {
    relayed(world, agent, "read", json!({"path": path})).await
}

async fn change(
    world: &World,
    agent: &Certified,
    path: &str,
    body: &Value,
) -> Result<(u64, Value), Box<dyn std::error::Error>> {
    relayed(
        world,
        agent,
        "change",
        json!({"method": "POST", "path": path, "body": body}),
    )
    .await
}

/// The grant `id` as Ada lists it.
async fn listed(world: &World, id: &str) -> Result<Value, Box<dyn std::error::Error>> {
    let (status, grants) = world.service.get("/grants", Some(&world.ada)).await?;
    assert_eq!(status, 200, "{grants}");
    grants["grants"]
        .as_array()
        .ok_or("no grants")?
        .iter()
        .find(|each| each["id"] == id)
        .cloned()
        .ok_or_else(|| "the grant is not listed".into())
}

#[tokio::test]
async fn a_stranger_holding_nothing_is_refused_by_the_acts_name() -> TestResult {
    let world = World::new().await?;
    let (status, refused) = read(&world, &world.stranger, "/people").await?;
    assert_eq!(status, 403, "{refused}");
    assert_eq!(refused["refusal"], "NotHeld", "{refused}");
    assert_eq!(refused["fields"]["action"], "read", "{refused}");
    assert_eq!(refused["fields"]["resource_kind"], "person", "{refused}");
    let own = format!("/agents/{}", world.stranger.id);
    let (status, refused) = read(&world, &world.stranger, &own).await?;
    assert_eq!(status, 403, "{refused}");
    assert_eq!(refused["refusal"], "NotHeld", "{refused}");
    Ok(())
}

#[tokio::test]
async fn administrator_standing_reads_the_directory_and_records_each_use() -> TestResult {
    let world = World::new().await?;
    let grant = standing(&world, &world.stranger, "administrator").await?;
    let (status, people) = read(&world, &world.stranger, "/people").await?;
    assert_eq!(status, 200, "{people}");
    assert_eq!(people["scope"], "directory", "{people}");
    let names: Vec<_> = people["people"]
        .as_array()
        .ok_or("no people")?
        .iter()
        .map(|person| person["id"].clone())
        .collect();
    assert!(
        names.contains(&json!(world.bea_person)),
        "Bea is of the organisation: {people}"
    );
    assert!(names.len() >= 2, "Ada and Bea at least: {people}");
    let own = format!("/agents/{}", world.stranger.id);
    let (status, agent) = read(&world, &world.stranger, &own).await?;
    assert_eq!(status, 200, "{agent}");
    assert_eq!(agent["id"], world.stranger.id, "{agent}");
    assert_eq!(agent["person"]["id"], world.bea_person, "{agent}");
    let (status, directory) = read(&world, &world.stranger, "/directory/people").await?;
    assert_eq!(status, 200, "{directory}");
    assert_eq!(directory["scope"], "directory", "{directory}");
    let held = listed(&world, &grant).await?;
    assert_eq!(held["last_use"]["seen"], true, "{held}");
    Ok(())
}

#[tokio::test]
async fn administrator_standing_issues_another_agents_certificate() -> TestResult {
    let world = World::new().await?;
    standing(&world, &world.stranger, "administrator").await?;
    let (_, request) = key_for(&world.service, &world.creator.id, "by-standing")?;
    let body = json!({"operation": operation()?, "request": request});
    let path = format!("/agents/{}/certificates", world.creator.id);
    let answer = world.signed(&world.stranger, &path, &body).await?;
    assert_eq!(answer.0, 200, "{}", answer.1);
    let serial = answer.1["recorded"]
        .as_str()
        .ok_or_else(|| format!("nothing recorded: {}", answer.1))?;
    let certificate = answer.1["certificates"]
        .as_array()
        .ok_or("no certificates")?
        .iter()
        .find(|certificate| certificate["serial"] == serial)
        .cloned()
        .ok_or("the certificate is not listed")?;
    assert_eq!(
        certificate["claims"]["issued_by"],
        json!({"actor": world.stranger.id, "responsible": world.bea_person}),
        "{certificate}"
    );
    Ok(())
}

#[tokio::test]
async fn reading_standing_reads_and_changes_nothing() -> TestResult {
    let world = World::new().await?;
    standing(&world, &world.stranger, "only.organisation.read").await?;
    let (status, people) = read(&world, &world.stranger, "/people").await?;
    assert_eq!(status, 200, "{people}");
    let body = json!({"operation": operation()?, "display_name": "Not registered"});
    let (status, refused) = change(&world, &world.stranger, "/agents", &body).await?;
    assert_eq!(status, 403, "{refused}");
    // Registering an agent is judged by its handler, which names the act.
    assert_eq!(refused["refusal"], "not_permitted", "{refused}");
    assert_eq!(refused["fields"]["action"], "agent.create", "{refused}");
    Ok(())
}

#[tokio::test]
async fn discovery_names_the_routes_the_standing_opens() -> TestResult {
    let world = World::new().await?;
    let (status, me) = world.service.get("/me", Some(&world.ada)).await?;
    assert_eq!(status, 200, "{me}");
    standing(&world, &world.stranger, "only.organisation.read").await?;
    let asked = json!({"jsonrpc": "2.0", "id": 1, "method": "tools/call",
        "params": {"name": "what-can-I-do", "arguments": {}}});
    let answer = world.mcp(&world.stranger, &asked).await?;
    let inventory = &mcp_answer::body(&answer["result"])["body"];
    let routes = inventory["routes"].as_array().ok_or("no routes")?;
    let people = routes
        .iter()
        .find(|route| route["method"] == "GET" && route["path"] == "/people")
        .ok_or_else(|| format!("GET /people is not listed: {inventory}"))?;
    assert_eq!(
        people["standing"],
        json!({"kind": "organisation", "id": me["organisation"]["id"], "action": "organisation.read"}),
        "{people}"
    );
    assert!(
        !routes
            .iter()
            .any(|route| route["method"] == "POST" && route["path"] == "/agents"),
        "reading opens no act: {inventory}"
    );
    Ok(())
}
