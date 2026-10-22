#![cfg(test)]

//! AGENTS-019 R4: an agent reaches what its person reaches. Every route a
//! person reaches is exactly one of three: admitted to an agent (judged at
//! the door by its declared scope, or by its own handler), a person's alone
//! on `HUMAN_ONLY` with its reason, or a responsibility kept from every
//! agent. A route an agent is given is judged by the agent's own grants: a
//! stranger holding nothing is refused by the act's name, and the same call
//! answers once the act is given. Discovery names the routes a held grant
//! opens, and an agent never decides a draft it made itself.

#[path = "support/agent_starts_agent.rs"]
mod support;

#[path = "../src/mcp_test_answer.rs"]
mod mcp_answer;

use std::collections::BTreeSet;

use lys_identity_server::openapi::{HUMAN_ONLY, reaches};
use serde_json::{Value, json};
use support::{Certified, TestResult, World, operation};

/// The routes kept from every agent, as `(method, path)`, the method upper case.
fn kept() -> Result<BTreeSet<(String, String)>, Box<dyn std::error::Error>> {
    let listed: Vec<Value> =
        serde_json::from_str(include_str!("../src/kept-responsibilities.json"))?;
    listed
        .iter()
        .map(|row| {
            let method = row["method"].as_str().ok_or("a kept row names no method")?;
            let path = row["path"].as_str().ok_or("a kept row names no path")?;
            Ok((method.to_ascii_uppercase(), path.to_owned()))
        })
        .collect()
}

#[test]
fn every_route_a_person_reaches_is_an_agents_or_named_human_only_or_kept() -> TestResult {
    let kept = kept()?;
    let human: BTreeSet<(String, String)> = HUMAN_ONLY
        .iter()
        .map(|(method, path, _)| (method.word().to_ascii_uppercase(), (*path).to_owned()))
        .collect();
    let routes = reaches();
    let mut wrong = Vec::new();
    for route in routes.iter().filter(|route| route.person) {
        let key = (route.method.to_ascii_uppercase(), route.path.to_owned());
        let sets = [
            route.agent && (route.scoped || route.handler),
            human.contains(&key),
            kept.contains(&key),
        ];
        let count = sets.iter().filter(|held| **held).count();
        if count != 1 {
            wrong.push(format!("{} {} is in {count} of the three", key.0, key.1));
        }
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
    let mut unnamed = Vec::new();
    for (method, path, reason) in HUMAN_ONLY {
        if reason.trim().is_empty() {
            unnamed.push(format!("{} {path} gives no reason", method.word()));
        }
        if !routes
            .iter()
            .any(|route| route.method == method.word() && route.path == *path)
        {
            unnamed.push(format!("{} {path} is not a route", method.word()));
        }
    }
    assert!(unnamed.is_empty(), "{unnamed:#?}");
    Ok(())
}

/// `agent`'s MCP `change` of `path` with `body`: the relayed route's status and answer.
async fn change(
    world: &World,
    agent: &Certified,
    path: &str,
    body: &Value,
) -> Result<(u64, Value), Box<dyn std::error::Error>> {
    let asked = json!({"jsonrpc": "2.0", "id": 1, "method": "tools/call",
        "params": {"name": "change", "arguments": {"method": "POST", "path": path, "body": body}}});
    let answer = world.mcp(agent, &asked).await?;
    let relayed = mcp_answer::body(&answer["result"]);
    let status = relayed["status"]
        .as_u64()
        .ok_or_else(|| format!("no status: {answer}"))?;
    Ok((status, relayed["body"].clone()))
}

#[tokio::test]
async fn an_agent_sets_words_only_on_its_own_grant() -> TestResult {
    let world = World::new().await?;
    let path = format!("/agents/{}/words/context_warning", world.creator.id);
    let body = json!({"setting": {"kind": "text", "text": "Mind the context."}, "revision": 0});
    let (status, refused) = change(&world, &world.stranger, &path, &body).await?;
    assert_eq!(status, 403, "{refused}");
    assert_eq!(refused["refusal"], "NotHeld", "{refused}");
    assert_eq!(refused["fields"]["action"], "words.set", "{refused}");
    assert_eq!(refused["fields"]["scope_declared"], true, "{refused}");
    let words = json!({"kind": "words", "id": world.creator.id});
    world
        .give(&world.stranger.id, &words, "words.set", false)
        .await?;
    let (status, saved) = change(&world, &world.stranger, &path, &body).await?;
    assert_eq!(status, 200, "{saved}");
    assert!(saved["revision"].as_u64().is_some(), "{saved}");
    Ok(())
}

#[tokio::test]
async fn discovery_names_the_admission_a_held_grant_opens() -> TestResult {
    let world = World::new().await?;
    let (status, me) = world.service.get("/me", Some(&world.bea)).await?;
    assert_eq!(status, 200, "{me}");
    let organisation = json!({"kind": "organisation", "id": me["organisation"]["id"]});
    world
        .give(&world.stranger.id, &organisation, "organisation.admit", false)
        .await?;
    let asked = json!({"jsonrpc": "2.0", "id": 1, "method": "tools/call",
        "params": {"name": "what-can-I-do", "arguments": {}}});
    let answer = world.mcp(&world.stranger, &asked).await?;
    let inventory = &mcp_answer::body(&answer["result"])["body"];
    let routes = inventory["routes"].as_array().ok_or("no routes")?;
    assert!(
        routes.iter().any(|route| route["path"] == "/people/admit"),
        "{inventory}"
    );
    Ok(())
}

#[tokio::test]
async fn an_agent_never_decides_its_own_draft() -> TestResult {
    let world = World::new().await?;
    let drafted = json!({
        "operation": operation()?,
        "target": {"kind": "person", "id": world.bea_person, "action": "person.profile.set"},
        "method": "POST", "path": format!("/identities/{}/profile", world.bea_person),
        "body": "{}", "note": "a draft for Bea to decide",
    });
    let (status, created) = world.signed(&world.stranger, "/drafts", &drafted).await?;
    assert_eq!(status, 200, "{created}");
    let draft = created["draft"].as_str().ok_or("no draft")?.to_owned();
    let hash = created["creation_hash"]
        .as_str()
        .ok_or("no creation hash")?
        .to_owned();
    let decide = json!({"kind": "draft", "id": draft});
    let path = format!("/drafts/{draft}/refuse");
    let refusal = || -> Result<Value, Box<dyn std::error::Error>> {
        Ok(json!({"operation": operation()?, "creation_hash": hash, "reason": "not now"}))
    };
    world
        .give(&world.stranger.id, &decide, "draft.decide", false)
        .await?;
    let (status, own) = change(&world, &world.stranger, &path, &refusal()?).await?;
    assert_eq!(status, 403, "{own}");
    assert_eq!(own["refusal"], "own_draft", "{own}");
    world
        .give(&world.reviewer.id, &decide, "draft.decide", false)
        .await?;
    let (status, refused) = change(&world, &world.reviewer, &path, &refusal()?).await?;
    assert_eq!(status, 200, "{refused}");
    assert_eq!(refused["draft"], draft.as_str(), "{refused}");
    Ok(())
}
