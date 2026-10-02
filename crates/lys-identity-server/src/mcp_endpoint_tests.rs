//! The transport delegates through the router and retains its refusal.
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use axum::{Json, Router, routing::get};
use serde_json::{Value, json};
use std::error::Error;
use tower::ServiceExt;

async fn certified_agent() -> Result<
    (
        identity_contract::harness::Service,
        String,
        String,
        std::sync::Arc<lys_core::Ed25519Identity>,
    ),
    Box<dyn Error>,
> {
    use base64::Engine;
    use identity_contract::harness::{ADMINISTRATOR, Service};
    use lys_identity::{
        Actor, AuthMethod, Directory, IdentityId, LoginBinding, OperationId, Profile, Provenance,
        Transition,
    };
    let (service, (agent, key)) = Service::start_with(|config| {
        lys_log_store::FileLeafStore::create(&config.log_dir, &config.log_origin)?;
        let path = config.log_dir.clone();
        let key = lys_core::Ed25519Identity::load(&config.event_key_file)?;
        lys_identity::directory_migration::migrate(
            lys_log_store::FileLeafStore::open(&path)?,
            &key,
        )?;
        let mut directory = Directory::open(
            Box::new(move || lys_log_store::FileLeafStore::open(&path)),
            key,
        )?;
        let actor = Actor::new(
            LoginBinding::new(&config.issuer, ADMINISTRATOR)?,
            Provenance::new(AuthMethod::Oidc, 1),
        );
        let (person, _) = directory.setup_person(
            actor.clone(),
            OperationId::generate()?,
            Profile::new("Owner")?,
            1,
        )?;
        let (agent, _) = directory.register_agent(
            actor.clone(),
            OperationId::generate()?,
            person,
            Profile::new("Caller")?,
            2,
        )?;
        directory.transition(
            actor,
            OperationId::generate()?,
            IdentityId::Agent(agent),
            Transition::Activate,
            "",
            3,
        )?;
        Ok((
            agent.to_string(),
            std::sync::Arc::new(lys_core::Ed25519Identity::load(&config.event_key_file)?),
        ))
    })
    .await?;
    let cookie = service
        .sign_in(identity_contract::apps::login(ADMINISTRATOR))
        .await?;
    let request = lys_core::ca::create_certificate_request(&key, &agent)?;
    let (status, answer) = service
        .post(
            &format!("/agents/{agent}/certificates"),
            Some(&cookie),
            &json!({
                "operation":lys_identity::OperationId::generate()?.to_string(),
                "request":base64::engine::general_purpose::STANDARD.encode(request)
            }),
        )
        .await?;
    assert_eq!(status, 200, "{answer}");
    Ok((service, cookie, agent, key))
}

fn change_call(id: u64, path: &str, body: &Value) -> Result<Vec<u8>, Box<dyn Error>> {
    Ok(serde_json::to_vec(&json!({
        "jsonrpc":"2.0", "id":id, "method":"tools/call", "params":{
            "name":"change", "arguments":{"method":"POST","path":path,"body":body}
        }
    }))?)
}

async fn leaves(
    service: &identity_contract::harness::Service,
    cookie: &str,
) -> Result<u64, Box<dyn Error>> {
    let (status, page) = service.get("/receipts/0", Some(cookie)).await?;
    assert_eq!(status, 200, "{page}");
    Ok(page["checkpoint"]["tree_size"]
        .as_u64()
        .ok_or("the receipt checkpoint has no tree size")?)
}

fn nobody() -> Result<Value, Box<dyn Error>> {
    Ok(json!({
        "operation": lys_identity::OperationId::generate()?.to_string(),
        "display_name": "Nobody"
    }))
}

/// A change the route refuses is kept once as an ask, never answered as a
/// receipt; the same refused ask made again, under a fresh operation id,
/// writes no second leaf; and once the directory has moved it is kept afresh,
/// as a later leaf.
#[tokio::test]
async fn a_refused_ask_is_kept_once_and_answered_as_asked_not_as_a_receipt()
-> Result<(), Box<dyn Error>> {
    let (service, cookie, agent, key) = certified_agent().await?;
    let before = leaves(&service, &cookie).await?;
    let mut first = None;
    for id in [11_u64, 12] {
        let bytes = change_call(id, "/people", &nobody()?)?;
        let signature = signed(&agent, &key, &bytes)?;
        let response = post_signed(&service.base, bytes, &signature).await?;
        assert_eq!(response.status(), StatusCode::OK);
        let answer: Value = response.json().await?;
        assert_eq!(answer["result"]["isError"], true, "{answer}");
        assert_eq!(
            answer["result"]["structuredContent"]["status"], 401,
            "an agent is no signed-in person, so the route refuses after the ask is kept: {answer}"
        );
        assert_eq!(
            answer["result"]["structuredContent"]["body"]["refusal"],
            "NotSignedIn"
        );
        assert!(
            answer["result"].get("receipt").is_none(),
            "a refused change is never answered as a receipt: {answer}"
        );
        let asked = answer["result"]["asked"].clone();
        assert!(asked["operation"].is_string(), "the ask is named: {answer}");
        match &first {
            None => first = Some(asked),
            Some(first) => assert_eq!(&asked, first, "the same ask, the same leaf"),
        }
        assert_eq!(
            leaves(&service, &cookie).await?,
            before + 1,
            "one leaf for the ask, whatever the number of attempts or operation ids"
        );
    }
    let first = first.ok_or("the first ask")?;
    let other = json!({
        "operation": lys_identity::OperationId::generate()?.to_string(),
        "display_name": "Somebody"
    });
    let bytes = change_call(13, "/people", &other)?;
    let signature = signed(&agent, &key, &bytes)?;
    let answer: Value = post_signed(&service.base, bytes, &signature)
        .await?
        .json()
        .await?;
    assert_eq!(answer["result"]["isError"], true, "{answer}");
    assert_ne!(
        answer["result"]["asked"], first,
        "a different ask is its own leaf"
    );
    assert_eq!(leaves(&service, &cookie).await?, before + 2);

    let (status, made) = service
        .post(
            "/people",
            Some(&cookie),
            &json!({
                "operation": lys_identity::OperationId::generate()?.to_string(),
                "display_name": "Somebody Real"
            }),
        )
        .await?;
    assert_eq!(status, 200, "the administrator moves the directory: {made}");
    let moved = leaves(&service, &cookie).await?;
    assert!(moved > before + 2, "the directory log moved");
    let bytes = change_call(14, "/people", &nobody()?)?;
    let signature = signed(&agent, &key, &bytes)?;
    let answer: Value = post_signed(&service.base, bytes, &signature)
        .await?
        .json()
        .await?;
    assert_eq!(answer["result"]["isError"], true, "{answer}");
    let again = answer["result"]["asked"].clone();
    assert_ne!(
        again, first,
        "the judgment it was refused under may have changed, so the ask is kept afresh"
    );
    let index = |asked: &Value| asked["log"]["index"].as_u64().ok_or("the leaf's index");
    assert!(
        index(&again)? > index(&first)?,
        "the fresh leaf is later than the one asked before: {again} after {first}"
    );
    assert_eq!(leaves(&service, &cookie).await?, moved + 1);
    Ok(())
}

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

fn signed(
    agent: &str,
    key: &lys_core::Ed25519Identity,
    bytes: &[u8],
) -> Result<String, Box<dyn Error>> {
    use sha2::{Digest, Sha256};
    use std::time::{SystemTime, UNIX_EPOCH};
    let at = u64::try_from(SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis())?;
    let nonce = crate::routes::hex(&Sha256::digest(
        lys_identity::OperationId::generate()?
            .to_string()
            .as_bytes(),
    ));
    let payload = crate::agent_signature::payload("POST", "/mcp", bytes, at, &nonce);
    let signature =
        crate::routes::hex(&lys_core::attestation::sign_attestation(&payload, key).to_cose_bytes());
    Ok(format!("{agent} {at} {nonce} {signature}"))
}

async fn post_signed(
    base: &str,
    bytes: Vec<u8>,
    signature: &str,
) -> Result<reqwest::Response, Box<dyn Error>> {
    Ok(reqwest::Client::new()
        .post(format!("{base}/mcp"))
        .header("content-type", "application/json")
        .header("accept", "application/json, text/event-stream")
        .header("mcp-protocol-version", "2025-11-25")
        .header(crate::agent_signature::HEADER, signature)
        .body(bytes)
        .send()
        .await?)
}

fn tree_call(id: u64) -> Result<Vec<u8>, Box<dyn Error>> {
    Ok(serde_json::to_vec(&json!({
        "jsonrpc":"2.0", "id":id, "method":"tools/call", "params":{
            "name":"read", "arguments":{"method":"GET","path":"/tree"}
        }
    }))?)
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
        called["result"]["structuredContent"]["status"], 200,
        "{called}"
    );
    assert_eq!(
        called["result"]["structuredContent"]["body"]["root"]["id"], agent,
        "{called}"
    );
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
        called["result"]["structuredContent"]["status"], 200,
        "{called}"
    );
    Ok(())
}

fn register(router: Router, origin: &str) -> Result<Router, crate::error::ServerError> {
    Ok(router.clone().merge(super::routes(router, origin)?))
}

fn request(message: &Value) -> Result<Request<Body>, Box<dyn Error>> {
    Ok(Request::builder()
        .method("POST")
        .uri("/mcp")
        .header("content-type", "application/json")
        .header("accept", "application/json, text/event-stream")
        .header("mcp-protocol-version", "2025-11-25")
        .body(Body::from(serde_json::to_vec(message)?))?)
}

async fn answer(router: Router, message: &Value) -> Result<(StatusCode, Value), Box<dyn Error>> {
    let request = request(message)?;
    let response = router.oneshot(request).await?;
    let status = response.status();
    let body = to_bytes(response.into_body(), 2 * 1024 * 1024).await?;
    Ok((status, serde_json::from_slice(&body)?))
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
    assert_eq!(tools.len(), crate::openapi_table::TABLE.len());
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
        result["result"]["structuredContent"],
        json!({"status":status.as_u16(), "body":body})
    );
    Ok(())
}

#[tokio::test]
async fn a_call_the_http_route_refuses_is_refused_the_same_over_mcp() -> Result<(), Box<dyn Error>>
{
    use identity_contract::harness::{ADMINISTRATOR, Service};
    use lys_identity::{
        Actor, AuthMethod, IdentityId, LoginBinding, OperationId, Profile, Provenance, Transition,
    };
    let (service, ()) = Service::start_with(|config| {
        let administrator = Actor::new(
            LoginBinding::new(&config.issuer, ADMINISTRATOR)?,
            Provenance::new(AuthMethod::Oidc, 1),
        );
        lys_log_store::FileLeafStore::create(&config.log_dir, &config.log_origin)?;
        let path = config.log_dir.clone();
        let key = lys_identity::signer::load_service_key(&config.event_key_file)?;
        lys_identity::directory_migration::migrate(
            lys_log_store::FileLeafStore::open(&path)?,
            &key,
        )?;
        let mut directory = lys_identity::Directory::open(
            Box::new(move || lys_log_store::FileLeafStore::open(&path)),
            key,
        )?;
        directory.setup_person(
            administrator.clone(),
            OperationId::generate()?,
            Profile::new("Owner")?,
            1,
        )?;
        let (person, _) = directory.register_person(
            administrator.clone(),
            OperationId::generate()?,
            Profile::new("Member")?,
            2,
        )?;
        directory.bind_login(
            administrator.clone(),
            OperationId::generate()?,
            person,
            LoginBinding::new(&config.issuer, "member")?,
            3,
        )?;
        directory.transition(
            administrator,
            OperationId::generate()?,
            IdentityId::Person(person),
            Transition::Activate,
            "",
            4,
        )?;
        Ok(())
    })
    .await?;
    let cookie = service
        .sign_in(identity_contract::apps::login("member"))
        .await?;
    let (direct_status, direct_body) = service.get("/directory/people", Some(&cookie)).await?;
    assert_eq!(direct_status, 403, "{direct_body}");
    let client = reqwest::Client::new();
    let message = json!({"jsonrpc":"2.0", "id":4, "method":"tools/call", "params":{
        "name":"read", "arguments":{"method":"GET", "path":"/directory/people"}
    }});
    let response = client
        .post(format!("{}/mcp", service.base))
        .header("accept", "application/json, text/event-stream")
        .header("mcp-protocol-version", "2025-11-25")
        .header("cookie", &cookie)
        .json(&message)
        .send()
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let called: Value = response.json().await?;
    assert_eq!(called["result"]["isError"], true, "{called}");
    assert_eq!(
        called["result"]["structuredContent"],
        json!({"status":direct_status, "body":direct_body})
    );
    let (direct_status, direct_body) = service.get("/sessions", Some(&cookie)).await?;
    let response: Value = client
        .post(format!("{}/mcp", service.base))
        .header("accept", "application/json, text/event-stream")
        .header("mcp-protocol-version", "2025-11-25")
        .header("cookie", &cookie)
        .json(
            &json!({"jsonrpc":"2.0", "id":5, "method":"tools/call", "params":{
                "name":"read", "arguments":{"method":"GET", "path":"/sessions"}
            }}),
        )
        .send()
        .await?
        .json()
        .await?;
    assert_eq!(
        response["result"]["structuredContent"],
        json!({"status":direct_status, "body":direct_body})
    );
    let response = client
        .post(format!("{}/mcp", service.base))
        .header("accept", "application/json, text/event-stream")
        .json(&message)
        .send()
        .await?;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let response: Value = response.json().await?;
    assert_eq!(response["refusal"], "NotSignedIn");
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
    assert_eq!(response["result"]["structuredContent"]["status"], 404);
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
