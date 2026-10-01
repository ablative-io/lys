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

#[tokio::test]
async fn role_create_records_the_granted_agent() -> TestResult {
    let table = Table::fresh(false).await?;
    let mut body = version_body()?;
    body["name"] = json!("Reviewer");
    refused(&table, Method::POST, "/roles", Some(&body)).await?;
    table.grant("all", "creator", "role.create").await?;
    let (status, answer) = table.call(Method::POST, "/roles", Some(&body)).await?;
    assert_eq!(status, 200, "{answer}");
    assert_eq!(answer["versions"][0]["made_by"], table.agent);
    let id = answer["id"].as_str().ok_or("role id missing")?;
    assert_eq!(
        table.role_view(id).await?["versions"][0]["made_by"],
        table.agent
    );
    Ok(())
}

#[tokio::test]
async fn role_revise_records_the_granted_agent_and_cannot_end_a_holding() -> TestResult {
    let table = Table::fresh(true).await?;
    let path = format!("/roles/{}/versions", table.role);
    let body = version_body()?;
    refused(&table, Method::POST, &path, Some(&body)).await?;
    table.grant(&table.role, "reviser", "role.revise").await?;
    let (status, answer) = table.call(Method::POST, &path, Some(&body)).await?;
    assert_eq!(status, 200, "{answer}");
    assert_eq!(answer["versions"][2]["made_by"], table.agent);
    assert_eq!(
        table.role_view(&table.role).await?["versions"][2]["made_by"],
        table.agent
    );
    let end = format!("/roles/{}/holders/{}/end", table.role, table.agent);
    refused(
        &table,
        Method::POST,
        &end,
        Some(&json!({"assignment":table.assignment})),
    )
    .await?;
    assert!(table.role_view(&table.role).await?["holders"][0]["ended_by"].is_null());
    Ok(())
}

#[tokio::test]
async fn role_assign_records_the_granted_agent() -> TestResult {
    let table = Table::fresh(false).await?;
    let path = format!("/roles/{}/holders", table.role);
    let body = json!({"operation":operation()?, "holder":table.agent, "ends_at":null});
    refused(&table, Method::POST, &path, Some(&body)).await?;
    table
        .grant(&table.role, "assigner", "role.holder.assign")
        .await?;
    let (status, answer) = table.call(Method::POST, &path, Some(&body)).await?;
    assert_eq!(status, 200, "{answer}");
    assert_eq!(answer["holders"][0]["assigned_by"], table.agent);
    assert_eq!(
        table.role_view(&table.role).await?["holders"][0]["assigned_by"],
        table.agent
    );
    Ok(())
}

#[tokio::test]
async fn role_move_records_the_granted_agent() -> TestResult {
    let table = Table::fresh(true).await?;
    let path = format!("/roles/{}/holders/{}/move", table.role, table.agent);
    let body = json!({"assignment":table.assignment, "from_version":1, "to_version":2});
    refused(&table, Method::POST, &path, Some(&body)).await?;
    table
        .grant(&table.role, "mover", "role.holder.move")
        .await?;
    let (status, answer) = table.call(Method::POST, &path, Some(&body)).await?;
    assert_eq!(status, 200, "{answer}");
    assert_eq!(answer["holder"]["moves"][0]["by"], table.agent);
    assert_eq!(
        table.role_view(&table.role).await?["holders"][0]["moves"][0]["by"],
        table.agent
    );
    Ok(())
}

#[tokio::test]
async fn role_end_records_the_granted_agent() -> TestResult {
    let table = Table::fresh(true).await?;
    let path = format!("/roles/{}/holders/{}/end", table.role, table.agent);
    let body = json!({"assignment":table.assignment});
    refused(&table, Method::POST, &path, Some(&body)).await?;
    table.grant(&table.role, "ender", "role.holder.end").await?;
    let (status, answer) = table.call(Method::POST, &path, Some(&body)).await?;
    assert_eq!(status, 200, "{answer}");
    assert_eq!(answer["holders"][0]["ended_by"], table.agent);
    assert_eq!(
        table.role_view(&table.role).await?["holders"][0]["ended_by"],
        table.agent
    );
    Ok(())
}
