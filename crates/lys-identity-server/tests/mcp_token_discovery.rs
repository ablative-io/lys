#![cfg(test)]
//! A grant token discovers only its bound authority, even when its agent holds more.

use std::error::Error;
use std::time::{SystemTime, UNIX_EPOCH};

use identity_contract::apps::{Auth, login, ok, post, seeded};
use identity_contract::harness::ADMINISTRATOR;
use serde_json::{Value, json};

#[tokio::test]
async fn a_token_discovers_its_one_grant_without_borrowing_a_broader_grant()
-> Result<(), Box<dyn Error>> {
    let (service, seeded) = seeded().await?;
    let cookie = service.sign_in(login(ADMINISTRATOR)).await?;
    let person = seeded.people[0].id.to_string();
    let agent = seeded.people[0].agents[0].id.to_string();
    let resource = json!({"kind": "person", "id": "all"});
    let operation = || lys_identity::OperationId::generate().map(|id| id.to_string());
    let root = ok(post(&service, "/grants/roots", Auth::Cookie(&cookie), &json!({
        "operation": operation()?, "route": "api", "holder": person, "resource": resource, "relation": "alpha",
        "pass_on": {"kind": "to", "actions": ["read", "write"], "recipients": ["agent"]},
        "window": {"starts_at": 0, "ends_at": null}
    })).await?)?;
    let source = root["grant"].as_str().ok_or("no root grant")?;
    let mut narrow = None;
    for relation in ["beta", "alpha"] {
        let granted = ok(post(
            &service,
            "/grants",
            Auth::Cookie(&cookie),
            &json!({
                "operation": operation()?, "route": "api", "source": source, "recipient": agent,
                "responsible": person, "resource": resource, "relation": relation,
                "pass_on": {"kind": "use_only"}, "window": {"starts_at": 0, "ends_at": null}
            }),
        )
        .await?)?;
        if relation == "beta" {
            narrow = granted["grant"].as_str().map(str::to_owned);
        }
    }
    let narrow = narrow.ok_or("no narrow grant")?;
    let expires_at = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() + 300;
    let issued = ok(post(
        &service,
        &format!("/grants/{narrow}/tokens"),
        Auth::Cookie(&cookie),
        &json!({"expires_at": expires_at}),
    )
    .await?)?;
    let token = issued["token"].as_str().ok_or("no grant token")?;
    let response = reqwest::Client::new()
        .post(format!("{}/mcp", service.base))
        .header("accept", "application/json, text/event-stream")
        .header("lys-grant-token", token)
        .json(
            &json!({"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {
                "name": "what-can-I-do", "arguments": {"method": "GET", "path": "/undeclared"}
            }}),
        )
        .send()
        .await?;
    assert_eq!(response.status(), 200);
    let answer: Value = response.json().await?;
    assert_eq!(answer["result"]["isError"], false, "{answer}");
    let inventory = &answer["result"]["structuredContent"]["body"];
    assert_eq!(
        inventory["resources"],
        json!([{"kind": "person", "id": "all", "actions": ["read"]}]),
        "{answer}"
    );
    assert!(
        inventory["routes"]
            .as_array()
            .ok_or("no routes")?
            .iter()
            .all(|route| route["action"] == "read")
    );
    Ok(())
}
