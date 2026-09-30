#![cfg(test)]
//! MCP requests refuse servers absent from the reviewed profile.

#[path = "support/harness_description.rs"]
mod harness_description;

use std::error::Error;

use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_identity::OperationId;
use lys_identity_server::dev_seed::seed_configured;
use serde_json::{Value, json};

async fn post(
    service: &Service,
    path: &str,
    cookie: &str,
    body: &Value,
) -> Result<(u16, Value), Box<dyn Error>> {
    let response = reqwest::Client::new()
        .post(format!("{}{path}", service.base))
        .header(reqwest::header::COOKIE, cookie)
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body(body.to_string())
        .send()
        .await?;
    let status = response.status().as_u16();
    let text = response.text().await?;
    let answer = serde_json::from_str(&text).map_err(|error| {
        format!(
            "POST {path} answered {status} with {} body bytes instead of JSON: {error}",
            text.len()
        )
    })?;
    Ok((status, answer))
}

#[tokio::test]
async fn an_undeclared_mcp_server_is_refused_by_name() -> Result<(), Box<dyn Error>> {
    let (service, seeded) = Service::start_with(|config| {
        Ok(seed_configured(config, [ADMINISTRATOR, "other-subject"])?)
    })
    .await?;
    let cookie = service
        .sign_in(Login {
            subject: ADMINISTRATOR.to_owned(),
            email: "operator@example.test".to_owned(),
        })
        .await?;
    let agent = seeded.people[0].agents[0].id.to_string();
    let profile = format!("/agents/{agent}/provisioning");
    let (status, recorded) = post(
        &service,
        &profile,
        &cookie,
        &json!({
            "operation": OperationId::generate()?.to_string(), "from_version": 0,
            "model_access": ["primary-model"], "tools": [], "skills": [],
            "mcp_servers": [], "instructions": "", "note": "",
            "harness": harness_description::declared(),
        }),
    )
    .await?;
    assert_eq!(status, 200, "{recorded}");
    let (status, reviewed) = post(
        &service,
        &format!("{profile}/1/review"),
        &cookie,
        &json!({"operation": OperationId::generate()?.to_string()}),
    )
    .await?;
    assert_eq!(status, 200, "{reviewed}");
    let path = format!("/agents/{agent}/mcp-requests");
    let (status, refused) = post(
        &service,
        &path,
        &cookie,
        &json!({
            "operation": OperationId::generate()?.to_string(), "server": "undeclared-server",
        }),
    )
    .await?;
    assert_eq!(status, 404, "{refused}");
    assert_eq!(refused["refusal"], "mcp_server_unknown", "{refused}");
    assert!(refused.to_string().contains("undeclared-server"));
    Ok(())
}
