//! The creator issues the new agent's first certificate, and only the
//! first (AGENTS-016 R4): admitted by the registration record naming the
//! caller as the agent that registered it, by the caller's live grant of
//! `agent.create` from the person read at that moment, and by the agent
//! holding no certificate at all; every later certificate and every
//! withdrawal stays a person's.

use serde_json::{Value, json};

use super::support::{Answer, Certified, TestResult, World, key_for, operation, refused_as};
use super::{ok, registered};

/// `by`'s signed ask for a certificate of `agent` over a key kept as `name`;
/// the answer and the key's holder.
async fn asked(
    world: &World,
    by: &Certified,
    agent: &str,
    name: &str,
) -> Result<(Answer, Certified), Box<dyn std::error::Error>> {
    let (key, request) = key_for(&world.service, agent, name)?;
    let body = json!({"operation": operation()?, "request": request});
    let answer = world
        .signed(by, &format!("/agents/{agent}/certificates"), &body)
        .await?;
    Ok((answer, key))
}

/// The certificates Bea reads for `agent`.
async fn certificates(
    world: &World,
    agent: &str,
) -> Result<Vec<Value>, Box<dyn std::error::Error>> {
    let (status, read) = world
        .service
        .get(&format!("/agents/{agent}/certificates"), Some(&world.bea))
        .await?;
    assert_eq!(status, 200, "{read}");
    Ok(read["certificates"]
        .as_array()
        .ok_or("no certificates")?
        .clone())
}

fn refused(answer: &Answer, name: &str) {
    assert!(
        matches!(answer.0, 403 | 409),
        "{name}: {} {}",
        answer.0,
        answer.1
    );
    assert!(refused_as(answer, name), "{name}: {}", answer.1);
}

#[tokio::test]
async fn the_creator_issues_the_first_certificate_and_never_a_second() -> TestResult {
    let world = World::new().await?;
    world
        .give(
            &world.creator.id,
            &world.bea_resource(),
            "agent.create",
            false,
        )
        .await?;
    let agent = registered(&world).await?;

    let (answer, new) = asked(&world, &world.creator, &agent, "first").await?;
    let view = ok(&answer)?;
    let serial = view["recorded"].as_str().ok_or("no serial")?.to_owned();
    let held = certificates(&world, &agent).await?;
    assert_eq!(held.len(), 1, "{held:?}");
    assert_eq!(held[0]["person"], world.bea_person.as_str(), "{held:?}");
    assert_eq!(
        held[0]["claims"]["issued_by"],
        json!({"actor": world.creator.id, "responsible": world.bea_person}),
        "{held:?}"
    );
    // The new agent's own signed call is admitted.
    let asked_mcp = json!({"jsonrpc": "2.0", "id": 1, "method": "tools/call",
        "params": {"name": "what-can-I-do", "arguments": {}}});
    let reply = world.mcp(&new, &asked_mcp).await?;
    assert!(reply["error"].is_null(), "{reply}");
    assert!(!reply["result"].is_null(), "{reply}");

    // A second is refused; so is one after the person withdraws the first.
    let (second, _) = asked(&world, &world.creator, &agent, "second").await?;
    refused(&second, "certificate_not_first");
    let withdrawal = format!("/agents/{agent}/certificates/{serial}/withdrawal");
    let (status, withdrawn) = world
        .service
        .post(
            &withdrawal,
            Some(&world.ada),
            &json!({"reason": "replaced"}),
        )
        .await?;
    assert_eq!(status, 200, "{withdrawn}");
    let (third, _) = asked(&world, &world.creator, &agent, "third").await?;
    refused(&third, "certificate_not_first");
    // A withdrawal is never an agent's.
    let by_agent = world
        .signed(&world.creator, &withdrawal, &json!({"reason": "mine"}))
        .await?;
    assert_eq!(by_agent.0, 403, "{}", by_agent.1);
    assert!(refused_as(&by_agent, "not_permitted"), "{}", by_agent.1);
    Ok(())
}

#[tokio::test]
async fn only_the_registrar_issues_the_first_certificate() -> TestResult {
    let world = World::new().await?;
    world
        .give(
            &world.creator.id,
            &world.bea_resource(),
            "agent.create",
            false,
        )
        .await?;
    world
        .give(
            &world.stranger.id,
            &world.bea_resource(),
            "agent.create",
            false,
        )
        .await?;
    let agent = registered(&world).await?;
    // Another agent of Bea's holding agent.create, not its registrar.
    let (other, _) = asked(&world, &world.stranger, &agent, "other").await?;
    refused(&other, "agent_not_yours");
    // An agent the creator did not register.
    let reviewer = world.reviewer.id.clone();
    let (not_its, _) = asked(&world, &world.creator, &reviewer, "not-its").await?;
    refused(&not_its, "agent_not_yours");
    assert!(certificates(&world, &agent).await?.is_empty());
    Ok(())
}

#[tokio::test]
async fn a_revoked_agent_create_never_finishes_the_identity_it_began() -> TestResult {
    let world = World::new().await?;
    let create = world
        .give(
            &world.creator.id,
            &world.bea_resource(),
            "agent.create",
            false,
        )
        .await?;
    let agent = registered(&world).await?;
    let reason = json!({"operation": operation()?, "route": "api", "reason": "withdrawn"});
    let (status, revoked) = world
        .service
        .post(
            &format!("/grants/{create}/revoke"),
            Some(&world.bea),
            &reason,
        )
        .await?;
    assert_eq!(status, 200, "{revoked}");
    let (answer, _) = asked(&world, &world.creator, &agent, "late").await?;
    refused(&answer, "agent_create_revoked");
    assert!(certificates(&world, &agent).await?.is_empty());
    // A person still issues it.
    let (_, request) = key_for(&world.service, &agent, "by-bea")?;
    let body = json!({"operation": operation()?, "request": request});
    let (status, issued) = world
        .service
        .post(
            &format!("/agents/{agent}/certificates"),
            Some(&world.bea),
            &body,
        )
        .await?;
    assert_eq!(status, 200, "{issued}");
    assert_eq!(certificates(&world, &agent).await?.len(), 1);
    Ok(())
}
