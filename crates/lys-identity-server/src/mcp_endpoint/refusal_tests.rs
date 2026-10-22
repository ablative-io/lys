//! Scope details supplement grant refusals without admitting the refused call.

use axum::{Json, Router, http::StatusCode, routing::get};
use serde_json::{Value, json};
use std::error::Error;

use super::test_support::{answer, register};

type Result<T = ()> = std::result::Result<T, Box<dyn Error>>;

async fn refused(name: &'static str, path: &str, fields: Value) -> Result<Value> {
    let api = Router::new().route(
        path,
        get(move || {
            let fields = fields.clone();
            async move {
                (
                    StatusCode::FORBIDDEN,
                    Json(json!({
                        "refusal":name, "reason":"the required grant is absent", "fields":fields
                    })),
                )
            }
        }),
    );
    let (_, response) = answer(
        register(api, "http://localhost")?,
        &json!({
            "jsonrpc":"2.0", "id":1, "method":"tools/call", "params":{
                "name":"read", "arguments":{"path":path}
            }
        }),
    )
    .await?;
    Ok(crate::mcp_test_answer::body(&response["result"]))
}

#[tokio::test]
async fn sibling_grant_refusals_name_the_same_declared_resource_and_action() -> Result {
    for name in [
        "TokenScopeUndeclared",
        "GrantTokenScopeMismatch",
        "NotAdmitted",
        "not_permitted",
        "AgentNotVisible",
        "NotHeld",
    ] {
        let answer = refused(name, "/agents/requested", json!({"original":"retained"})).await?;
        assert_eq!(answer["status"], 403);
        assert_eq!(answer["body"]["refusal"], name);
        assert_eq!(answer["body"]["fields"]["resource_kind"], "agent");
        assert_eq!(answer["body"]["fields"]["resource_id"], "requested");
        assert_eq!(answer["body"]["fields"]["action"], "read");
        assert_eq!(answer["body"]["fields"]["route"], "/agents/requested");
        assert_eq!(answer["body"]["fields"]["original"], "retained");
    }
    Ok(())
}

#[tokio::test]
async fn a_refusal_on_a_route_without_a_scope_never_invents_a_grant() -> Result {
    let answer = refused("TokenScopeUndeclared", "/sign-in-providers", json!({})).await?;
    assert_eq!(
        answer["body"]["fields"]["missing"],
        "declared_resource_action_scope"
    );
    assert_eq!(answer["body"]["fields"]["resource_kind"], "http_route");
    assert_eq!(answer["body"]["fields"]["action"], "GET");
    assert_eq!(answer["body"]["can_grant"], json!([]));
    Ok(())
}

#[tokio::test]
async fn operational_refusals_do_not_ask_for_grants() -> Result {
    let answer = refused("DirectoryUnavailable", "/agents/requested", json!({})).await?;
    assert_eq!(answer["body"]["fields"], json!({}));
    assert!(answer["body"].get("can_grant").is_none());
    Ok(())
}

#[tokio::test]
async fn a_scope_refused_before_relay_still_names_the_requested_route() -> Result {
    use axum::body::to_bytes;
    use tower::ServiceExt;
    let router = register(Router::new(), "http://localhost")?;
    let mut request = super::test_support::request(&json!({
        "jsonrpc":"2.0", "id":1, "method":"tools/call", "params":{
            "name":"read", "arguments":{"path":"/sign-in-providers"}
        }
    }))?;
    request.headers_mut().insert(
        crate::grant_tokens::HEADER,
        axum::http::HeaderValue::from_static("not-an-issued-credential"),
    );
    let response = router.oneshot(request).await?;
    assert_eq!(response.status(), StatusCode::OK);
    let response: Value = serde_json::from_slice(&to_bytes(response.into_body(), 65_536).await?)?;
    let answer = crate::mcp_test_answer::body(&response["result"]);
    assert_eq!(answer["status"], 403);
    assert_eq!(answer["body"]["refusal"], "TokenScopeUndeclared");
    assert_eq!(answer["body"]["fields"]["route"], "/sign-in-providers");
    assert_eq!(answer["body"]["fields"]["resource_kind"], "http_route");
    assert_eq!(answer["body"]["fields"]["action"], "GET");
    Ok(())
}

fn same_admission(named: &Value, status: u16, direct: &Value) -> Result {
    assert_eq!(named["status"], status);
    if direct.get("refusal").is_none() {
        assert_eq!(&named["body"], direct);
        return Ok(());
    }
    let original = direct["reason"]
        .as_str()
        .ok_or("HTTP refusal has no reason")?;
    assert!(
        named["body"]["reason"]
            .as_str()
            .is_some_and(|reason| reason.starts_with(original))
    );
    for (key, value) in direct.as_object().ok_or("HTTP refusal is not an object")? {
        if key != "reason" && key != "fields" {
            assert_eq!(&named["body"][key], value, "HTTP refusal field {key}");
        }
    }
    Ok(())
}

async fn member_fixture() -> Result<(identity_contract::harness::Service, String)> {
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
        let (owner, _) = directory.setup_person(
            administrator.clone(),
            OperationId::generate()?,
            Profile::new("Owner")?,
            Profile::new("Example organisation")?,
            1,
        )?;
        let organisation = identity_contract::fixtures::organisation_of(&mut directory, owner)?;
        let (person, _) = directory.register_person(
            administrator.clone(),
            OperationId::generate()?,
            Profile::new("Member")?,
            organisation,
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
    Ok((service, cookie))
}

#[tokio::test]
async fn scope_details_keep_the_http_refusal_admission() -> Result {
    let (service, cookie) = member_fixture().await?;
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
    let named = crate::mcp_test_answer::body(&called["result"]);
    same_admission(&named, direct_status, &direct_body)?;
    assert_eq!(named["body"]["fields"]["resource_kind"], "person");
    assert_eq!(named["body"]["fields"]["resource_id"], "all");
    assert_eq!(named["body"]["fields"]["action"], "read");
    assert_eq!(named["body"]["fields"]["route"], "/directory/people");
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
    same_admission(
        &crate::mcp_test_answer::body(&response["result"]),
        direct_status,
        &direct_body,
    )?;
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
