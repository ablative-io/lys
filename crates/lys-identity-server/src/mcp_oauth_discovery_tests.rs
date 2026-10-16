#![cfg(test)]
//! Discovery states the connected app's live reach, including an empty reach.

use super::{REDIRECT, VERIFIER, client, connected, located, pair, service};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::error::Error;

#[tokio::test]
async fn an_empty_connected_app_names_unscoped_route_rules_without_claiming_resource_grants()
-> Result<(), Box<dyn Error>> {
    let (service, cookie) = service().await?;
    let (_, tokens) = connected(&service, &cookie).await?;
    let token = tokens["access_token"].as_str().ok_or("no access token")?;
    {
        let arguments = json!({});
        let response = client()?
            .post(format!("{}/mcp", service.base))
            .header("accept", "application/json, text/event-stream")
            .bearer_auth(token)
            .json(
                &json!({"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {
                    "name": "what-can-I-do", "arguments": arguments
                }}),
            )
            .send()
            .await?;
        assert_eq!(response.status(), 200);
        let answer: Value = response.json().await?;
        let result = &answer["result"];
        assert_eq!(result["isError"], false, "{answer}");
        assert_eq!(
            crate::mcp_test_answer::body(result)["status"],
            200,
            "{answer}"
        );
        let body = &crate::mcp_test_answer::body(result)["body"];
        let routes = body["routes"].as_array().ok_or("no route inventory")?;
        for path in ["/me", "/drafts", "/dashboard"] {
            let route = routes
                .iter()
                .find(|route| route["path"] == path)
                .ok_or("an unscoped route was omitted")?;
            assert!(route["rule"]["authentication"].is_array());
            assert!(
                route.get("resource").is_none(),
                "a route rule is not a resource grant"
            );
        }
        assert_eq!(body["resources"], json!([]), "{answer}");
        assert_eq!(
            body["message"],
            "This connection has no resource grants. The listed routes apply their own authentication, role and input rules; listing them grants no authority.",
            "{answer}"
        );
    }
    Ok(())
}

#[tokio::test]
async fn consent_groups_duplicate_grants_by_resource_and_says_what_an_empty_choice_means()
-> Result<(), Box<dyn Error>> {
    let (service, cookie) = service().await?;
    let (_, me) = service.get("/me", Some(&cookie)).await?;
    let person = me["person"]["id"].as_str().ok_or("no person id")?;
    for _ in 0..2 {
        let (status, answer) = service.post("/grants/roots", Some(&cookie), &json!({
            "operation": lys_identity::OperationId::generate()?.to_string(), "route": "api",
            "holder": person, "resource": {"kind": "person", "id": "all"}, "relation": "alpha",
            "pass_on": {"kind": "to", "actions": ["read", "write"], "recipients": ["agent"]},
            "window": {"starts_at": 0, "ends_at": null}
        })).await?;
        assert_eq!(status, 200, "{answer}");
    }
    let client = client()?;
    let registered: Value = client
        .post(format!("{}/oauth/mcp/register", service.base))
        .json(&json!({"client_name": "Notes", "redirect_uris": [REDIRECT]}))
        .send()
        .await?
        .json()
        .await?;
    let client_id = registered["client_id"].as_str().ok_or("no client id")?;
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(VERIFIER.as_bytes()));
    let page = client
        .get(format!("{}/oauth/mcp/authorize", service.base))
        .query(&[
            ("response_type", "code"),
            ("client_id", client_id),
            ("redirect_uri", REDIRECT),
            ("code_challenge", challenge.as_str()),
            ("code_challenge_method", "S256"),
        ])
        .header("cookie", &cookie)
        .send()
        .await?
        .text()
        .await?;
    assert_eq!(page.matches("type=\"checkbox\"").count(), 1, "{page}");
    assert!(page.contains("read and write"), "{page}");
    assert!(page.contains("Ticking nothing connects nothing"), "{page}");
    let hidden = |name: &str| -> Result<String, Box<dyn Error>> {
        Ok(page
            .split(&format!("name=\"{name}\" value=\""))
            .nth(1)
            .and_then(|rest| rest.split('"').next())
            .ok_or("no form choice")?
            .to_owned())
    };
    let approval = client
        .post(format!("{}/oauth/mcp/consent", service.base))
        .header("cookie", &cookie)
        .form(&[
            ("asking", hidden("asking")?),
            ("decision", "approve".to_owned()),
            ("grant", hidden("grant")?),
        ])
        .send()
        .await?;
    assert_eq!(approval.status(), 200);
    let code = pair(&located(approval).await?, "code").ok_or("no code")?;
    let token: Value = client
        .post(format!("{}/oauth/mcp/token", service.base))
        .form(&[
            ("grant_type", "authorization_code"),
            ("client_id", client_id),
            ("code", &code),
            ("code_verifier", VERIFIER),
            ("redirect_uri", REDIRECT),
        ])
        .send()
        .await?
        .json()
        .await?;
    let access = token["access_token"]
        .as_str()
        .ok_or("no app access token")?;
    let response = client
        .post(format!("{}/mcp", service.base))
        .header("accept", "application/json, text/event-stream")
        .bearer_auth(access)
        .json(
            &json!({"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {
                "name": "what-can-I-do", "arguments": {}
            }}),
        )
        .send()
        .await?;
    assert_eq!(response.status(), 200);
    let answer: Value = response.json().await?;
    assert_granted_discovery(&answer)
}

fn assert_granted_discovery(answer: &Value) -> Result<(), Box<dyn Error>> {
    let discovered = &crate::mcp_test_answer::body(&answer["result"])["body"];
    assert_eq!(answer["result"]["isError"], false, "{answer}");
    assert_eq!(
        discovered["resources"],
        json!([{"kind": "person", "id": "all", "actions": ["read", "write"]}]),
        "{answer}"
    );
    assert!(
        discovered["routes"]
            .as_array()
            .ok_or("no routes")?
            .iter()
            .any(|route| route["method"] == "GET" && route["path"] == "/people"),
        "{answer}"
    );
    assert!(
        discovered["routes"]
            .as_array()
            .ok_or("no routes")?
            .iter()
            .all(|route| route["path"] != "/directory/people"
                && route["path"] != "/directory/people/{id}/sessions"),
        "{answer}"
    );
    Ok(())
}
