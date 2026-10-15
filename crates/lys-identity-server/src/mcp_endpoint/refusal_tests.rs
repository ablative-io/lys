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
    let answer = refused("TokenScopeUndeclared", "/security-events", json!({})).await?;
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
            "name":"read", "arguments":{"path":"/security-events"}
        }
    }))?;
    request.headers_mut().insert(
        crate::grant_tokens::HEADER,
        axum::http::HeaderValue::from_static("not-an-issued-credential"),
    );
    let response = router.oneshot(request).await?;
    assert_eq!(response.status(), StatusCode::OK);
    let response: Value = serde_json::from_slice(&to_bytes(response.into_body(), 65536).await?)?;
    let answer = crate::mcp_test_answer::body(&response["result"]);
    assert_eq!(answer["status"], 403);
    assert_eq!(answer["body"]["refusal"], "TokenScopeUndeclared");
    assert_eq!(answer["body"]["fields"]["route"], "/security-events");
    assert_eq!(answer["body"]["fields"]["resource_kind"], "http_route");
    assert_eq!(answer["body"]["fields"]["action"], "GET");
    Ok(())
}
