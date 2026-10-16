//! Validate and dispatch one MCP tool call through the admitted router.

use super::{Endpoint, ResultValue, TOOLS};
use axum::body::Body;
use axum::extract::{MatchedPath, OriginalUri, Request};
use axum::http::{Method, Uri, header};
use axum::response::IntoResponse;
use serde::Deserialize;
use serde_json::Value;
use std::sync::Arc;
use tower::ServiceExt;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Call {
    name: String,
    arguments: Arguments,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Arguments {
    #[serde(default = "read_method")]
    method: String,
    #[serde(default = "root_path")]
    path: String,
    body: Option<Value>,
}

struct GivenFields {
    empty: bool,
    path: bool,
    method: bool,
}

fn read_method() -> String {
    "GET".to_owned()
}
fn root_path() -> String {
    "/".to_owned()
}

pub(super) async fn call(
    endpoint: &Endpoint,
    parts: axum::http::request::Parts,
    params: &Value,
    (signer, app): crate::mcp_callers::Callers,
    message: &[u8],
) -> ResultValue {
    let (call, uri) = parsed(params)?;
    if call.name == "what-can-I-do" {
        if let Some(app) = app.as_ref().filter(|app| app.person_reads.is_some()) {
            return super::person::discovery(endpoint, app, message).await;
        }
        return super::discovery::answer(endpoint, &parts.headers, signer, app.as_ref()).await;
    }
    if let Some(app) = app.as_ref().filter(|app| app.person_reads.is_some()) {
        return super::person::call(
            endpoint,
            app,
            super::person::Reading {
                method: &call.arguments.method,
                uri,
                has_body: call.arguments.body.is_some(),
                message,
            },
        )
        .await;
    }
    dispatch(endpoint, parts, call, uri, (signer, app), message).await
}

fn parsed(params: &Value) -> Result<(Call, Uri), (i32, String)> {
    // `_meta` is the field MCP reserves on every request's params for the
    // client's own use (Claude Code sends one on each call); it asks nothing
    // of the tool, so it is set aside and every other field is still held to
    // the call's shape.
    let mut params = params.clone();
    if let Some(fields) = params.as_object_mut() {
        fields.remove("_meta");
    }
    let fields = params["arguments"]
        .as_object()
        .ok_or_else(|| (-32602, "the tool arguments must be an object".to_owned()))?;
    let given = GivenFields {
        empty: fields.is_empty(),
        path: fields.contains_key("path"),
        method: fields.contains_key("method"),
    };
    let call: Call = serde_json::from_value(params)
        .map_err(|error| (-32602, format!("invalid tool call: {error}")))?;
    if !TOOLS.iter().any(|(name, _)| *name == call.name) {
        return Err((-32602, format!("unknown tool {}", call.name)));
    }
    validate_arguments(&given, &call)?;
    let uri: Uri = call
        .arguments
        .path
        .parse()
        .map_err(|error| (-32602, format!("invalid route path: {error}")))?;
    if uri.scheme().is_some()
        || uri.authority().is_some()
        || !call.arguments.path.starts_with('/')
        || call.arguments.path.starts_with("//")
    {
        return Err((
            -32602,
            "the tool path must be local to the HTTP router".to_owned(),
        ));
    }
    if call.name == "drafts" && uri.path() != "/drafts" && !uri.path().starts_with("/drafts/") {
        return Err((
            -32602,
            "drafts requires a declared /drafts route".to_owned(),
        ));
    }
    Ok((call, uri))
}

fn validate_arguments(given: &GivenFields, call: &Call) -> Result<(), (i32, String)> {
    if call.name == "what-can-I-do" {
        return if given.empty {
            Ok(())
        } else {
            Err((
                -32602,
                "what-can-I-do requires empty arguments {}".to_owned(),
            ))
        };
    }
    if !given.path {
        return Err((
            -32602,
            format!("{} requires its local route path", call.name),
        ));
    }
    let methods: &[&str] = if call.name == "change" {
        if !given.method {
            return Err((-32602, "change requires an explicit method".to_owned()));
        }
        &["POST", "PUT", "PATCH", "DELETE"]
    } else {
        &["GET", "POST"]
    };
    if !methods.contains(&call.arguments.method.as_str()) {
        return Err((
            -32602,
            format!("{} requires {}", call.name, methods.join(", ")),
        ));
    }
    Ok(())
}

fn grant_principal(
    endpoint: &Endpoint,
    headers: &axum::http::HeaderMap,
    method: &str,
    path: &str,
) -> Result<crate::agent_signature::TokenPrincipal, crate::grant_tokens::TokenError> {
    let token = crate::grant_tokens::header(headers)?;
    let (resource, action) = crate::openapi_table::token_scope(method, path)?;
    let state = endpoint.state.as_ref().ok_or_else(|| {
        crate::grant_tokens::TokenError::Unavailable("MCP has no token authority state".to_owned())
    })?;
    crate::grant_tokens::principal(state, token, &resource, &action)
}

async fn dispatch(
    endpoint: &Endpoint,
    mut parts: axum::http::request::Parts,
    call: Call,
    uri: Uri,
    (signer, app): crate::mcp_callers::Callers,
    message: &[u8],
) -> ResultValue {
    let mut token = None;
    let method = call.arguments.method.clone();
    let route = uri.clone();
    let mut agent = signer.or_else(|| app.as_ref().map(|app| app.agent));
    if parts.headers.contains_key(crate::grant_tokens::HEADER) {
        let admitted = grant_principal(endpoint, &parts.headers, &method, route.path());
        match admitted {
            Ok(principal) => {
                token = Some(principal);
                agent = agent.or(Some(principal.holder));
                parts.extensions.insert(principal);
            }
            Err(error) => {
                return super::refusal::rendered(
                    endpoint,
                    &method,
                    &route,
                    agent,
                    error.into_response(),
                )
                .await;
            }
        }
    }
    parts.method = Method::from_bytes(call.arguments.method.as_bytes())
        .map_err(|error| (-32602, error.to_string()))?;
    parts.uri = uri;
    parts.extensions.remove::<MatchedPath>();
    parts.extensions.remove::<OriginalUri>();
    for name in [header::CONTENT_LENGTH, header::CONTENT_TYPE, header::ACCEPT] {
        parts.headers.remove(name);
    }
    parts.headers.insert(
        header::ACCEPT,
        axum::http::HeaderValue::from_static("application/json"),
    );
    let bytes = if let Some(body) = call.arguments.body {
        parts.headers.insert(
            header::CONTENT_TYPE,
            axum::http::HeaderValue::from_static("application/json"),
        );
        serde_json::to_vec(&body).map_err(|error| (-32603, error.to_string()))?
    } else {
        Vec::new()
    };
    let witness =
        crate::mcp_receipts::witness(endpoint.state.as_deref(), &parts, signer, app.as_ref())
            .map_err(|error| {
                (
                    -32603,
                    format!("the call's agent could not be named: {error}"),
                )
            })?;
    let relay_evidence = signer
        .and_then(|_| parts.headers.get(crate::agent_signature::HEADER))
        .and_then(|value| value.to_str().ok())
        .map(|header| (header.to_owned(), Arc::<[u8]>::from(message)));
    let relayed = signer
        .or_else(|| app.as_ref().map(|app| app.agent))
        .map(|agent| crate::agent_signature::Relay {
            agent,
            signed: relay_evidence,
        });
    let kept = kept_before(endpoint, &parts, witness, &bytes)?;
    let body = Body::from(bytes);
    if signer.is_some() {
        parts.headers.remove(crate::agent_signature::HEADER);
    }
    if app.is_some() {
        parts.headers.remove(header::AUTHORIZATION);
    }
    let relay = dispatch_router(endpoint.router.clone(), Request::from_parts(parts, body));
    let answered = match (relayed, token) {
        (Some(relayed), _) => crate::agent_signature::relayed(relayed, relay).await,
        (None, Some(principal)) => crate::agent_signature::token_scoped(principal, relay).await,
        (None, None) => relay.await,
    };
    let response = answered;
    let made = response.status().is_success();
    let mut result = super::refusal::rendered(endpoint, &method, &route, agent, response).await?;
    if let Some(kept) = kept {
        kept.answer(&endpoint.refused, made, &mut result)
            .map_err(|error| (-32603, format!("the kept leaf could not be named: {error}")))?;
    }
    Ok(result)
}

fn kept_before(
    endpoint: &Endpoint,
    parts: &axum::http::request::Parts,
    witness: Option<crate::mcp_receipts::Witness>,
    bytes: &[u8],
) -> Result<Option<crate::mcp_receipts::Kept>, (i32, String)> {
    let kept = match (endpoint.state.as_deref(), witness) {
        (Some(state), Some(witness)) if crate::mcp_receipts::changing(&parts.method) => Some(
            crate::mcp_receipts::Kept::before(
                state,
                &endpoint.refused,
                witness,
                &parts.method,
                &parts.uri,
                bytes,
            )
            .map_err(|error| {
                (
                    -32603,
                    format!(
                        "the change was not made: its receipt could not be kept first: {error}"
                    ),
                )
            })?,
        ),
        _ => None,
    };
    Ok(kept)
}

#[cfg(test)]
mod validation_tests {
    use super::parsed;
    use serde_json::json;

    #[test]
    fn a_read_without_its_required_path_is_refused_before_router_dispatch() {
        assert!(parsed(&json!({"name":"read","arguments":{}})).is_err());
    }

    #[test]
    fn a_change_without_its_required_method_is_refused_before_router_dispatch() {
        assert!(parsed(&json!({"name":"change","arguments":{"path":"/people"}})).is_err());
    }

    #[test]
    fn a_change_does_not_accept_a_read_method_outside_its_declared_schema() {
        assert!(
            parsed(&json!({"name":"change","arguments":{"method":"GET","path":"/people"}}))
                .is_err()
        );
    }

    #[test]
    fn the_drafts_tool_does_not_dispatch_a_route_outside_its_declared_drafts_path() {
        assert!(parsed(&json!({"name":"drafts","arguments":{"path":"/people"}})).is_err());
    }

    #[test]
    fn discovery_accepts_only_the_empty_arguments_it_declares() {
        assert!(parsed(&json!({"name":"what-can-I-do","arguments":{"path":"/people"}})).is_err());
        assert!(parsed(&json!({"name":"what-can-I-do","arguments":{}})).is_ok());
    }
}

pub(crate) async fn dispatch_router(
    router: axum::Router,
    request: Request,
) -> axum::response::Response {
    match router.oneshot(request).await {
        Ok(response) => response,
        Err(error) => match error {},
    }
}
