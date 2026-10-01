//! The advertised tools retain the route table and its schema gaps.
use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::error::Error;
use tower::ServiceExt;

use crate::openapi_table::{E, TABLE};

fn endpoint() -> Result<Router, crate::error::ServerError> {
    crate::mcp_endpoint::routes(
        Router::new().fallback(|| async { StatusCode::IM_A_TEAPOT }),
        "http://localhost",
    )
}

async fn answer(router: Router, method: &str, params: Value) -> Result<Value, Box<dyn Error>> {
    let request = Request::builder()
        .method("POST")
        .uri("/mcp")
        .header("content-type", "application/json")
        .header("accept", "application/json, text/event-stream")
        .body(Body::from(serde_json::to_vec(&json!({
            "jsonrpc":"2.0", "id":1, "method":method, "params":params
        }))?))?;
    let response = router.oneshot(request).await?;
    assert_eq!(response.status(), StatusCode::OK);
    Ok(serde_json::from_slice(
        &to_bytes(response.into_body(), 2 * 1024 * 1024).await?,
    )?)
}

#[tokio::test]
async fn tools_list_covers_every_table_route_once() -> Result<(), Box<dyn Error>> {
    let listed = answer(endpoint()?, "tools/list", Value::Null).await?;
    let tools = listed["result"]["tools"].as_array().ok_or("no tools")?;
    assert_eq!(tools.len(), TABLE.len());
    let names: BTreeSet<_> = tools.iter().map(|tool| tool["name"].as_str()).collect();
    assert_eq!(names.len(), TABLE.len());
    let routes: BTreeSet<_> = tools
        .iter()
        .map(|tool| {
            (
                tool["_meta"]["lys/method"].as_str(),
                tool["_meta"]["lys/path"].as_str(),
            )
        })
        .collect();
    let expected: BTreeSet<_> = TABLE
        .iter()
        .map(|E(method, path, ..)| (Some(method.word()), Some(*path)))
        .collect();
    assert_eq!(routes, expected);
    Ok(())
}

#[tokio::test]
async fn a_route_without_a_schema_is_named_not_guessed() -> Result<(), Box<dyn Error>> {
    let listed = answer(endpoint()?, "tools/list", Value::Null).await?;
    let tools = listed["result"]["tools"].as_array().ok_or("no tools")?;
    for path in [
        "/authority",
        "/configuration",
        "/secrets/scope",
        "/agents/{id}/start",
    ] {
        let tool = tools
            .iter()
            .find(|tool| tool["_meta"]["lys/path"] == path)
            .ok_or_else(|| format!("no tool for {path}"))?;
        assert!(
            tool["description"]
                .as_str()
                .ok_or("no description")?
                .contains("no schema")
        );
        assert_eq!(tool["_meta"]["lys/responseSchema"], "no schema");
        assert!(tool.get("outputSchema").is_none());
        assert_eq!(
            tool["inputSchema"]["properties"]["body"],
            json!({"description":"no schema"})
        );
    }
    Ok(())
}

#[tokio::test]
async fn every_advertised_tool_name_is_accepted_and_an_unknown_name_is_refused()
-> Result<(), Box<dyn Error>> {
    let router = endpoint()?;
    let listed = answer(router.clone(), "tools/list", Value::Null).await?;
    let tools = listed["result"]["tools"].as_array().ok_or("no tools")?;
    for tool in tools {
        let method = tool["_meta"]["lys/method"]
            .as_str()
            .ok_or("no method")?
            .to_ascii_uppercase();
        let template = tool["_meta"]["lys/path"].as_str().ok_or("no path")?;
        let path = template
            .split('/')
            .map(|part| {
                if part.starts_with('{') {
                    "example"
                } else {
                    part
                }
            })
            .collect::<Vec<_>>()
            .join("/");
        let called = answer(
            router.clone(),
            "tools/call",
            json!({
                "name":tool["name"], "arguments":{"method":method,"path":path}
            }),
        )
        .await?;
        assert_eq!(
            called["result"]["structuredContent"]["status"], 418,
            "{}: {called}",
            tool["name"]
        );
    }
    let unknown = answer(
        router,
        "tools/call",
        json!({
            "name":"unlisted-tool", "arguments":{"method":"GET","path":"/authority"}
        }),
    )
    .await?;
    assert_eq!(unknown["error"]["code"], -32602);
    assert_eq!(unknown["error"]["message"], "unknown tool unlisted-tool");
    Ok(())
}

#[tokio::test]
async fn a_generated_tool_cannot_route_to_another_method_or_path() -> Result<(), Box<dyn Error>> {
    let router = endpoint()?;
    let listed = answer(router.clone(), "tools/list", Value::Null).await?;
    let tool = listed["result"]["tools"]
        .as_array()
        .ok_or("no tools")?
        .iter()
        .find(|tool| tool["_meta"]["lys/path"] == "/identities/{id}/profile")
        .ok_or("no profile tool")?;
    for (method, path) in [
        ("POST", "/identities/example/transitions"),
        ("GET", "/identities/example/profile"),
        ("POST", "/identities/example/profile/extra"),
        ("POST", "/identities//profile"),
    ] {
        let refused = answer(
            router.clone(),
            "tools/call",
            json!({
                "name":tool["name"], "arguments":{"method":method,"path":path}
            }),
        )
        .await?;
        assert_eq!(refused["error"]["code"], -32602);
        assert!(
            refused["error"]["message"]
                .as_str()
                .ok_or("no refusal")?
                .contains(tool["name"].as_str().ok_or("no name")?)
        );
        assert!(refused.get("result").is_none());
    }
    let allowed = answer(router, "tools/call", json!({
        "name":tool["name"], "arguments":{"method":"POST","path":"/identities/example/profile?test=yes"}
    })).await?;
    assert_eq!(allowed["result"]["structuredContent"]["status"], 418);
    Ok(())
}

fn component_references(value: &mut Value) {
    match value {
        Value::Object(members) => {
            for (key, member) in members {
                if key == "$ref" {
                    if let Some(name) = member
                        .as_str()
                        .and_then(|target| target.strip_prefix("#/$defs/"))
                    {
                        *member = Value::String(format!("#/components/schemas/{name}"));
                    }
                } else {
                    component_references(member);
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                component_references(item);
            }
        }
        _ => {}
    }
}

#[tokio::test]
async fn generated_schema_definitions_keep_the_actual_route_types() -> Result<(), Box<dyn Error>> {
    let api = crate::openapi::api();
    let document = api.document().map_err(|faults| faults.join("; "))?;
    let listed = answer(endpoint()?, "tools/list", Value::Null).await?;
    let tools = listed["result"]["tools"].as_array().ok_or("no tools")?;
    for tool in tools {
        let method = tool["_meta"]["lys/method"].as_str().ok_or("no method")?;
        let path = tool["_meta"]["lys/path"].as_str().ok_or("no path")?;
        let contract = api
            .routes()
            .iter()
            .find(|route| route.method.word() == method && route.path == path)
            .ok_or("no contract")?;
        if let Some(request) = contract.request.as_deref() {
            let schema = if method == "get" {
                &tool["_meta"]["lys/querySchema"]
            } else {
                &tool["inputSchema"]
            };
            assert!(schema["$defs"].get(request).is_some());
        }
        if let Some(response) = contract.response.as_deref() {
            assert!(tool["outputSchema"]["$defs"].get(response).is_some());
        }
        for schema in [
            &tool["inputSchema"],
            &tool["outputSchema"],
            &tool["_meta"]["lys/querySchema"],
        ] {
            if let Some(definitions) = schema["$defs"].as_object() {
                for (name, definition) in definitions {
                    let mut original = definition.clone();
                    component_references(&mut original);
                    assert_eq!(
                        original, document["components"]["schemas"][name],
                        "{path}: {name}"
                    );
                }
            }
        }
    }
    Ok(())
}
