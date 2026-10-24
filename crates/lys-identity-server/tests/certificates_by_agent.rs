#![cfg(test)]

//! An agent holding `agent.certificate.issue` issues another agent's
//! certificate (24 October 2026). A stranger holding nothing is refused by
//! the registrar rule's name, as before; given the act on the person the
//! agent answers to, the same call issues a further certificate, the use is
//! recorded, and the certificate names the issuing agent and the person
//! responsible. The MCP relay answers the same as a signed call.

#[path = "support/agent_starts_agent.rs"]
pub mod support;

#[path = "../src/mcp_test_answer.rs"]
mod mcp_answer;

use serde_json::{Value, json};
use support::{Answer, Certified, TestResult, World, key_for, operation, refused_as};

const ACT: &str = "agent.certificate.issue";

/// A request for a fresh key of `agent`, named `name` beside the service.
fn request_for(
    world: &World,
    agent: &str,
    name: &str,
) -> Result<Value, Box<dyn std::error::Error>> {
    let (_, request) = key_for(&world.service, agent, name)?;
    Ok(json!({"operation": operation()?, "request": request}))
}

/// `caller`'s signed ask for a certificate of `agent` over a fresh key.
async fn asked(
    world: &World,
    caller: &Certified,
    agent: &str,
    name: &str,
) -> Result<Answer, Box<dyn std::error::Error>> {
    let body = request_for(world, agent, name)?;
    world
        .signed(caller, &format!("/agents/{agent}/certificates"), &body)
        .await
}

/// The certificate recorded by `answer`, as the view lists it.
fn recorded(answer: &Value) -> Result<Value, Box<dyn std::error::Error>> {
    let serial = answer["recorded"]
        .as_str()
        .ok_or_else(|| format!("nothing recorded: {answer}"))?;
    answer["certificates"]
        .as_array()
        .ok_or_else(|| format!("no certificates: {answer}"))?
        .iter()
        .find(|certificate| certificate["serial"] == serial)
        .cloned()
        .ok_or_else(|| format!("{serial} not listed: {answer}").into())
}

#[tokio::test]
async fn a_stranger_holding_nothing_is_refused_by_name() -> TestResult {
    let world = World::new().await?;
    let answer = asked(&world, &world.stranger, &world.creator.id, "refused").await?;
    assert_eq!(answer.0, 403, "{}", answer.1);
    assert!(refused_as(&answer, "agent_not_yours"), "{}", answer.1);
    Ok(())
}

#[tokio::test]
async fn an_agent_given_the_act_on_the_person_issues_a_further_certificate() -> TestResult {
    let world = World::new().await?;
    let grant = world
        .give(&world.stranger.id, &world.bea_resource(), ACT, false)
        .await?;
    let answer = asked(&world, &world.stranger, &world.creator.id, "again").await?;
    assert_eq!(answer.0, 200, "{}", answer.1);
    let certificate = recorded(&answer.1)?;
    assert_eq!(certificate["person"], world.bea_person, "{certificate}");
    assert_eq!(
        certificate["claims"]["issued_by"],
        json!({"actor": world.stranger.id, "responsible": world.bea_person}),
        "{certificate}"
    );
    let listed = answer.1["certificates"]
        .as_array()
        .ok_or("no certificates")?;
    assert!(
        listed.len() >= 2,
        "a further certificate, not the first: {}",
        answer.1
    );
    // The use is recorded against the grant that admitted it.
    let held = world
        .beas_grants()
        .await?
        .into_iter()
        .find(|each| each["id"] == grant)
        .ok_or("the grant is not listed")?;
    assert_eq!(held["last_use"]["seen"], true, "{held}");
    Ok(())
}

#[tokio::test]
async fn the_mcp_relay_issues_the_same() -> TestResult {
    let world = World::new().await?;
    world
        .give(&world.stranger.id, &world.bea_resource(), ACT, false)
        .await?;
    let path = format!("/agents/{}/certificates", world.reviewer.id);
    let body = request_for(&world, &world.reviewer.id, "relayed")?;
    let asked = json!({"jsonrpc": "2.0", "id": 1, "method": "tools/call",
        "params": {"name": "change", "arguments": {"method": "POST", "path": path, "body": body}}});
    let answer = world.mcp(&world.stranger, &asked).await?;
    let relayed = mcp_answer::body(&answer["result"]);
    assert_eq!(relayed["status"], 200, "{answer}");
    let certificate = recorded(&relayed["body"])?;
    assert_eq!(
        certificate["claims"]["issued_by"]["actor"], world.stranger.id,
        "{certificate}"
    );
    Ok(())
}

#[tokio::test]
async fn the_act_on_another_agent_opens_nothing() -> TestResult {
    let world = World::new().await?;
    // The act on the stranger's own record reaches neither the creator nor
    // the person they both answer to.
    let own = json!({"kind": "agent", "id": world.stranger.id});
    world.give(&world.stranger.id, &own, ACT, false).await?;
    let answer = asked(&world, &world.stranger, &world.creator.id, "elsewhere").await?;
    assert_eq!(answer.0, 403, "{}", answer.1);
    assert!(refused_as(&answer, "agent_not_yours"), "{}", answer.1);
    Ok(())
}
