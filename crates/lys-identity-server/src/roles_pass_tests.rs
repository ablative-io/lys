use std::error::Error;

use reqwest::Method;
use serde_json::{Value, json};

#[path = "roles_pass_fixture.rs"]
mod fixture;
use fixture::{Table, operation, version_body};

type TestResult = Result<(), Box<dyn Error>>;

async fn refused(table: &Table, method: Method, path: &str, body: Option<&Value>) -> TestResult {
    let (status, answer) = table.call(method, path, body).await?;
    assert_eq!(status, 403, "{answer}");
    assert_eq!(answer["refusal"], "NotHeld", "{answer}");
    Ok(())
}

#[tokio::test]
async fn role_list_requires_its_collection_read_grant() -> TestResult {
    let table = Table::fresh(false).await?;
    refused(&table, Method::GET, "/roles", None).await?;
    table.grant("all", "reader", "read").await?;
    let (status, answer) = table.call(Method::GET, "/roles", None).await?;
    assert_eq!(status, 200, "{answer}");
    assert_eq!(answer["roles"][0]["id"], table.role);
    Ok(())
}

#[tokio::test]
async fn role_read_requires_the_named_roles_read_grant() -> TestResult {
    let table = Table::fresh(false).await?;
    let path = format!("/roles/{}", table.role);
    refused(&table, Method::GET, &path, None).await?;
    table.grant(&table.role, "reader", "read").await?;
    let other = format!("/roles/{}", operation()?);
    refused(&table, Method::GET, &other, None).await?;
    let (status, answer) = table.call(Method::GET, &path, None).await?;
    assert_eq!(status, 200, "{answer}");
    assert_eq!(answer["id"], table.role);
    Ok(())
}

/// Creating, revising, assigning, moving and ending roles are a person's to
/// give an agent (AGENTS-019 R1): before each is given the agent is refused
/// NotHeld at its route, and once given the agent does it.
#[tokio::test]
async fn role_changes_are_given_to_an_agent_and_refused_until_they_are() -> TestResult {
    let table = Table::fresh(true).await?;
    let mut create = version_body()?;
    create["name"] = json!("Reviewer");
    let holder = format!("/roles/{}/holders/{}", table.role, table.agent);
    let cases = [
        ("all", "creator", "role.create", "/roles".to_owned(), create),
        (
            table.role.as_str(),
            "reviser",
            "role.revise",
            format!("/roles/{}/versions", table.role),
            version_body()?,
        ),
        (
            table.role.as_str(),
            "assigner",
            "role.holder.assign",
            format!("/roles/{}/holders", table.role),
            json!({"operation":operation()?, "holder":table.agent, "ends_at":null}),
        ),
        (
            table.role.as_str(),
            "mover",
            "role.holder.move",
            format!("{holder}/move"),
            json!({"assignment":table.assignment, "from_version":1, "to_version":2}),
        ),
        (
            table.role.as_str(),
            "ender",
            "role.holder.end",
            format!("{holder}/end"),
            json!({"assignment":table.assignment}),
        ),
    ];
    for (resource, relation, action, path, body) in cases {
        refused(&table, Method::POST, &path, Some(&body)).await?;
        let (status, answer) = table.give(resource, relation, action).await?;
        assert_eq!(status, 200, "{action}: {answer}");
        let (status, answer) = table.call(Method::POST, &path, Some(&body)).await?;
        assert_eq!(status, 200, "{action}: {answer}");
    }
    let view = table.role_view(&table.role).await?;
    let ended = view["holders"]
        .as_array()
        .ok_or("no holders")?
        .iter()
        .any(|holder| holder["ended_by"] == table.agent.as_str());
    assert!(
        ended,
        "the agent ended the assignment it was given to end: {view}"
    );
    Ok(())
}

/// Holding a role whose templates name authority gives an agent none of it:
/// assignment issues no grant, so the agent holds nothing and stays refused.
#[tokio::test]
async fn a_held_role_with_authority_templates_gives_an_agent_nothing() -> TestResult {
    let table = Table::fresh(false).await?;
    let mut body = version_body()?;
    body["name"] = json!("Administrator's deputy");
    body["grant_templates"] = json!([
        {"resource": {"kind": "role", "id": "all"}, "relation": "creator", "days": null}
    ]);
    let (status, role) = table.as_administrator("/roles", &body).await?;
    assert_eq!(status, 200, "{role}");
    let id = role["id"].as_str().ok_or("role id missing")?;
    let (status, held) = table
        .as_administrator(
            &format!("/roles/{id}/holders"),
            &json!({"operation": operation()?, "holder": table.agent, "ends_at": null}),
        )
        .await?;
    assert_eq!(status, 200, "{held}");
    let (status, grants) = table.administrator_get("/grants").await?;
    assert_eq!(status, 200, "{grants}");
    let held: Vec<&Value> = grants["grants"]
        .as_array()
        .ok_or("no grants")?
        .iter()
        .filter(|grant| grant["holder"] == table.agent.as_str())
        .collect();
    assert!(held.is_empty(), "{held:?}");
    let mut create = version_body()?;
    create["name"] = json!("Made by the agent");
    refused(&table, Method::POST, "/roles", Some(&create)).await?;
    Ok(())
}
