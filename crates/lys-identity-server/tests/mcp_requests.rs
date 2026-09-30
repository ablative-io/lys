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

async fn version(
    service: &Service,
    cookie: &str,
    agent: &str,
    from: u32,
    servers: Value,
    reviewed: bool,
) -> Result<(), Box<dyn Error>> {
    let path = format!("/agents/{agent}/provisioning");
    let (status, recorded) = post(
        service,
        &path,
        cookie,
        &json!({
            "operation": OperationId::generate()?.to_string(), "from_version": from,
            "model_access": ["primary-model"], "tools": [], "skills": [],
            "mcp_servers": servers, "instructions": "", "note": "",
            "harness": harness_description::declared(),
        }),
    )
    .await?;
    assert_eq!(status, 200, "{recorded}");
    if reviewed {
        let (status, answer) = post(
            service,
            &format!("{path}/{}/review", from + 1),
            cookie,
            &json!({"operation": OperationId::generate()?.to_string()}),
        )
        .await?;
        assert_eq!(status, 200, "{answer}");
    }
    Ok(())
}

fn declared_server(name: &str) -> Value {
    json!([{"name": name, "url": "https://tools.example.test/mcp"}])
}

async fn without_request_storage(
    service: &mut Service,
    path: &str,
    cookie: &str,
) -> Result<(), Box<dyn Error>> {
    service
        .restart_adjusted(|config| config.requests_dir = None)
        .await?;
    let answer = service.get(path, Some(cookie)).await?;
    assert_eq!(answer.0, 503, "{}", answer.1);
    assert_eq!(
        answer.1["refusal"], "McpRequestsUnavailable",
        "{}",
        answer.1
    );
    let answer = post(
        service,
        path,
        cookie,
        &json!({
            "operation": OperationId::generate()?.to_string(), "server": "dot",
        }),
    )
    .await?;
    assert_eq!(answer.0, 503, "{}", answer.1);
    assert_eq!(
        answer.1["refusal"], "McpRequestsUnavailable",
        "{}",
        answer.1
    );
    Ok(())
}

#[tokio::test]
async fn a_declared_server_request_keeps_the_reviewed_version_and_reads_back_pending()
-> Result<(), Box<dyn Error>> {
    let (mut service, seeded) = Service::start_with(|config| {
        Ok(seed_configured(config, [ADMINISTRATOR, "holder-subject"])?)
    })
    .await?;
    let administrator = service
        .sign_in(Login {
            subject: ADMINISTRATOR.to_owned(),
            email: "operator@example.test".to_owned(),
        })
        .await?;
    let holder = service
        .sign_in(Login {
            subject: "holder-subject".to_owned(),
            email: "holder@example.test".to_owned(),
        })
        .await?;
    let donor = seeded.people[0].agents[0].id.to_string();
    let agent = seeded.people[1].agents[0].id.to_string();
    version(
        &service,
        &administrator,
        &donor,
        0,
        declared_server("dot"),
        true,
    )
    .await?;
    version(&service, &administrator, &agent, 0, json!([]), true).await?;
    version(
        &service,
        &administrator,
        &agent,
        1,
        declared_server("dot"),
        false,
    )
    .await?;
    let path = format!("/agents/{agent}/mcp-requests");
    let body = json!({"operation": OperationId::generate()?.to_string(), "server": "dot"});
    let (status, asked) = post(&service, &path, &holder, &body).await?;
    assert_eq!(status, 200, "{asked}");
    assert!(
        asked["id"].as_str().is_some_and(|id| !id.is_empty()),
        "{asked}"
    );
    assert_eq!(asked["agent"], agent, "{asked}");
    assert_eq!(asked["server"], "dot", "{asked}");
    assert_eq!(asked["profile_version"], 1, "{asked}");
    assert_eq!(asked["state"], "pending", "{asked}");
    assert_eq!(
        asked["asked_by"],
        seeded.people[1].id.to_string(),
        "{asked}"
    );
    assert!(
        asked["asked_at"].as_u64().is_some_and(|at| at > 0),
        "{asked}"
    );
    let (status, replayed) = post(&service, &path, &holder, &body).await?;
    assert_eq!(status, 200, "{replayed}");
    assert_eq!(replayed, asked);
    let mut changed = body.clone();
    changed["server"] = json!("other-server");
    let (status, reused) = post(&service, &path, &holder, &changed).await?;
    assert_eq!(status, 409, "{reused}");
    assert_eq!(reused["refusal"], "RequestReused", "{reused}");
    let (status, listed) = service.get(&path, Some(&holder)).await?;
    assert_eq!(status, 200, "{listed}");
    assert_eq!(listed, json!({"requests": [asked.clone()]}));
    let profile = format!("/agents/{agent}/provisioning");
    let (status, before) = service.get(&profile, Some(&holder)).await?;
    assert_eq!(status, 200, "{before}");
    assert_eq!(
        before["versions"]
            .as_array()
            .ok_or("versions missing")?
            .len(),
        2
    );
    service.restart().await?;
    let holder = service
        .sign_in(Login {
            subject: "holder-subject".to_owned(),
            email: "holder@example.test".to_owned(),
        })
        .await?;
    let (status, listed) = service.get(&path, Some(&holder)).await?;
    assert_eq!(status, 200, "{listed}");
    assert_eq!(listed, json!({"requests": [asked.clone()]}));
    let (status, replayed) = post(&service, &path, &holder, &body).await?;
    assert_eq!(status, 200, "{replayed}");
    assert_eq!(replayed, asked);
    let (status, after) = service.get(&profile, Some(&holder)).await?;
    assert_eq!(status, 200, "{after}");
    assert_eq!(after, before);
    without_request_storage(&mut service, &path, &holder).await?;
    Ok(())
}

#[tokio::test]
async fn a_server_held_by_the_latest_reviewed_version_is_refused_by_name()
-> Result<(), Box<dyn Error>> {
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
    version(&service, &cookie, &agent, 0, declared_server("dot"), true).await?;
    version(
        &service,
        &cookie,
        &agent,
        1,
        declared_server("unreviewed-server"),
        false,
    )
    .await?;
    let path = format!("/agents/{agent}/mcp-requests");
    let (status, held) = post(
        &service,
        &path,
        &cookie,
        &json!({"operation": OperationId::generate()?.to_string(), "server": "dot"}),
    )
    .await?;
    assert_eq!(status, 409, "{held}");
    assert_eq!(held["refusal"], "mcp_server_held", "{held}");
    assert!(held.to_string().contains("dot"), "{held}");
    let (status, unknown) = post(
        &service,
        &path,
        &cookie,
        &json!({"operation": OperationId::generate()?.to_string(), "server": "unreviewed-server"}),
    )
    .await?;
    assert_eq!(status, 404, "{unknown}");
    assert_eq!(unknown["refusal"], "mcp_server_unknown", "{unknown}");
    assert!(
        unknown.to_string().contains("unreviewed-server"),
        "{unknown}"
    );
    let (status, listed) = service.get(&path, Some(&cookie)).await?;
    assert_eq!(status, 200, "{listed}");
    assert_eq!(listed, json!({"requests": []}));
    Ok(())
}

#[tokio::test]
async fn an_agent_with_only_an_unreviewed_profile_cannot_record_a_request()
-> Result<(), Box<dyn Error>> {
    let (service, seeded) = Service::start_with(|config| {
        Ok(seed_configured(config, [ADMINISTRATOR, "holder-subject"])?)
    })
    .await?;
    let administrator = service
        .sign_in(Login {
            subject: ADMINISTRATOR.to_owned(),
            email: "operator@example.test".to_owned(),
        })
        .await?;
    let holder = service
        .sign_in(Login {
            subject: "holder-subject".to_owned(),
            email: "holder@example.test".to_owned(),
        })
        .await?;
    let donor = seeded.people[0].agents[0].id.to_string();
    let agent = seeded.people[1].agents[0].id.to_string();
    version(
        &service,
        &administrator,
        &donor,
        0,
        declared_server("dot"),
        true,
    )
    .await?;
    version(&service, &administrator, &agent, 0, json!([]), false).await?;
    let path = format!("/agents/{agent}/mcp-requests");
    let (status, refused) = post(
        &service,
        &path,
        &holder,
        &json!({
            "operation": OperationId::generate()?.to_string(), "server": "dot",
        }),
    )
    .await?;
    assert_eq!(status, 409, "{refused}");
    assert_eq!(refused["refusal"], "ProfileNotReviewed", "{refused}");
    let reason = refused["reason"]
        .as_str()
        .ok_or("the refusal has no reason")?;
    assert!(
        reason.contains("version 1") && reason.contains("not reviewed"),
        "{refused}"
    );
    let (status, listed) = service.get(&path, Some(&holder)).await?;
    assert_eq!(status, 200, "{listed}");
    assert_eq!(listed, json!({"requests": []}));
    Ok(())
}
