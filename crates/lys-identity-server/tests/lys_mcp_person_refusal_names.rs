//! Missing MCP authority names the route and the exact scope the caller needs.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use identity_contract::apps::issued;
use identity_contract::harness::{ADMINISTRATOR, Service};
use reqwest::{Client, redirect::Policy};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::error::Error;

type Result<T = ()> = std::result::Result<T, Box<dyn Error>>;
const REDIRECT: &str = "https://app.example/callback";
const VERIFIER: &str = "a-verifier-of-at-least-forty-three-characters-long";

async fn connection(service: &Service, cookie: &str, client: &Client) -> Result<Value> {
    let response = client
        .post(format!("{}/oauth/mcp/register", service.base))
        .json(&json!({"client_name":"Refusal names", "redirect_uris":[REDIRECT]}))
        .send()
        .await?;
    assert_eq!(response.status(), 201);
    let registered: Value = response.json().await?;
    let id = registered["client_id"].as_str().ok_or("no client id")?;
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(VERIFIER.as_bytes()));
    let response = client
        .get(format!("{}/oauth/mcp/authorize", service.base))
        .header("cookie", cookie)
        .query(&[
            ("response_type", "code"),
            ("client_id", id),
            ("redirect_uri", REDIRECT),
            ("code_challenge", challenge.as_str()),
            ("code_challenge_method", "S256"),
        ])
        .send()
        .await?;
    assert_eq!(response.status(), 200);
    let page = response.text().await?;
    let asking = page
        .split("name=\"asking\" value=\"")
        .nth(1)
        .and_then(|rest| rest.split('"').next())
        .ok_or("no consent form")?;
    let response = client
        .post(format!("{}/oauth/mcp/consent", service.base))
        .header("cookie", cookie)
        .form(&[
            ("asking", asking),
            ("decision", "approve"),
            ("mode", "agent"),
        ])
        .send()
        .await?;
    assert_eq!(response.status(), 200);
    assert!(!response.headers().contains_key("location"));
    let page = response.text().await?;
    let address = page
        .split("readonly value=\"")
        .nth(1)
        .and_then(|text| text.split('"').next())
        .ok_or("no finishing address")?
        .replace("&amp;", "&");
    let back = reqwest::Url::parse(&address)?;
    let code = back
        .query_pairs()
        .find(|(key, _)| key == "code")
        .map(|(_, value)| value.into_owned())
        .ok_or("no code")?;
    let response = client
        .post(format!("{}/oauth/mcp/token", service.base))
        .form(&[
            ("grant_type", "authorization_code"),
            ("client_id", id),
            ("code", code.as_str()),
            ("code_verifier", VERIFIER),
            ("redirect_uri", REDIRECT),
        ])
        .send()
        .await?;
    assert_eq!(response.status(), 200);
    Ok(response.json().await?)
}

async fn read(client: &Client, service: &Service, token: &str, path: &str) -> Result<Value> {
    let response = client
        .post(format!("{}/mcp", service.base))
        .header("accept", "application/json, text/event-stream")
        .bearer_auth(token)
        .json(
            &json!({"jsonrpc":"2.0", "id":1, "method":"tools/call", "params":{
            "name":"read", "arguments":{"path":path}}}),
        )
        .send()
        .await?;
    assert_eq!(response.status(), 200, "MCP transport status");
    let answer: Value = response.json().await?;
    Ok(serde_json::from_str(
        answer["result"]["content"][0]["text"]
            .as_str()
            .ok_or("no MCP refusal body")?,
    )?)
}

#[tokio::test]
async fn a_connection_missing_a_grant_names_its_resource_action_route_and_grantor() -> Result {
    let (service, seeded) = Service::start_with(|config| {
        Ok(
            identity_contract::lys_identity_server::dev_seed::seed_configured(
                config,
                [ADMINISTRATOR, "other"],
            )?,
        )
    })
    .await?;
    let cookie = service
        .sign_in(identity_contract::apps::login(ADMINISTRATOR))
        .await?;
    let client = Client::builder().redirect(Policy::none()).build()?;
    let tokens = connection(&service, &cookie, &client).await?;
    let token = tokens["access_token"].as_str().ok_or("no access token")?;
    let target = seeded.people[0].agents[0].id.to_string();
    let path = format!("/agents/{target}");
    let answer = read(&client, &service, token, &path).await?;
    assert_eq!(answer["status"], 403, "missing authority status: {answer}");
    assert_eq!(
        answer["body"]["fields"]["resource_kind"], "agent",
        "missing grant refusal must name resource kind: {answer}"
    );
    assert_eq!(answer["body"]["fields"]["resource_id"], target);
    assert_eq!(answer["body"]["fields"]["action"], "read");
    assert_eq!(answer["body"]["fields"]["route"], path);
    assert_eq!(answer["body"]["fields"]["method"], "GET");
    let grantors = answer["body"]["can_grant"]
        .as_array()
        .ok_or("no eligible grantors")?;
    assert!(
        grantors
            .iter()
            .any(|person| person["id"] == seeded.people[0].id.to_string()),
        "the known administrator must be named"
    );
    let (status, grants) = service.get("/grants", Some(&cookie)).await?;
    assert_eq!(status, 200);
    assert_eq!(
        issued(&grants),
        Vec::<serde_json::Value>::new(),
        "refusal must not create authority"
    );
    Ok(())
}

#[tokio::test]
async fn an_undeclared_sign_in_providers_scope_names_the_route_and_service_declaration_needed()
-> Result {
    let (service, _) = Service::start_with(|config| {
        Ok(
            identity_contract::lys_identity_server::dev_seed::seed_configured(
                config,
                [ADMINISTRATOR, "other"],
            )?,
        )
    })
    .await?;
    let cookie = service
        .sign_in(identity_contract::apps::login(ADMINISTRATOR))
        .await?;
    let client = Client::builder().redirect(Policy::none()).build()?;
    let tokens = connection(&service, &cookie, &client).await?;
    let token = tokens["access_token"].as_str().ok_or("no access token")?;
    let answer = read(&client, &service, token, "/sign-in-providers").await?;
    assert_eq!(answer["status"], 403);
    assert_eq!(answer["body"]["refusal"], "TokenScopeUndeclared");
    assert_eq!(
        answer["body"]["fields"]["route"], "/sign-in-providers",
        "undeclared scope refusal must name its route: {answer}"
    );
    assert_eq!(answer["body"]["fields"]["resource_kind"], "http_route");
    assert_eq!(
        answer["body"]["fields"]["resource_id"],
        "/sign-in-providers"
    );
    assert_eq!(answer["body"]["fields"]["action"], "GET");
    assert_eq!(
        answer["body"]["fields"]["missing"],
        "declared_resource_action_scope"
    );
    assert!(
        answer["body"]["reason"]
            .as_str()
            .is_some_and(|reason| reason.contains("no grant can enable"))
    );
    assert_eq!(answer["body"]["can_grant"], json!([]));
    Ok(())
}
