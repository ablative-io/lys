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
    method: String,
    path: String,
    body: Option<Value>,
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
        return super::discovery::answer(endpoint, &parts.headers, signer, app.as_ref()).await;
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
    let call: Call = serde_json::from_value(params)
        .map_err(|error| (-32602, format!("invalid tool call: {error}")))?;
    if !["GET", "POST", "PUT", "PATCH", "DELETE"].contains(&call.arguments.method.as_str()) {
        return Err((
            -32602,
            "the tool call requires GET, POST, PUT, PATCH or DELETE".to_owned(),
        ));
    }
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
    if !TOOLS.iter().any(|(name, _)| *name == call.name) {
        return Err((-32602, format!("unknown tool {}", call.name)));
    }
    Ok((call, uri))
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
    if parts.headers.contains_key(crate::grant_tokens::HEADER) {
        let admitted = (|| {
            let token = crate::grant_tokens::header(&parts.headers)?;
            let (resource, action) =
                crate::openapi_table::token_scope(&call.arguments.method, uri.path())?;
            let state = endpoint.state.as_ref().ok_or_else(|| {
                crate::grant_tokens::TokenError::Unavailable(
                    "MCP has no token authority state".to_owned(),
                )
            })?;
            crate::grant_tokens::principal(state, token, &resource, &action)
        })();
        match admitted {
            Ok(principal) => {
                token = Some(principal);
                parts.extensions.insert(principal);
            }
            Err(error) => return crate::mcp_callers::rendered(error.into_response()).await,
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
        .or_else(|| app.as_ref().map(|(agent, _)| *agent))
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
    let relay = endpoint
        .router
        .clone()
        .oneshot(Request::from_parts(parts, body));
    let answered = match (relayed, token) {
        (Some(relayed), _) => crate::agent_signature::relayed(relayed, relay).await,
        (None, Some(principal)) => crate::agent_signature::token_scoped(principal, relay).await,
        (None, None) => relay.await,
    };
    let response = match answered {
        Ok(response) => response,
        Err(error) => match error {},
    };
    let made = response.status().is_success();
    let mut result = crate::mcp_callers::rendered(response).await?;
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
