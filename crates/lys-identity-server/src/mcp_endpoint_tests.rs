//! The transport delegates through the router and retains its refusal.
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use axum::{Json, Router, routing::get};
use serde_json::{Value, json};
use std::error::Error;
use tower::ServiceExt;

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
