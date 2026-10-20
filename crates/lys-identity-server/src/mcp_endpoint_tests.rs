//! The transport delegates through the router and retains its refusal.
use super::test_support::{
    answer, certified_agent, change_call, nobody, post_signed, register, request, signed, tree_call,
};
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use axum::{Json, Router, routing::get};
use serde_json::{Value, json};
use std::error::Error;
use tower::ServiceExt;

#[tokio::test]
async fn a_signed_mcp_call_cannot_borrow_an_administrator_cookie() -> Result<(), Box<dyn Error>> {
    use sha2::{Digest, Sha256};
    use std::time::{SystemTime, UNIX_EPOCH};
    let (service, cookie, agent, key) = certified_agent().await?;
    let bytes = serde_json::to_vec(&json!({
        "jsonrpc":"2.0", "id":9, "method":"tools/call", "params":{
            "name":"get_directory_people", "arguments":{"method":"GET","path":"/directory/people"}
        }
    }))?;
    let at = u64::try_from(SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis())?;
    let nonce = crate::routes::hex(&Sha256::digest(
        lys_identity::OperationId::generate()?
            .to_string()
            .as_bytes(),
    ));
    let payload = crate::agent_signature::payload("POST", "/mcp", &bytes, at, &nonce);
    let signature = crate::routes::hex(
        &lys_core::attestation::sign_attestation(&payload, &key).to_cose_bytes(),
    );
    let response = reqwest::Client::new()
        .post(format!("{}/mcp", service.base))
        .header("content-type", "application/json")
        .header("accept", "application/json, text/event-stream")
        .header("cookie", cookie)
        .header(
            crate::agent_signature::HEADER,
            format!("{agent} {at} {nonce} {signature}"),
        )
        .body(bytes)
        .send()
        .await?;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let refused: Value = response.json().await?;
    assert_eq!(refused["refusal"], "AgentSignatureRefused", "{refused}");
    assert!(
        refused["reason"]
            .as_str()
            .ok_or("no refusal reason")?
            .contains("cookie")
    );
    Ok(())
}

#[tokio::test]
async fn a_signed_mcp_call_reaches_the_route_as_the_agent() -> Result<(), Box<dyn Error>> {
    let (service, _cookie, agent, key) = certified_agent().await?;
    let bytes = tree_call(10)?;
    let signature = signed(&agent, &key, &bytes)?;
    let response = post_signed(&service.base, bytes, &signature).await?;
    assert_eq!(response.status(), StatusCode::OK);
    let called: Value = response.json().await?;
    assert_eq!(
        crate::mcp_test_answer::body(&called["result"])["status"],
        200,
        "{called}"
    );
    assert_eq!(
        crate::mcp_test_answer::body(&called["result"])["body"]["root"]["id"],
        agent,
        "{called}"
    );
    Ok(())
}

#[tokio::test]
async fn a_call_carrying_the_clients_meta_is_answered_and_any_other_stray_field_refused()
-> Result<(), Box<dyn Error>> {
    let (service, _cookie, agent, key) = certified_agent().await?;
    let call = |id: u64, extra: &str| {
        serde_json::to_vec(&json!({
            "jsonrpc":"2.0", "id":id, "method":"tools/call", "params":{
                "name":"read", "arguments":{"method":"GET","path":"/tree"},
                extra: {"progressToken": 1}
            }
        }))
    };
    let bytes = call(20, "_meta")?;
    let signature = signed(&agent, &key, &bytes)?;
    let called: Value = post_signed(&service.base, bytes, &signature)
        .await?
        .json()
        .await?;
    assert_eq!(
        crate::mcp_test_answer::body(&called["result"])["status"],
        200,
        "{called}"
    );
    let bytes = call(21, "meta")?;
    let signature = signed(&agent, &key, &bytes)?;
    let refused: Value = post_signed(&service.base, bytes, &signature)
        .await?
        .json()
        .await?;
    assert_eq!(refused["error"]["code"], -32602, "{refused}");
    Ok(())
}

#[tokio::test]
async fn a_replayed_signed_mcp_message_is_refused() -> Result<(), Box<dyn Error>> {
    let (service, _cookie, agent, key) = certified_agent().await?;
    let bytes = tree_call(11)?;
    let signature = signed(&agent, &key, &bytes)?;
    let first = post_signed(&service.base, bytes.clone(), &signature).await?;
    assert_eq!(first.status(), StatusCode::OK);
    let replayed = post_signed(&service.base, bytes, &signature).await?;
    assert_eq!(replayed.status(), StatusCode::UNAUTHORIZED);
    let refused: Value = replayed.json().await?;
    assert_eq!(refused["refusal"], "AgentSignatureRefused", "{refused}");
    assert!(
        refused["reason"]
            .as_str()
            .ok_or("no refusal reason")?
            .contains("nonce")
    );
    Ok(())
}

#[tokio::test]
async fn a_signed_mcp_message_altered_after_signing_is_refused() -> Result<(), Box<dyn Error>> {
    let (service, _cookie, agent, key) = certified_agent().await?;
    let signature = signed(&agent, &key, &tree_call(12)?)?;
    let altered = serde_json::to_vec(&json!({
        "jsonrpc":"2.0", "id":12, "method":"tools/call", "params":{
            "name":"read", "arguments":{"method":"GET","path":"/directory/people"}
        }
    }))?;
    let response = post_signed(&service.base, altered, &signature).await?;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let refused: Value = response.json().await?;
    assert_eq!(refused["refusal"], "AgentSignatureRefused", "{refused}");
    assert!(
        refused["reason"]
            .as_str()
            .ok_or("no refusal reason")?
            .contains("does not verify")
    );
    Ok(())
}

#[tokio::test]
async fn a_relayed_agent_gains_nothing_its_grants_do_not_give() -> Result<(), Box<dyn Error>> {
    let (service, cookie, agent, key) = certified_agent().await?;
    let (direct_status, _) = service.get("/directory/people", Some(&cookie)).await?;
    assert_eq!(direct_status, 200);
    let bytes = serde_json::to_vec(&json!({
        "jsonrpc":"2.0", "id":13, "method":"tools/call", "params":{
            "name":"read", "arguments":{"method":"GET","path":"/directory/people"}
        }
    }))?;
    let signature = signed(&agent, &key, &bytes)?;
    let response = post_signed(&service.base, bytes, &signature).await?;
    assert_eq!(response.status(), StatusCode::OK);
    let called: Value = response.json().await?;
    assert_eq!(called["result"]["isError"], true, "{called}");
    assert_ne!(
        crate::mcp_test_answer::body(&called["result"])["status"],
        200,
        "{called}"
    );
    Ok(())
}

#[tokio::test]
async fn mcp_initialize_and_tools_list_answer() -> Result<(), Box<dyn Error>> {
    let router = register(Router::new(), "http://localhost")?;
    let (status, initialized) = answer(
        router.clone(),
        &json!({
            "jsonrpc":"2.0", "id":1, "method":"initialize", "params":{
                "protocolVersion":"2025-11-25", "capabilities":{},
                "clientInfo":{"name":"test", "version":"1"}
            }
        }),
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(initialized["result"]["protocolVersion"], "2025-11-25");
    assert_eq!(initialized["result"]["capabilities"]["tools"], json!({}));
    let (_, listed) = answer(
        router.clone(),
        &json!({"jsonrpc":"2.0", "id":2, "method":"tools/list"}),
    )
    .await?;
    let tools = listed["result"]["tools"].as_array().ok_or("no tools")?;
    let names: Vec<&str> = tools
        .iter()
        .filter_map(|tool| tool["name"].as_str())
        .collect();
    assert_eq!(
        names,
        ["what-can-I-do", "read", "change", "drafts"],
        "Lys advertises four tools, not one per route: {listed}"
    );
    for tool in tools {
        assert!(
            !tool["description"]
                .as_str()
                .ok_or("no description")?
                .is_empty()
        );
        assert_eq!(tool["inputSchema"]["type"], "object");
    }
    let response = router
        .clone()
        .oneshot(request(
            &json!({"jsonrpc":"2.0", "method":"notifications/initialized"}),
        )?)
        .await?;
    assert_eq!(response.status(), StatusCode::ACCEPTED);
    assert!(to_bytes(response.into_body(), 1024).await?.is_empty());
    assert_eq!(
        router
            .oneshot(Request::builder().uri("/mcp").body(Body::empty())?)
            .await?
            .status(),
        StatusCode::METHOD_NOT_ALLOWED
    );
    Ok(())
}

#[tokio::test]
async fn a_router_refusal_keeps_its_status_and_body_over_mcp() -> Result<(), Box<dyn Error>> {
    let api = Router::new().route(
        "/protected",
        get(|| async {
            (
                StatusCode::FORBIDDEN,
                Json(json!({"refusal":"NotAdmitted","reason":"this caller may not act"})),
            )
        }),
    );
    let direct = api
        .clone()
        .oneshot(Request::builder().uri("/protected").body(Body::empty())?)
        .await?;
    let status = direct.status();
    let body: Value = serde_json::from_slice(&to_bytes(direct.into_body(), 1024).await?)?;
    let (_, result) = answer(
        register(api, "http://localhost")?,
        &json!({
            "jsonrpc":"2.0", "id":3, "method":"tools/call", "params":{
                "name":"read", "arguments":{"method":"GET","path":"/protected"}
            }
        }),
    )
    .await?;
    assert_eq!(result["result"]["isError"], true);
    assert_eq!(
        crate::mcp_test_answer::body(&result["result"]),
        json!({"status":status.as_u16(), "body":body})
    );
    Ok(())
}

#[tokio::test]
async fn mcp_transport_rejects_foreign_origins_versions_and_external_routes()
-> Result<(), Box<dyn Error>> {
    let router = register(Router::new(), "http://localhost")?;
    let ping = json!({"jsonrpc":"2.0", "id":6,"method":"ping"});
    for (header, value, expected) in [
        ("origin", "https://foreign.invalid", StatusCode::FORBIDDEN),
        ("mcp-protocol-version", "invalid", StatusCode::BAD_REQUEST),
    ] {
        let mut request = request(&ping)?;
        request.headers_mut().insert(header, value.parse()?);
        assert_eq!(router.clone().oneshot(request).await?.status(), expected);
    }
    for path in ["https://foreign.invalid/me", "//foreign.invalid/me"] {
        let (_, response) = answer(router.clone(), &json!({"jsonrpc":"2.0", "id":7,"method":"tools/call","params":{"name":"read","arguments":{"method":"GET","path":path}}})).await?;
        assert_eq!(response["error"]["code"], -32602);
    }
    let (_, response) = answer(router, &json!({"jsonrpc":"2.0", "id":8,"method":"tools/call","params":{"name":"read","arguments":{"method":"GET","path":"/mcp"}}})).await?;
    assert_eq!(
        crate::mcp_test_answer::body(&response["result"])["status"],
        404
    );
    Ok(())
}

#[tokio::test]
async fn a_signed_message_cannot_also_carry_a_grant_token() -> Result<(), Box<dyn Error>> {
    let (service, _cookie, agent, key) = certified_agent().await?;
    let bytes = tree_call(14)?;
    let signature = signed(&agent, &key, &bytes)?;
    let response = reqwest::Client::new()
        .post(format!("{}/mcp", service.base))
        .header("content-type", "application/json")
        .header("accept", "application/json, text/event-stream")
        .header(crate::agent_signature::HEADER, signature)
        .header(crate::grant_tokens::HEADER, "another-agents-token")
        .body(bytes)
        .send()
        .await?;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let refused: Value = response.json().await?;
    assert_eq!(refused["refusal"], "AgentSignatureRefused", "{refused}");
    assert!(
        refused["reason"]
            .as_str()
            .ok_or("no refusal reason")?
            .contains("a grant token cannot carry another credential")
    );
    Ok(())
}

/// An agent's MCP call to a route whose door knew only a person's session answered
/// `NotSignedIn` (12 Oct 2026). Relayed, it is judged through the agent's own grants: with
/// no grant for the route's action it is refused by name, and with one it reaches the
/// route, which refuses it too: admitting a person is organisation.admit on the caller's
/// own organisation (ACCESS-009 R1), an act withheld from agents, so no agent registers
/// a person by any route.
#[tokio::test]
async fn an_agents_mcp_call_is_judged_by_its_grants_and_admitting_a_person_is_withheld_from_it()
-> Result<(), Box<dyn Error>> {
    let (service, cookie, agent, key) = certified_agent().await?;
    let register = |id: u64| -> Result<(Vec<u8>, String), Box<dyn Error>> {
        let bytes = change_call(id, "/people", &nobody()?)?;
        let signature = signed(&agent, &key, &bytes)?;
        Ok((bytes, signature))
    };

    let (bytes, signature) = register(20)?;
    let refused: Value = post_signed(&service.base, bytes, &signature)
        .await?
        .json()
        .await?;
    let answer = &crate::mcp_test_answer::body(&refused["result"]);
    assert_eq!(refused["result"]["isError"], true, "{refused}");
    let name = answer["body"]["refusal"]
        .as_str()
        .ok_or("no refusal name")?;
    assert_ne!(name, "NotSignedIn", "{refused}");

    let (status, tree) = service.get("/tree", Some(&cookie)).await?;
    assert_eq!(status, 200, "{tree}");
    let person = tree["root"]["id"].as_str().ok_or("no person")?.to_owned();
    let resource = json!({"kind": "person", "id": "all"});
    let (status, root) = service
        .post(
            "/grants/roots",
            Some(&cookie),
            &json!({
                "operation": lys_identity::OperationId::generate()?.to_string(),
                "route": "api", "holder": person, "resource": resource,
                "relation": "only.person.create",
                "pass_on": {"kind": "to", "actions": ["person.create"], "recipients": ["agent"]},
                "window": {"starts_at": 0, "ends_at": null},
            }),
        )
        .await?;
    assert_eq!(status, 200, "{root}");
    let (status, granted) = service
        .post(
            "/grants",
            Some(&cookie),
            &json!({
                "operation": lys_identity::OperationId::generate()?.to_string(),
                "route": "api", "source": root["grant"], "recipient": agent,
                "responsible": person, "resource": resource,
                "relation": "only.person.create", "pass_on": {"kind": "use_only"},
                "window": {"starts_at": 0, "ends_at": null},
            }),
        )
        .await?;
    assert_eq!(status, 200, "{granted}");

    let (bytes, signature) = register(21)?;
    let refused: Value = post_signed(&service.base, bytes, &signature)
        .await?
        .json()
        .await?;
    let answer = &crate::mcp_test_answer::body(&refused["result"]);
    assert_eq!(answer["status"], 403, "{refused}");
    assert_eq!(answer["body"]["refusal"], "NotAdmitted", "{refused}");
    assert!(
        answer["body"]["reason"]
            .as_str()
            .is_some_and(|reason| reason.contains("organisation.admit")),
        "{refused}"
    );
    Ok(())
}
